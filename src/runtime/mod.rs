pub mod executor;
pub mod reactor;
pub mod state;
pub mod context;
pub(crate) mod metrics;
pub(crate) mod trace;

pub use state::*;

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::thread;

use crate::runtime::executor::Task;
use crate::runtime::trace::*;
use crate::runtime::metrics::emit as metrics_emit;

thread_local! {
    static STATE: RefCell<i32> = RefCell::new(0);
}

pub struct Runtime {
    pub(crate) state: Arc<State>
}

impl Runtime {
    pub fn new() -> std::io::Result<Self> {
        info!("Initializing runtime");
        let state = Arc::new(State::new()?);
        Ok(Runtime {
            state
        })
    }

    pub fn block_on<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + Send + 'static
    {
        info!("Starting runtime with block_on");
        let _guard = context::enter(self.state.clone());

        let reactor_state = self.state.clone();
        thread::spawn(move || {
            let _guard = context::enter(reactor_state);
            reactor::run_reactor();
        });

        // Create main task with completion flag
        let main_task_complete = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let complete_flag = main_task_complete.clone();
        
        let wrapped_future = async move {
            future.await;
            complete_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        };
        
        let task = Arc::new(Task {
            future: Arc::new(Mutex::new(Box::pin(wrapped_future)))
        });
        
        metrics_emit::task_spawned();
        debug!("Spawning initial task");
        
        self.state.task_queue.lock().unwrap().push(task);

        // Run executor until main task completes
        executor::run_executor_until(main_task_complete);
    }
}