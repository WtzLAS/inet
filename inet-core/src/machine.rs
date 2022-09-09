use crossbeam_queue::SegQueue;

use crate::model::Agent;

pub struct Machine {
    eqs: SegQueue<(*mut Agent, *mut Agent)>
}