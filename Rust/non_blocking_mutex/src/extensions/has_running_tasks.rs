use crate::non_blocking_mutex::NonBlockingMutex;
use crate::non_blocking_mutex_task::NonBlockingMutexTask;
use std::sync::atomic::Ordering;

impl<
        'captured_variables,
        State,
        TNonBlockingMutexTask: NonBlockingMutexTask<State> + 'captured_variables,
    > NonBlockingMutex<'captured_variables, State, TNonBlockingMutexTask>
{
    pub fn has_running_tasks(&self, ordering: Ordering) -> bool {
        return self.task_count.load(ordering) != 0;
    }

    pub fn has_no_running_tasks(&self, ordering: Ordering) -> bool {
        return self.task_count.load(ordering) == 0;
    }
}
