use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use crate::runtime::{IoState, State};

struct TimerFuture {
    pub deadline: std::time::Instant,
    pub slab_key: RefCell<Option<usize>>,
    pub io_state: Arc<IoState>,
    pub state: Arc<State>,
}

impl TimerFuture {
    pub fn new(duration: std::time::Duration, io_state: Arc<IoState>, state: Arc<State>) -> Self {
        Self {
            io_state,
            deadline: std::time::Instant::now() + duration,
            state,
            slab_key: RefCell::new(None),
        }
    }
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline {
            println!("[Future] TimerFuture completed");
            return Poll::Ready(());
        }

        println!("[Future] TimerFuture pending, registering waker");
        // For demonstration, we just return Pending
        self.io_state.read_waker.register(cx.waker());

        if let Some(slab_id) = *self.slab_key.borrow() {
            println!(
                "[Future] IoState already registered in slab with id {}",
                slab_id
            );
        } else {
            println!("[Future] IoState not registered in slab, registering now");

            let slab_id = {
                let mut slab = self.state.slabs.write().unwrap();
                slab.insert(self.io_state.clone())
            };
            let mut slab_key = self.slab_key.borrow_mut();
            *slab_key = Some(slab_id);
            println!("[Future] Registered IoState in slab with id {}", slab_id);
        }
        Poll::Pending
    }
}
