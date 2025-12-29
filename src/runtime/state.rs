use futures::task::AtomicWaker;
use slab::Slab;
use std::sync::{Arc, Mutex, RwLock};
use mio::{Poll, Registry};

use super::executor::Task;

pub struct IoState {
    pub read_waker: AtomicWaker,
    pub write_waker: AtomicWaker,
}

impl IoState {
    pub fn new() -> Self {
        Self {
            read_waker: AtomicWaker::new(),
            write_waker: AtomicWaker::new()
        }
    }
}

pub struct State {
    pub task_queue: Mutex<Vec<Arc<Task>>>,
    pub slabs: RwLock<Slab<Arc<IoState>>>,
    pub poll: Mutex<Poll>,
    pub poll_registry: Registry
}

impl State {
    pub fn new() -> std::io::Result<Self> {
        let poll = Poll::new()?;
        let registry = poll.registry().try_clone()?;
        Ok(State {
            task_queue: Mutex::new(vec!()),
            slabs: RwLock::new(Slab::new()),
            poll: Mutex::new(poll),
            poll_registry: registry
        })
    }
}