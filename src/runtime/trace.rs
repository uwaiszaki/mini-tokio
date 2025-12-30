//! Internal tracing utilities

#[cfg(feature = "tracing")]
pub(crate) use tracing::{debug, error, info, trace, warn};

#[cfg(not(feature = "tracing"))]
mod noop {
    /// No-op trace macro when tracing is disabled
    macro_rules! trace {
        ($($tt:tt)*) => {};
    }

    /// No-op debug macro when tracing is disabled
    macro_rules! debug {
        ($($tt:tt)*) => {};
    }

    /// No-op info macro when tracing is disabled
    macro_rules! info {
        ($($tt:tt)*) => {};
    }

    /// No-op warn macro when tracing is disabled
    macro_rules! log_warn {
        ($($tt:tt)*) => {};
    }

    /// No-op error macro when tracing is disabled
    macro_rules! error {
        ($($tt:tt)*) => {};
    }

    pub(crate) use {debug, error, info, trace};
    pub(crate) use log_warn as warn;
}

#[cfg(not(feature = "tracing"))]
pub(crate) use noop::*;

