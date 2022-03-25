use std::{
    collections::HashMap,
    fmt::Debug,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use crossbeam_queue::SegQueue;
use easy_parallel::Parallel;
use sharded_slab::{pool::Ref, Clear, Pool};
use smallvec::SmallVec;

#[derive(Debug)]
pub enum Agent {
    Tag(AtomicBool, AtomicUsize),
    Custom(usize, SmallVec<[usize; 16]>),
}

impl Clear for Agent {
    fn clear(&mut self) {
        match self {
            Agent::Tag(is_ind, target) => {
                *is_ind.get_mut() = false;
                *target.get_mut() = 0;
            }
            Agent::Custom(type_id, ports) => {
                *type_id = 0;
                ports.clear();
            }
        }
    }
}

impl Default for Agent {
    fn default() -> Self {
        Agent::Tag(AtomicBool::new(false), AtomicUsize::new(0))
    }
}

type BoxedRuleFn = Box<dyn Fn(Context) + Send + Sync>;

pub struct MachineBuilder {
    next_type_id: usize,
    rules: HashMap<(usize, usize), BoxedRuleFn>,
    agents: Pool<Agent>,
    eqs: Vec<(usize, usize)>,
}

impl MachineBuilder {
    pub fn new() -> MachineBuilder {
        MachineBuilder {
            next_type_id: 0,
            rules: HashMap::new(),
            agents: Pool::default(),
            eqs: vec![],
        }
    }

    #[inline(always)]
    pub fn new_rule(&mut self, lhs_type: usize, rhs_type: usize, rule_fn: BoxedRuleFn) {
        self.rules.insert((lhs_type, rhs_type), rule_fn);
    }

    #[inline(always)]
    pub fn new_type(&mut self) -> usize {
        let type_id = self.next_type_id;
        self.next_type_id += 1;
        type_id
    }

    #[inline(always)]
    pub fn new_tag(&self) -> usize {
        self.agents
            .create_with(|v| {
                *v = Agent::default();
            })
            .expect("allocation failed")
    }

    #[inline(always)]
    pub fn new_agent(
        &mut self,
        type_id: usize,
        principal_port: usize,
        aux_ports: &[usize],
    ) -> usize {
        if type_id >= self.next_type_id {
            panic!(
                "invalid type id, cap: {}, actual: {}",
                self.next_type_id - 1,
                type_id
            );
        }
        let id = self
            .agents
            .create_with(|v| match v {
                Agent::Tag(_, _) => {
                    *v = Agent::Custom(type_id, SmallVec::from_slice(aux_ports));
                }
                Agent::Custom(old_type_id, old_ports) => {
                    *old_type_id = type_id;
                    old_ports.extend_from_slice(aux_ports);
                }
            })
            .expect("allocation failed");
        self.eqs.push((id, principal_port));
        id
    }

    pub fn into_machine(self) -> Machine {
        let eqs = SegQueue::new();
        for v in self.eqs {
            eqs.push(v);
        }
        Machine {
            max_type_id: self.next_type_id - 1,
            rules: self.rules,
            agents: self.agents,
            eqs,
        }
    }
}

impl Default for MachineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Context<'a> {
    pub machine: &'a Machine,
    pub lhs_id: usize,
    pub rhs_id: usize,
    pub lhs_ports: &'a [usize],
    pub rhs_ports: &'a [usize],
}

impl Context<'_> {
    pub fn remove_old_agents(&self) {
        self.machine.remove_agent(self.lhs_id);
        self.machine.remove_agent(self.rhs_id);
    }
}

pub struct Machine {
    max_type_id: usize,
    rules: HashMap<(usize, usize), BoxedRuleFn>,
    agents: Pool<Agent>,
    eqs: SegQueue<(usize, usize)>,
}

impl Machine {
    #[inline(always)]
    pub fn new_tag(&self) -> usize {
        self.agents
            .create_with(|v| {
                *v = Agent::default();
            })
            .expect("allocation failed")
    }

