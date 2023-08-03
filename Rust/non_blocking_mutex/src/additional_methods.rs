use std::sync::Arc;
use crate::{MutexGuard, NonBlockingMutex};
use std::sync::atomic::Ordering;

impl<'captured_variables, State> NonBlockingMutex<'captured_variables, State> {
    #[inline]
    pub fn has_no_running_tasks(&self, ordering: Ordering) -> bool {
        return self.task_count.load(ordering) == 0;
    }

    /// [NonBlockingMutex::run_knowing_that_no_other_runs_are_in_progress_and_no_more_runs_will_happen]
    /// should be called only if there won't be other calls
    /// to methods of [NonBlockingMutex],
    /// to be sure that it is run last, like inside [Drop]
    #[inline]
    pub unsafe fn run_knowing_that_no_other_runs_are_in_progress_and_no_more_runs_will_happen(
        &self,
        run_with_state: impl FnOnce(MutexGuard<State>) + 'captured_variables,
    ) {
        run_with_state(unsafe { MutexGuard::new(self) });
    }
}

impl<'captured_variables, State: Send + 'captured_variables>
NonBlockingMutex<'captured_variables, State>
{
    /// [NonBlockingMutex::run_if_first_or_schedule_on_first_to_run_last]
    /// should be called only if there won't be other calls
    /// to [NonBlockingMutex::run_if_first_or_schedule_on_first],
    /// to be sure that it is run last, like inside [Drop]
    #[inline]
    pub unsafe fn run_if_first_or_schedule_on_first_to_run_last(
        non_blocking_mutex_arc: Arc<Self>,
        run_with_state: impl FnOnce(MutexGuard<State>) + Send + 'captured_variables,
    ) {
        let non_blocking_mutex_ref = non_blocking_mutex_arc.as_ref();
        if non_blocking_mutex_ref
            .task_count
            .fetch_add(1, Ordering::Acquire)
            != 0
        {
            let non_blocking_mutex_arc_clone = non_blocking_mutex_arc.clone();

            let recursive_run_with_state = move |state2: MutexGuard<State>| {
                Self::run_if_last_or_reschedule(
                    non_blocking_mutex_arc_clone,
                    run_with_state,
                    state2,
                );
            };

            non_blocking_mutex_ref
                .task_queue
                .push_back(Box::new(recursive_run_with_state));
        } else {
            // If we acquired first lock, run should be executed immediately and run loop started
            run_with_state(unsafe { MutexGuard::new(non_blocking_mutex_ref) });
            // Since we know that no more calls will happen, we don't need to decrement
        }
    }

    unsafe fn run_if_last_or_reschedule(
        non_blocking_mutex_arc: Arc<NonBlockingMutex<'captured_variables, State>>,
        run_with_state: impl FnOnce(MutexGuard<State>) + Send + 'captured_variables,
        state: MutexGuard<State>,
    ) {
        let non_blocking_mutex_ref = non_blocking_mutex_arc.as_ref();
        if non_blocking_mutex_ref
            .task_count
            .fetch_add(1, Ordering::Relaxed)
            == 1
        {
            run_with_state(state);
        } else {
            let non_blocking_mutex_arc_clone = non_blocking_mutex_arc.clone();
            let recursive_run_with_state = move |state2: MutexGuard<State>| {
                Self::run_if_last_or_reschedule(
                    non_blocking_mutex_arc_clone,
                    run_with_state,
                    state2,
                );
            };

            non_blocking_mutex_ref
                .task_queue
                .push_back(Box::new(recursive_run_with_state));
        }
    }
}