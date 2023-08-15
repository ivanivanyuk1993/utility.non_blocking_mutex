use crate::mutex_guard::MutexGuard;

/// Implementing custom [FnOnce] is unstable,
/// so we need [NonBlockingMutexTask]
pub trait NonBlockingMutexTask<'unsafe_state_ref, State: ?Sized + 'unsafe_state_ref>:
    Send + Sized
{
    fn run_with_state(self, state: MutexGuard<'unsafe_state_ref, State>);
}

impl<
        'unsafe_state_ref,
        State: ?Sized + 'unsafe_state_ref,
        TFnOnce: FnOnce(MutexGuard<'unsafe_state_ref, State>) + Send + Sized,
    > NonBlockingMutexTask<'unsafe_state_ref, State> for TFnOnce
{
    fn run_with_state(self, state: MutexGuard<'unsafe_state_ref, State>) {
        self(state);
    }
}
