use crate::mutex_guard::MutexGuard;
use crate::non_blocking_mutex_task::NonBlockingMutexTask;

pub struct DynamicNonBlockingMutexTask<'captured_variables, State: ?Sized> {
    run_with_state_box: Box<dyn FnOnce(MutexGuard<State>) + Send + 'captured_variables>,
}

impl<'captured_variables, State> DynamicNonBlockingMutexTask<'captured_variables, State> {
    pub fn from_fn_once_impl(
        run_with_state: impl FnOnce(MutexGuard<State>) + Send + 'captured_variables,
    ) -> Self {
        Self {
            run_with_state_box: Box::new(run_with_state),
        }
    }

    pub fn from_fn_once_dyn_box(
        run_with_state_box: Box<dyn FnOnce(MutexGuard<State>) + Send + 'captured_variables>,
    ) -> Self {
        Self { run_with_state_box }
    }
}

impl<'captured_variables, State: ?Sized> NonBlockingMutexTask<State>
    for DynamicNonBlockingMutexTask<'captured_variables, State>
{
    fn run_with_state(self, state: MutexGuard<State>) {
        (self.run_with_state_box)(state);
    }
}
