pub mod executor;
pub mod reactor;
pub mod state;
pub mod context;

pub use state::*;

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::thread;

use crate::runtime::executor::Task;

thread_local! {
    static STATE: RefCell<i32> = RefCell::new(0);
}

pub struct Runtime {
    state: Arc<State>
}

impl Runtime {
    pub fn new() -> std::io::Result<Self> {
        let state = Arc::new(State::new()?);
        Ok(Runtime {
            state
        })
    }

    pub fn block_on<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + Send + 'static
    {
        let _guard = context::enter(self.state.clone());

        let reactor_state = self.state.clone();
        thread::spawn(move || {
            let _guard = context::enter(reactor_state);
            reactor::run_reactor();
        });

        let task = Arc::new(Task {
            future: Arc::new(Mutex::new(Box::pin(future)))
        });
        self.state.task_queue.lock().unwrap().push(task);

        executor::run_executor();
    }
}