use std::collections::VecDeque;

use scc::HashMap;
use uuid::Uuid;

pub trait Agent: Sync {
    fn type_id(&self) -> Uuid;
    fn type_arity(&self) -> usize;
    fn id(&self) -> Uuid;
    fn ports(&self) -> Option<&[Uuid]>;
    fn ports_mut(&mut self) -> Option<&mut [Uuid]>;
}

pub trait Interact<T: Agent>: Agent {
    fn interact(ctx: &mut Machine, lhs: Uuid, rhs: Uuid);
}

impl<L, R> Interact<L> for R
where
    L: Interact<R>,
    R: Agent,
{
    fn interact(ctx: &mut Machine, lhs: Uuid, rhs: Uuid) {
        L::interact(ctx, rhs, lhs)
    }
}

pub struct Name(Uuid);

impl Agent for Name {
    fn type_id(&self) -> Uuid {
        Uuid::from_bytes([174, 217, 145, 87, 176, 230, 64, 255, 161, 131, 91, 88, 20, 127, 68, 40])
    }

    fn type_arity(&self) -> usize {
        0
    }

    fn id(&self) -> Uuid {
        self.0
    }

    fn ports(&self) -> Option<&[Uuid]> {
        None
    }

    fn ports_mut(&mut self) -> Option<&mut [Uuid]> {
        None
    }
}

pub struct Indirection {
    id: Uuid,
    target: [Uuid; 1],
}

impl Agent for Indirection {
    fn type_id(&self) -> Uuid {
        Uuid::from_bytes([130, 16, 66, 172, 163, 246, 74, 238, 170, 116, 43, 215, 22, 163, 85, 154])
    }

    fn type_arity(&self) -> usize {
        1
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn ports(&self) -> Option<&[Uuid]> {
        Some(&self.target)
    }

    fn ports_mut(&mut self) -> Option<&mut [Uuid]> {
        Some(&mut self.target)
    }
}

pub struct Machine {
    agents: HashMap<Uuid, Box<dyn Agent>>,
    eqs: VecDeque<(Uuid, Uuid)>,
}

impl Machine {
    fn eval(&mut self) {
        
    }
}