    #[inline(always)]
    pub fn new_agent(&self, type_id: usize, principal_port: usize, aux_ports: &[usize]) -> usize {
        if type_id >= self.max_type_id {
            panic!(
                "invalid type id, cap: {}, actual: {}",
                self.max_type_id, type_id
            );
        }
        let id = self
            .agents
            .create_with(|v| match v {
                Agent::Tag(_, _) => {
                    *v = Agent::Custom(type_id, SmallVec::from_slice(aux_ports));
                }
                Agent::Custom(old_type_id, old_ports) => {
                    *old_type_id = type_id;
                    old_ports.extend_from_slice(aux_ports);
                }
            })
            .expect("allocation failed");
        self.eqs.push((id, principal_port));
        id
    }

    #[inline(always)]
    pub fn get_agent(&self, id: usize) -> Option<Ref<Agent>> {
        self.agents.get(id)
    }

    #[inline(always)]
    pub fn remove_agent(&self, id: usize) {
        self.agents.clear(id);
    }

    #[inline(always)]
    pub fn new_eq(&self, lhs_id: usize, rhs_id: usize) {
        self.eqs.push((lhs_id, rhs_id));
    }

    pub fn eval(&self) -> (usize, usize) {
        let interactions = AtomicUsize::new(0);
        let name_op = AtomicUsize::new(0);
        Parallel::new()
            .each(0..2, |_| {
                while let Some((lhs_id, rhs_id)) = self.eqs.pop() {
                    let lhs_agent = self
                        .get_agent(lhs_id)
                        .expect("agent is cleared unexpectedly");
                    let rhs_agent = self
                        .get_agent(rhs_id)
                        .expect("agent is cleared unexpectedly");
                    match (&*lhs_agent, &*rhs_agent) {
                        (_, Agent::Tag(is_ind, target)) => {
                            loop {
                                if is_ind.load(Ordering::Acquire) {
                                    let target_value = target.load(Ordering::Acquire);
                                    self.remove_agent(rhs_id);
                                    self.new_eq(lhs_id, target_value);
                                    break;
                                } else if is_ind
                                    .compare_exchange(
                                        false,
                                        true,
                                        Ordering::AcqRel,
                                        Ordering::Relaxed,
                                    )
                                    .is_ok()
                                {
                                    target.store(lhs_id, Ordering::Release);
                                    break;
                                }
                            }
                            name_op.fetch_add(1, Ordering::Relaxed);
                        }
                        (Agent::Tag(is_ind, target), _) => {
                            loop {
                                if is_ind.load(Ordering::Acquire) {
                                    let target_value = target.load(Ordering::Acquire);
                                    self.remove_agent(lhs_id);
                                    self.new_eq(target_value, rhs_id);
                                    break;
                                } else if is_ind
                                    .compare_exchange(
                                        false,
                                        true,
                                        Ordering::AcqRel,
                                        Ordering::Relaxed,
                                    )
                                    .is_ok()
                                {
                                    target.store(rhs_id, Ordering::Release);
                                    break;
                                }
                            }
                            name_op.fetch_add(1, Ordering::Relaxed);
                        }
                        (
                            Agent::Custom(lhs_type_id, lhs_ports),
                            Agent::Custom(rhs_type_id, rhs_ports),
                        ) => {
                            let rule = self.rules.get(&(*lhs_type_id, *rhs_type_id));
                            match rule {
                                Some(rule) => {
                                    rule(Context {
                                        machine: self,
                                        lhs_id,
                                        rhs_id,
                                        lhs_ports,
                                        rhs_ports,
                                    });
                                }
                                None => {
                                    let reverse_rule = self
                                        .rules
                                        .get(&(*rhs_type_id, *lhs_type_id))
                                        .expect("no rules available");
                                    reverse_rule(Context {
                                        machine: self,
                                        rhs_id,
                                        lhs_id,
                                        rhs_ports,
                                        lhs_ports,
                                    });
                                }
                            }
                            interactions.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            })
            .run();
        (
            interactions.load(Ordering::Relaxed),
            name_op.load(Ordering::Relaxed),
        )
    }
}
