use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Wake, Waker};

use crate::runtime::context;
use crate::runtime::trace::*;
use crate::runtime::metrics::emit as metrics_emit;

pub struct Task {
    pub future: Arc<Mutex<Pin<Box<dyn futures::Future<Output = ()> + Send + 'static>>>>
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        trace!("Waking task and adding to queue");
        let state = context::state();
        let mut queue = state.task_queue.lock().unwrap();
        queue.push(self.clone());
    }
}

pub fn run_executor() {
    info!("Executor started");
    let executor_state = context::state();
    
    loop {
        let task = {
            let mut queue = executor_state.task_queue.lock().unwrap();
            queue.pop()
        };
        
        if let Some(task) = task {
            debug!("Polling task");
            
            let mut future = task.future.lock().unwrap();
            let waker = Waker::from(task.clone());
            let mut context = Context::from_waker(&waker);

            match future.as_mut().poll(&mut context) {
                std::task::Poll::Ready(_) => {
                    debug!("Task completed");
                    metrics_emit::task_completed();
                }
                std::task::Poll::Pending => {
                    trace!("Task returned Pending");
                }
            }
        }
    }
}
