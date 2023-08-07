use crate::mutex_guard::MutexGuard;

/// Implementing custom [FnOnce] is unstable,
/// so we need [SizedTaskWithStaticDispatch]
pub trait SizedTaskWithStaticDispatch<'unsafe_state_ref, State: ?Sized + 'unsafe_state_ref>:
    Send + Sized
{
    fn run_with_state(self, state: MutexGuard<'unsafe_state_ref, State>);
}
