use std::{
    collections::HashMap,
    fmt::Debug,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use crossbeam_queue::SegQueue;
use easy_parallel::Parallel;
use sharded_slab::{pool::Ref, Clear, Pool};
use smallvec::SmallVec;
use snafu::{OptionExt, Snafu};

#[derive(Debug)]
pub enum Agent {
    Tag(AtomicBool, AtomicUsize),
    Custom(usize, SmallVec<[usize; 16]>),
}

impl Clear for Agent {
    fn clear(&mut self) {
        match self {
            Agent::Tag(is_ind, target) => {
                *is_ind = AtomicBool::new(false);
                *target = AtomicUsize::new(0);
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

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("failed to allocate memory for new agents"))]
    AllocationFailed,
    #[snafu(display("provided invalid type id: actual={actual} cap={cap}"))]
    InvalidTypeId { actual: usize, cap: usize },
    #[snafu(display("agent {id} is accidentally cleared"))]
    MissingAgent { id: usize },
    #[snafu(display("no rule for {lhs_type_id} >< {rhs_type_id}"))]
    NoRule {
        lhs_type_id: usize,
        rhs_type_id: usize,
    },
}

type Result<T> = std::result::Result<T, Error>;

type BoxedRuleFn = Box<dyn Fn(&Machine, &[usize], &[usize]) + Send + Sync>;

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

    pub fn new_rule(&mut self, lhs_type: usize, rhs_type: usize, rule_fn: BoxedRuleFn) {
        self.rules.insert((lhs_type, rhs_type), rule_fn);
    }

    pub fn new_type(&mut self) -> usize {
        let type_id = self.next_type_id;
        self.next_type_id += 1;
        type_id
    }

    pub fn new_tag(&self) -> Result<usize> {
        self.agents
            .create_with(|v| {
                *v = Agent::default();
            })
            .context(AllocationFailedSnafu)
    }

    pub fn new_agent(
        &mut self,
        type_id: usize,
        principal_port: usize,
        aux_ports: &[usize],
    ) -> Result<usize> {
        if type_id >= self.next_type_id {
            return Err(Error::InvalidTypeId {
                actual: type_id,
                cap: self.next_type_id - 1,
            });
        }
        let id = self
            .agents
            .create_with(|v| {
                *v = Agent::Custom(type_id, SmallVec::from_slice(aux_ports));
            })
            .context(AllocationFailedSnafu)?;
        self.eqs.push((id, principal_port));
        Ok(id)
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

pub struct Machine {
    max_type_id: usize,
    rules: HashMap<(usize, usize), BoxedRuleFn>,
    agents: Pool<Agent>,
    eqs: SegQueue<(usize, usize)>,
}

impl Machine {
    pub fn new_tag(&self) -> Result<usize> {
        self.agents
            .create_with(|v| {
                *v = Agent::default();
            })
            .context(AllocationFailedSnafu)
    }

    pub fn new_agent(
        &self,
        type_id: usize,
        principal_port: usize,
        aux_ports: &[usize],
    ) -> Result<usize> {
        if type_id > self.max_type_id {
            return Err(Error::InvalidTypeId {
                actual: type_id,
                cap: self.max_type_id,
            });
        }
        let id = self
            .agents
            .create_with(|v| {
                *v = Agent::Custom(type_id, SmallVec::from_slice(aux_ports));
            })
            .context(AllocationFailedSnafu)?;
        self.eqs.push((id, principal_port));
        Ok(id)
    }

    pub fn get_agent(&self, id: usize) -> Option<Ref<Agent>> {
        self.agents.get(id)
    }

    pub fn remove_agent(&self, id: usize) {
        self.agents.clear(id);
    }

    pub fn new_eq(&self, lhs_id: usize, rhs_id: usize) {
        self.eqs.push((lhs_id, rhs_id));
    }

    pub fn eval(&self) -> Result<(usize, usize)> {
        let interactions = AtomicUsize::new(0);
        let name_op = AtomicUsize::new(0);
        Parallel::new()
            .each(0..2, |_| -> Result<()> {
                while let Some((lhs_id, rhs_id)) = self.eqs.pop() {
                    let lhs_agent = self
                        .get_agent(lhs_id)
                        .context(MissingAgentSnafu { id: lhs_id })?;
                    let rhs_agent = self
                        .get_agent(rhs_id)
                        .context(MissingAgentSnafu { id: rhs_id })?;
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
                                if is_ind.load(Ordering::SeqCst) {
                                    let target_value = target.load(Ordering::SeqCst);
                                    self.remove_agent(lhs_id);
                                    self.new_eq(target_value, rhs_id);
                                    break;
                                } else if is_ind
                                    .compare_exchange(
                                        false,
                                        true,
                                        Ordering::SeqCst,
                                        Ordering::SeqCst,
                                    )
                                    .is_ok()
                                {
                                    target.store(rhs_id, Ordering::SeqCst);
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
                                    rule(self, lhs_ports, rhs_ports);
                                }
                                None => {
                                    let reverse_rule = self
                                        .rules
                                        .get(&(*rhs_type_id, *lhs_type_id))
                                        .context(NoRuleSnafu {
                                            lhs_type_id: *lhs_type_id,
                                            rhs_type_id: *rhs_type_id,
                                        })?;
                                    reverse_rule(self, rhs_ports, lhs_ports);
                                }
                            }
                            self.remove_agent(lhs_id);
                            self.remove_agent(rhs_id);
                            interactions.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                Ok(())
            })
            .run();
        Ok((
            interactions.load(Ordering::Relaxed),
            name_op.load(Ordering::Relaxed),
        ))
    }
}
