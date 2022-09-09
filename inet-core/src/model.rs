use std::sync::atomic::AtomicPtr;

pub enum Agent {
    Name(AtomicPtr<Agent>),
    Custom(Vec<*mut Agent>),
}