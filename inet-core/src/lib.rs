use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    mem::swap,
    ptr::null_mut,
    sync::atomic::{AtomicPtr, Ordering},
};

use crossbeam_queue::SegQueue;

pub type BoxRuleFn = Box<dyn Fn(Context) + Send + Sync>;

#[derive(Debug)]
pub enum Agent {
    Name { port: AtomicPtr<Agent> },
    Normal { id: usize, ports: Vec<*mut Agent> },
}

impl Agent {
    pub fn ports(&self) -> &Vec<*mut Agent> {
        match self {
            Agent::Name { port: _ } => {
                panic!("try to get ports on a name agent");
            }
            Agent::Normal { id: _, ports } => ports,
        }
    }

    pub fn ports_mut(&mut self) -> &mut Vec<*mut Agent> {
        match self {
            Agent::Name { port: _ } => {
                panic!("try to get mutable ports on a name agent");
            }
            Agent::Normal { id: _, ports } => ports,
        }
    }

    pub fn drop_recursive(self: Box<Self>) {
        let mut set = HashSet::new();
        set.insert(&*self as *const Agent as *mut Agent);
        self.drop_recursive_impl(&mut set);
    }

    fn drop_recursive_impl(self, set: &mut HashSet<*mut Agent>) {
        match self {
            Agent::Name { mut port } => {
                let ptr = port.get_mut();
                if !ptr.is_null() && !set.contains(ptr) {
                    set.insert(*ptr);
                    let agent = unsafe { Box::from_raw(*ptr) };
                    agent.drop_recursive_impl(set);
                }
            }
            Agent::Normal { id: _, ports } => {
                for port in ports.into_iter() {
                    if set.contains(&port) {
                        continue;
                    }
                    set.insert(port);
                    let agent = unsafe { Box::from_raw(port) };
                    agent.drop_recursive_impl(set);
                }
            }
        }
    }
}

pub struct Machine {
    eqs: SegQueue<(*mut Agent, *mut Agent)>,
    rules: HashMap<(usize, usize), BoxRuleFn>,
}

impl Machine {
    pub fn new() -> Self {
        Self {
            eqs: SegQueue::new(),
            rules: HashMap::new(),
        }
    }

    pub fn new_name(&self) -> *mut Agent {
        Box::into_raw(Box::new(Agent::Name {
            port: AtomicPtr::new(null_mut()),
        }))
    }

    pub fn new_agent(id: usize, pri_port: *mut Agent, aux_ports: &[*mut Agent]) -> *mut Agent {
        let mut ports = vec![pri_port];
        ports.extend_from_slice(aux_ports);
        let agent = Box::new(Agent::Normal { id, ports });
        Box::into_raw(agent)
    }

    pub fn new_eq(&self, lhs: *mut Agent, rhs: *mut Agent) {
        self.eqs.push((lhs, rhs));
    }

    pub fn new_agent_and_eq(
        &self,
        id: usize,
        pri_port: *mut Agent,
        aux_ports: &[*mut Agent],
    ) -> *mut Agent {
        let mut ports = vec![pri_port];
        ports.extend_from_slice(aux_ports);
        let agent = Box::new(Agent::Normal { id, ports });
        let result = Box::into_raw(agent);
        self.eqs.push((result, pri_port));
        result
    }

    pub fn new_rule(&mut self, id_lhs: usize, id_rhs: usize, rule: BoxRuleFn) {
        self.rules.insert((id_lhs, id_rhs), rule);
    }

    pub fn run(&self) -> (usize, usize) {
        let mut op_interact = 0;
        let mut op_name = 0;
        while let Some((ptr_lhs, ptr_rhs)) = self.eqs.pop() {
            let lhs = unsafe { Box::from_raw(ptr_lhs) };
            let rhs = unsafe { Box::from_raw(ptr_rhs) };
            // println!("\n[+] {:?} >< {:?}", lhs, rhs);
            // println!(" |  {:?} >< {:?}", ptr_lhs, ptr_rhs);
            match (&*lhs, &*rhs) {
                (_, Agent::Name { port }) => {
                    match port.compare_exchange(
                        std::ptr::null_mut(),
                        ptr_lhs,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            // println!(" |  Var1: {:?} now points at {:?}", ptr_rhs, ptr_lhs);
                            Box::leak(lhs);
                            Box::leak(rhs);
                        },
                        Err(target) => {
                            // println!(" |  Ind1: {:?} <=> {:?}", ptr_lhs, target);
                            self.eqs.push((Box::into_raw(lhs), target));
                        },
                    }
                    op_name += 1;
                }
                (Agent::Name { port }, _) => {
                    match port.compare_exchange(
                        std::ptr::null_mut(),
                        ptr_rhs,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            // println!(" |  Var2: {:?} now points at {:?}", ptr_lhs, ptr_rhs);
                            Box::leak(lhs);
                            Box::leak(rhs);
                        },
                        Err(target) => {
                            // println!(" |  Ind2: {:?} <=> {:?}", target, ptr_rhs);
                            self.eqs.push((target, Box::into_raw(rhs)))
                        },
                    }
                    op_name += 1;
                }
                (
                    Agent::Normal {
                        id: id_lhs,
                        ports: _,
                    },
                    Agent::Normal {
                        id: id_rhs,
                        ports: _,
                    },
                ) => {
                    let rule = self
                        .rules
                        .get(&(*id_lhs, *id_rhs))
                        .expect("no rule is applicable");

                    rule(Context {
                        machine: self,
                        ptr_lhs: Box::into_raw(lhs),
                        ptr_rhs: Box::into_raw(rhs),
                    });

                    op_interact += 1;
                }
            }
        }
        (op_interact, op_name)
    }

    pub fn par_run() {

    }
}

pub struct Context<'a> {
    pub machine: &'a Machine,
    pub ptr_lhs: *mut Agent,
    pub ptr_rhs: *mut Agent,
}

impl<'a> Context<'a> {
    pub fn reverse(&mut self) {
        swap(&mut self.ptr_lhs, &mut self.ptr_rhs);
    }

    pub fn lhs(&self) -> Box<Agent> {
        unsafe { Box::from_raw(self.ptr_lhs) }
    }

    pub fn rhs(&self) -> Box<Agent> {
        unsafe { Box::from_raw(self.ptr_rhs) }
    }
}
