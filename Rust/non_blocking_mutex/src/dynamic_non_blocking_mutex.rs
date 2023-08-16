use crate::dynamic_non_blocking_mutex_task::DynamicNonBlockingMutexTask;
use crate::mutex_guard::MutexGuard;
use crate::non_blocking_mutex::NonBlockingMutex;
use crate::non_blocking_mutex_task::NonBlockingMutexTask;
use std::sync::atomic::Ordering;

pub type DynamicNonBlockingMutex<'captured_variables, State> = NonBlockingMutex<
    'captured_variables,
    State,
    DynamicNonBlockingMutexTask<'captured_variables, State>,
>;

impl<'captured_variables, State> DynamicNonBlockingMutex<'captured_variables, State> {
    /// Please don't forget that order of execution is not guaranteed. Atomicity of operations is guaranteed,
    /// but order can be random
    pub fn run_fn_once_if_first_or_schedule_on_first(
        &self,
        run_with_state: impl FnOnce(MutexGuard<State>) + Send + 'captured_variables,
    ) {
        if self.task_count.fetch_add(1, Ordering::Acquire) != 0 {
            self.task_queue
                .push_back(DynamicNonBlockingMutexTask::from_fn_once_impl(
                    run_with_state,
                ));
        } else {
            // If we acquired first lock, run should be executed immediately and run loop started
            run_with_state(unsafe { MutexGuard::new(&self.unsafe_state) });
            /// Note that if [`fetch_sub`] != 1
            /// => some thread entered first if block in method
            /// => [ShardedQueue::push_back] is guaranteed to be called
            /// => [ShardedQueue::pop_front_or_spin_wait_item] will not deadlock while spins until it gets item
            ///
            /// Notice that we run action first, and only then decrement count
            /// with releasing(pushing) memory changes, even if it looks otherwise
            while self.task_count.fetch_sub(1, Ordering::Release) != 1 {
                self.task_queue
                    .pop_front_or_spin_wait_item()
                    .run_with_state(unsafe { MutexGuard::new(&self.unsafe_state) });
            }
        }
    }
}
