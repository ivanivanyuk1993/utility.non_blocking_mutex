use crate::mutex_guard::MutexGuard;

/// Implementing custom [FnOnce] is unstable,
/// so we need [NonBlockingMutexTask]
pub trait NonBlockingMutexTask<State: ?Sized>: Send + Sized {
    /// We don't need to specify `'unsafe_state_ref` here,
    /// because all references held by [NonBlockingMutexTask] (including those in [State])
    /// must outlive [crate::non_blocking_mutex::NonBlockingMutex]
    /// (because [NonBlockingMutexTask] is stored in [crate::non_blocking_mutex::NonBlockingMutex],
    /// and Rust compiler enforces that all references held by type should be valid(outlive) while type
    /// lives),
    /// and reference to [crate::non_blocking_mutex::NonBlockingMutex] must live shorter than
    /// [crate::non_blocking_mutex::NonBlockingMutex]
    /// (because Rust compiler enforces that type should be valid(outlive) while type is referenced),
    /// so [State] will always outlive reference to
    /// [crate::non_blocking_mutex::NonBlockingMutex].`unsafe_state` in
    /// [crate::non_blocking_mutex::NonBlockingMutex::run_if_first_or_schedule_on_first]
    fn run_with_state(self, state: MutexGuard<State>);
}

impl<State: ?Sized, TFnOnce: FnOnce(MutexGuard<State>) + Send + Sized> NonBlockingMutexTask<State>
    for TFnOnce
{
    fn run_with_state(self, state: MutexGuard<State>) {
        self(state);
    }
}
