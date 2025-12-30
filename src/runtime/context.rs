use std::cell::RefCell;
use std::sync::Arc;
use super::State;

thread_local! {
    static CONTEXT: RefCell<Option<Arc<State>>> = RefCell::new(None)
}

pub(crate) struct EntryGuard {
    old_context: Option<Arc<State>>
}

// This will allow nested Runtimes handling
impl Drop for EntryGuard {
    fn drop(&mut self) {
        CONTEXT.with(|ctx| {
            let old_context = self.old_context.take();
            *ctx.borrow_mut() = old_context
        });
    }
}

pub(crate) fn enter(state: Arc<State>) -> EntryGuard {
    CONTEXT.with(|ctx| {
        let old_context = ctx.borrow_mut().replace(state);
        EntryGuard { old_context }
    })
}

pub(crate) fn state() -> Arc<State> {
    CONTEXT.with(|ctx| {
        ctx.borrow().clone().expect("Not in runtime")
    })
}