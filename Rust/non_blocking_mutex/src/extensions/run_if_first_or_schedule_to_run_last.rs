use crate::dynamic_non_blocking_mutex::DynamicNonBlockingMutex;
use crate::dynamic_non_blocking_mutex_task::DynamicNonBlockingMutexTask;
use crate::mutex_guard::MutexGuard;
use crate::non_blocking_mutex::NonBlockingMutex;
use std::sync::atomic::Ordering;
use std::sync::Arc;

impl<'captured_variables, State: Send + 'captured_variables>
    DynamicNonBlockingMutex<'captured_variables, State>
{
    /// [NonBlockingMutex::run_if_first_or_schedule_to_run_last]
    /// should be called only if there won't be other calls to [NonBlockingMutex],
    /// to be sure that it is run last, like inside [Drop]
    pub unsafe fn run_if_first_or_schedule_to_run_last(
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

            let recursive_run_with_state = move |state_2: MutexGuard<State>| {
                Self::run_if_last_or_reschedule(
                    non_blocking_mutex_arc_clone,
                    run_with_state,
                    state_2,
                );
            };

            non_blocking_mutex_ref.task_queue.push_back(
                DynamicNonBlockingMutexTask::from_fn_once_impl(recursive_run_with_state),
            );
        } else {
            // If we acquired first lock, run should be executed immediately and run loop started
            run_with_state(unsafe { MutexGuard::new(&non_blocking_mutex_ref.unsafe_state) });
            // Since we know that no more calls will happen, we don't need to decrement
        }
    }

    unsafe fn run_if_last_or_reschedule(
        non_blocking_mutex_arc: Arc<Self>,
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
            let recursive_run_with_state = move |state_2: MutexGuard<State>| {
                Self::run_if_last_or_reschedule(
                    non_blocking_mutex_arc_clone,
                    run_with_state,
                    state_2,
                );
            };

            non_blocking_mutex_ref.task_queue.push_back(
                DynamicNonBlockingMutexTask::from_fn_once_impl(recursive_run_with_state),
            );
        }
    }
}
