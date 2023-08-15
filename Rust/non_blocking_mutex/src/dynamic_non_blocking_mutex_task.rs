use crate::mutex_guard::MutexGuard;
use crate::non_blocking_mutex_task::NonBlockingMutexTask;

pub struct DynamicNonBlockingMutexTask<
    'captured_variables,
    'unsafe_state_ref,
    State: ?Sized + 'unsafe_state_ref,
> {
    run_with_state_box:
        Box<dyn FnOnce(MutexGuard<'unsafe_state_ref, State>) + Send + 'captured_variables>,
}

impl<'captured_variables, 'unsafe_state_ref, State>
    DynamicNonBlockingMutexTask<'captured_variables, 'unsafe_state_ref, State>
{
    pub fn from_fn_once_impl(
        run_with_state: impl FnOnce(MutexGuard<'unsafe_state_ref, State>) + Send + 'captured_variables,
    ) -> Self {
        Self {
            run_with_state_box: Box::new(run_with_state),
        }
    }

    pub fn from_fn_once_dyn_box(
        run_with_state_box: Box<
            dyn FnOnce(MutexGuard<'unsafe_state_ref, State>) + Send + 'captured_variables,
        >,
    ) -> Self {
        Self { run_with_state_box }
    }
}

impl<'captured_variables, 'unsafe_state_ref, State: ?Sized + 'unsafe_state_ref>
    NonBlockingMutexTask<'unsafe_state_ref, State>
    for DynamicNonBlockingMutexTask<'captured_variables, 'unsafe_state_ref, State>
{
    fn run_with_state(self, state: MutexGuard<'unsafe_state_ref, State>) {
        (self.run_with_state_box)(state);
    }
}
