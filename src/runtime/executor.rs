use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Wake, Waker};

use crate::runtime::context;
use crate::runtime::trace::*;
use crate::runtime::metrics::emit as metrics_emit;

pub struct Task {
    pub future: Arc<Mutex<Pin<Box<dyn futures::Future<Output = ()> + Send + 'static>>>>
}

/// Spawn a new task onto the runtime
///
/// This function spawns a new task that will be executed by the runtime's executor.
/// The task must return `()` and be `Send + 'static`.
pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static
{
    trace!("Spawning new task");
    let state = context::state();

    let task = Arc::new(Task {
        future: Arc::new(Mutex::new(Box::pin(future)))
    });

    metrics_emit::task_spawned();
    state.task_queue.lock().unwrap().push(task);
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
                    // Task will be re-added to queue when waker is called
                }
            }
        } else {
            // Queue is empty, yield to avoid busy-loop
            trace!("Executor queue empty, yielding");
            std::thread::yield_now();
        }
    }
}

/// Run executor until the completion flag is set
pub(crate) fn run_executor_until(complete: Arc<std::sync::atomic::AtomicBool>) {
    info!("Executor started (with completion check)");
    let executor_state = context::state();
    
    loop {
        // Check if main task completed
        if complete.load(std::sync::atomic::Ordering::SeqCst) {
            info!("Main task completed, executor exiting");
            break;
        }
        
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
                    // Task will be re-added to queue when waker is called
                }
            }
        } else {
            // Queue is empty, yield to avoid busy-loop
            trace!("Executor queue empty, yielding");
            std::thread::yield_now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    #[test]
    fn test_task_creation() {
        let future = Box::pin(async {});
        let task = Task {
            future: Arc::new(Mutex::new(future))
        };
        
        // Task should be created successfully
        assert!(Arc::strong_count(&task.future) == 1);
    }
    
    #[test]
    fn test_wake_implementation() {
        let future = Box::pin(async {});
        let task = Arc::new(Task {
            future: Arc::new(Mutex::new(future))
        });
        
        // Waker should be created without panicking
        let waker = Waker::from(task.clone());
        waker.wake();
        // If we get here, Wake trait works
    }
}
