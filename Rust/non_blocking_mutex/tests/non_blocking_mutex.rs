use non_blocking_mutex::mutex_guard::MutexGuard;
use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
use non_blocking_mutex::non_blocking_mutex_task::NonBlockingMutexTask;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{available_parallelism, scope};

#[test]
fn can_use_Fn() {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();

    let count_of_calls = AtomicUsize::new(0);
    let count_of_calls_ref = &count_of_calls;
    let mut count = 0;
    let mut last_state = 0;
    let last_state_ref = &mut last_state;

    let increment = |count_to_increment: &mut usize| {
        let incremented_count_of_calls = count_of_calls_ref.fetch_add(1, Ordering::Relaxed) + 1;
        *count_to_increment += incremented_count_of_calls;
    };

    {
        let non_blocking_mutex = NonBlockingMutex::new(max_concurrent_thread_count, 0usize);

        increment(&mut count);
        increment(&mut count);

        enum TaskArgsEnum<'last_state_ref, Increment: FnMut(&mut usize) + Send> {
            Increment(Increment),
            SetLastState(&'last_state_ref mut usize),
        }

        struct Task<'last_state_ref, Increment: FnMut(&mut usize) + Send> {
            task_args_enum: TaskArgsEnum<'last_state_ref, Increment>,
        }

        impl<'last_state_ref, Increment: FnMut(&mut usize) + Send> Task<'last_state_ref, Increment> {
            pub fn new_increment(increment: Increment) -> Self {
                Self {
                    task_args_enum: TaskArgsEnum::Increment(increment),
                }
            }

            pub fn new_set_last_state(last_state_ref: &'last_state_ref mut usize) -> Self {
                Self {
                    task_args_enum: TaskArgsEnum::SetLastState(last_state_ref),
                }
            }
        }

        impl<'last_state_ref, 'unsafe_state_ref, Increment: FnMut(&mut usize) + Send>
            NonBlockingMutexTask<'unsafe_state_ref, usize> for Task<'last_state_ref, Increment>
        {
            fn run_with_state(self, mut state: MutexGuard<'unsafe_state_ref, usize>) -> () {
                match self.task_args_enum {
                    TaskArgsEnum::Increment(mut increment) => {
                        increment(&mut state);
                        increment(&mut state);
                    }
                    TaskArgsEnum::SetLastState(last_state) => {
                        *last_state = *state;
                    }
                }
            }
        }

        non_blocking_mutex.run_if_first_or_schedule_on_first(Task::new_increment(increment));

        non_blocking_mutex.run_if_first_or_schedule_on_first(Task::new_increment(increment));

        non_blocking_mutex
            .run_if_first_or_schedule_on_first(Task::new_set_last_state(last_state_ref));
    }

    assert_eq!(count_of_calls.load(Ordering::Relaxed), 6);
    assert_eq!(count, 3);
    assert_eq!(last_state, 18);
}

#[test]
fn can_use_Fn_recursively() {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();

    let count_of_calls = AtomicUsize::new(0);
    let count_of_calls_ref = &count_of_calls;
    let mut count = 0;
    let mut last_state = 0;
    let last_state_ref = &mut last_state;
    let mut state_1 = 0usize;
    let state_1_ref = &mut state_1;
    let mut state_2 = 0usize;
    let state_2_ref = &mut state_2;

    let increment = |count_to_increment: &mut usize| {
        let incremented_count_of_calls = count_of_calls_ref.fetch_add(1, Ordering::Relaxed) + 1;
        *count_to_increment += incremented_count_of_calls;
    };

    {
        let non_blocking_mutex_arc =
            Arc::new(NonBlockingMutex::new(max_concurrent_thread_count, 0));
        let non_blocking_mutex = non_blocking_mutex_arc.as_ref();

        increment(&mut count);
        increment(&mut count);

        enum TaskArgsEnum<'last_state_ref, 'state_ref, Increment: FnMut(&mut usize) + Send> {
            IncrementAndRunRecursion(
                Increment,
                Arc<NonBlockingMutex<usize, Task<'last_state_ref, 'state_ref, Increment>>>,
                &'state_ref mut usize,
            ),
            IncrementAndSetStateSnapshot(Increment, &'state_ref mut usize),
            SetLastState(&'last_state_ref mut usize),
        }

        struct Task<'last_state_ref, 'state_ref, Increment: FnMut(&mut usize) + Send> {
            task_args_enum: TaskArgsEnum<'last_state_ref, 'state_ref, Increment>,
        }

        impl<'last_state_ref, 'state_ref, Increment: FnMut(&mut usize) + Send>
            Task<'last_state_ref, 'state_ref, Increment>
        {
            pub fn new_increment_and_run_recursive(
                increment: Increment,
                non_blocking_mutex_arc: Arc<
                    NonBlockingMutex<usize, Task<'last_state_ref, 'state_ref, Increment>>,
                >,
                state_ref: &'state_ref mut usize,
            ) -> Self {
                Self {
                    task_args_enum: TaskArgsEnum::IncrementAndRunRecursion(
                        increment,
                        non_blocking_mutex_arc,
                        state_ref,
                    ),
                }
            }

            pub fn new_increment_and_set_state_snapshot(
                increment: Increment,
                state_ref: &'state_ref mut usize,
            ) -> Self {
                Self {
                    task_args_enum: TaskArgsEnum::IncrementAndSetStateSnapshot(
                        increment, state_ref,
                    ),
                }
            }

            pub fn new_set_last_state(last_state_ref: &'last_state_ref mut usize) -> Self {
                Self {
                    task_args_enum: TaskArgsEnum::SetLastState(last_state_ref),
                }
            }
        }

        impl<
                'last_state_ref,
                'state_ref,
                'unsafe_state_ref,
                Increment: FnMut(&mut usize) + Send,
            > NonBlockingMutexTask<'unsafe_state_ref, usize>
            for Task<'last_state_ref, 'state_ref, Increment>
        {
            fn run_with_state(self, mut state: MutexGuard<'unsafe_state_ref, usize>) -> () {
                match self.task_args_enum {
                    TaskArgsEnum::IncrementAndRunRecursion(
                        mut increment,
                        non_blocking_mutex_arc,
                        state_ref,
                    ) => {
                        increment(&mut state);
                        increment(&mut state);
                        non_blocking_mutex_arc.run_if_first_or_schedule_on_first(
                            Task::new_increment_and_set_state_snapshot(increment, state_ref),
                        );
                    }
                    TaskArgsEnum::IncrementAndSetStateSnapshot(mut increment, state_ref) => {
                        increment(&mut state);
                        increment(&mut state);

                        *state_ref = *state;
                    }
                    TaskArgsEnum::SetLastState(last_state) => {
                        *last_state = *state;
                    }
                }
            }
        }

        non_blocking_mutex.run_if_first_or_schedule_on_first(
            Task::new_increment_and_run_recursive(
                increment,
                non_blocking_mutex_arc.clone(),
                state_1_ref,
            ),
        );

        non_blocking_mutex.run_if_first_or_schedule_on_first(
            Task::new_increment_and_run_recursive(
                increment,
                non_blocking_mutex_arc.clone(),
                state_2_ref,
            ),
        );

        non_blocking_mutex
            .run_if_first_or_schedule_on_first(Task::new_set_last_state(last_state_ref));
    }

    assert_eq!(count_of_calls.load(Ordering::Relaxed), 10);
    assert_eq!(state_1, 18);
    assert_eq!(state_2, 52);
    assert_eq!(count, 3);
    assert_eq!(last_state, 52);
}

#[test]
fn can_use_FnMut() {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();

    let mut count_of_calls = 0;
    let count_of_calls_ref = &mut count_of_calls;
    let mut count = 0;
    let mut last_state = 0;
    let last_state_ref = &mut last_state;

    let mut increment = |count_to_increment: &mut usize| {
        *count_of_calls_ref += 1;
        *count_to_increment += *count_of_calls_ref;
    };

    {
        let non_blocking_mutex = NonBlockingMutex::new(max_concurrent_thread_count, 0);

        increment(&mut count);
        increment(&mut count);

        enum TaskArgsEnum<'last_state_ref, Increment: FnMut(&mut usize) + Send> {
            Increment(Increment),
            SetLastState(&'last_state_ref mut usize),
        }

        struct Task<'last_state_ref, Increment: FnMut(&mut usize) + Send> {
            task_args_enum: TaskArgsEnum<'last_state_ref, Increment>,
        }

        impl<'last_state_ref, Increment: FnMut(&mut usize) + Send> Task<'last_state_ref, Increment> {
            pub fn new_increment(increment: Increment) -> Self {
                Self {
                    task_args_enum: TaskArgsEnum::Increment(increment),
                }
            }

            pub fn new_set_last_state(last_state_ref: &'last_state_ref mut usize) -> Self {
                Self {
                    task_args_enum: TaskArgsEnum::SetLastState(last_state_ref),
                }
            }
        }

        impl<'last_state_ref, 'unsafe_state_ref, Increment: FnMut(&mut usize) + Send>
            NonBlockingMutexTask<'unsafe_state_ref, usize> for Task<'last_state_ref, Increment>
        {
            fn run_with_state(self, mut state: MutexGuard<'unsafe_state_ref, usize>) -> () {
                match self.task_args_enum {
                    TaskArgsEnum::Increment(mut increment) => {
                        increment(&mut state);
                        increment(&mut state);
                    }
                    TaskArgsEnum::SetLastState(last_state) => {
                        *last_state = *state;
                    }
                }
            }
        }

        non_blocking_mutex.run_if_first_or_schedule_on_first(Task::new_increment(increment));

        non_blocking_mutex
            .run_if_first_or_schedule_on_first(Task::new_set_last_state(last_state_ref));
    }

    assert_eq!(count_of_calls, 4);
    assert_eq!(count, 3);
    assert_eq!(last_state, 7);
}

#[test]
fn can_use_FnMut_recursively() {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();

    let mut count_of_calls = 0;
    let count_of_calls_ref = &mut count_of_calls;
    let mut count = 0;
    let mut last_state = 0;
    let last_state_ref = &mut last_state;
    let mut state_1 = 0usize;
    let state_1_ref = &mut state_1;
    let mut state_2 = 0usize;
    let state_2_ref = &mut state_2;

    let mut increment = |count_to_increment: &mut usize| {
        *count_of_calls_ref += 1;
        *count_to_increment += *count_of_calls_ref;
    };

    {
        let non_blocking_mutex_arc =
            Arc::new(NonBlockingMutex::new(max_concurrent_thread_count, 0));
        let non_blocking_mutex = non_blocking_mutex_arc.as_ref();

        increment(&mut count);
        increment(&mut count);

        enum TaskArgsEnum<'last_state_ref, 'state_ref, Increment: FnMut(&mut usize) + Send> {
            IncrementAndRunRecursion(
                Increment,
                Arc<NonBlockingMutex<usize, Task<'last_state_ref, 'state_ref, Increment>>>,
                &'state_ref mut usize,
            ),
            IncrementAndSetStateSnapshot(Increment, &'state_ref mut usize),
            SetLastState(&'last_state_ref mut usize),
        }

        struct Task<'last_state_ref, 'state_ref, Increment: FnMut(&mut usize) + Send> {
            task_args_enum: TaskArgsEnum<'last_state_ref, 'state_ref, Increment>,
        }

        impl<'last_state_ref, 'state_ref, Increment: FnMut(&mut usize) + Send>
            Task<'last_state_ref, 'state_ref, Increment>
        {
            pub fn new_increment_and_run_recursive(
                increment: Increment,
                non_blocking_mutex_arc: Arc<
                    NonBlockingMutex<usize, Task<'last_state_ref, 'state_ref, Increment>>,
                >,
                state_ref: &'state_ref mut usize,
            ) -> Self {
                Self {
                    task_args_enum: TaskArgsEnum::IncrementAndRunRecursion(
                        increment,
                        non_blocking_mutex_arc,
                        state_ref,
                    ),
                }
            }

            pub fn new_increment_and_set_state_snapshot(
                increment: Increment,
                state_ref: &'state_ref mut usize,
            ) -> Self {
                Self {
                    task_args_enum: TaskArgsEnum::IncrementAndSetStateSnapshot(
                        increment, state_ref,
                    ),
                }
            }

            pub fn new_set_last_state(last_state_ref: &'last_state_ref mut usize) -> Self {
                Self {
                    task_args_enum: TaskArgsEnum::SetLastState(last_state_ref),
                }
            }
        }

        impl<
                'last_state_ref,
                'state_ref,
                'unsafe_state_ref,
                Increment: FnMut(&mut usize) + Send,
            > NonBlockingMutexTask<'unsafe_state_ref, usize>
            for Task<'last_state_ref, 'state_ref, Increment>
        {
            fn run_with_state(self, mut state: MutexGuard<'unsafe_state_ref, usize>) -> () {
                match self.task_args_enum {
                    TaskArgsEnum::IncrementAndRunRecursion(
                        mut increment,
                        non_blocking_mutex_arc,
                        state_ref,
                    ) => {
                        increment(&mut state);
                        increment(&mut state);
                        non_blocking_mutex_arc.run_if_first_or_schedule_on_first(
                            Task::new_increment_and_set_state_snapshot(increment, state_ref),
                        );
                    }
                    TaskArgsEnum::IncrementAndSetStateSnapshot(mut increment, state_ref) => {
                        increment(&mut state);
                        increment(&mut state);

                        *state_ref = *state;
                    }
                    TaskArgsEnum::SetLastState(last_state) => {
                        *last_state = *state;
                    }
                }
            }
        }

        non_blocking_mutex.run_if_first_or_schedule_on_first(
            Task::new_increment_and_run_recursive(
                &mut increment,
                non_blocking_mutex_arc.clone(),
                state_1_ref,
            ),
        );

        non_blocking_mutex
            .run_if_first_or_schedule_on_first(Task::new_set_last_state(last_state_ref));
    }

    assert_eq!(count, 3);
    assert_eq!(count_of_calls, 6);
    assert_eq!(last_state, 18);
    assert_eq!(state_1, 18);
}

#[test]
fn small_state_is_expected() {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();
    let non_blocking_mutex = NonBlockingMutex::new(max_concurrent_thread_count, 0);
    let non_blocking_mutex_ref = &non_blocking_mutex;
    let operation_count = 1e4 as usize;

    enum TaskArgsEnum<'state_ref> {
        Increment,
        SetStateSnapshot(&'state_ref mut usize),
    }

    struct Task<'state_ref> {
        task_args_enum: TaskArgsEnum<'state_ref>,
    }

    impl<'state_ref> Task<'state_ref> {
        pub fn new_increment() -> Self {
            Self {
                task_args_enum: TaskArgsEnum::Increment,
            }
        }

        pub fn new_set_state_snapshot(state_ref: &'state_ref mut usize) -> Self {
            Self {
                task_args_enum: TaskArgsEnum::SetStateSnapshot(state_ref),
            }
        }
    }

    impl<'state_ref, 'unsafe_state_ref> NonBlockingMutexTask<'unsafe_state_ref, usize>
        for Task<'state_ref>
    {
        fn run_with_state(self, mut state: MutexGuard<'unsafe_state_ref, usize>) -> () {
            match self.task_args_enum {
                TaskArgsEnum::Increment => {
                    *state += 1;
                }
                TaskArgsEnum::SetStateSnapshot(state_ref) => {
                    *state_ref = *state;
                }
            }
        }
    }

    scope(move |scope| {
        for _ in 0..max_concurrent_thread_count {
            scope.spawn(move || {
                for _i in 0..operation_count {
                    non_blocking_mutex_ref.run_if_first_or_schedule_on_first(Task::new_increment());
                }
            });
        }
    });

    let expected_state = operation_count * max_concurrent_thread_count;
    let mut actual_state = 0;
    non_blocking_mutex
        .run_if_first_or_schedule_on_first(Task::new_set_state_snapshot(&mut actual_state));

    assert_eq!(expected_state, actual_state);
}

#[test]
fn big_state_is_expected() {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();

    #[derive(Copy, Clone, Debug, PartialEq)]
    struct BigState {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    }
    let non_blocking_mutex = NonBlockingMutex::new(
        max_concurrent_thread_count,
        BigState {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
        },
    );
    let non_blocking_mutex_ref = &non_blocking_mutex;
    let operation_count = 1e4 as usize;

    enum TaskArgsEnum<'state_ref> {
        Increment,
        SetStateSnapshot(&'state_ref mut BigState),
    }

    struct Task<'state_ref> {
        task_args_enum: TaskArgsEnum<'state_ref>,
    }

    impl<'state_ref> Task<'state_ref> {
        pub fn new_increment() -> Self {
            Self {
                task_args_enum: TaskArgsEnum::Increment,
            }
        }

        pub fn new_set_state_snapshot(state_ref: &'state_ref mut BigState) -> Self {
            Self {
                task_args_enum: TaskArgsEnum::SetStateSnapshot(state_ref),
            }
        }
    }

    impl<'state_ref, 'unsafe_state_ref> NonBlockingMutexTask<'unsafe_state_ref, BigState>
        for Task<'state_ref>
    {
        fn run_with_state(self, mut state: MutexGuard<'unsafe_state_ref, BigState>) -> () {
            match self.task_args_enum {
                TaskArgsEnum::Increment => {
                    state.a += 1;
                    state.b += 2;
                    state.c += 3;
                    state.d += 4;
                }
                TaskArgsEnum::SetStateSnapshot(state_ref) => {
                    *state_ref = *state;
                }
            }
        }
    }

    scope(|scope| {
        for _ in 0..max_concurrent_thread_count {
            scope.spawn(|| {
                for _i in 0..operation_count {
                    non_blocking_mutex_ref.run_if_first_or_schedule_on_first(Task::new_increment());
                }
            });
        }
    });

    let expected_state = BigState {
        a: operation_count * max_concurrent_thread_count,
        b: operation_count * max_concurrent_thread_count * 2,
        c: operation_count * max_concurrent_thread_count * 3,
        d: operation_count * max_concurrent_thread_count * 4,
    };
    let mut actual_state = BigState {
        a: 0,
        b: 0,
        c: 0,
        d: 0,
    };
    non_blocking_mutex
        .run_if_first_or_schedule_on_first(Task::new_set_state_snapshot(&mut actual_state));

    assert_eq!(expected_state, actual_state);
}

#[test]
fn run_count_is_expected() {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();
    let operation_count = 1e4 as usize;

    // Create an atomic counter to track the number of actions executed
    let task_counter = AtomicUsize::new(0);

    enum TaskArgsEnum<'atomic_counter_ref, 'state_ref> {
        Increment(&'atomic_counter_ref AtomicUsize),
        SetStateSnapshot(&'state_ref mut usize),
    }

    struct Task<'atomic_counter_ref, 'state_ref> {
        task_args_enum: TaskArgsEnum<'atomic_counter_ref, 'state_ref>,
    }

    impl<'atomic_counter_ref, 'state_ref> Task<'atomic_counter_ref, 'state_ref> {
        pub fn new_increment(atomic_counter: &'atomic_counter_ref AtomicUsize) -> Self {
            Self {
                task_args_enum: TaskArgsEnum::Increment(atomic_counter),
            }
        }

        pub fn new_set_state_snapshot(state_ref: &'state_ref mut usize) -> Self {
            Self {
                task_args_enum: TaskArgsEnum::SetStateSnapshot(state_ref),
            }
        }
    }

    impl<'atomic_counter_ref, 'state_ref, 'unsafe_state_ref>
        NonBlockingMutexTask<'unsafe_state_ref, usize> for Task<'atomic_counter_ref, 'state_ref>
    {
        fn run_with_state(self, mut state: MutexGuard<'unsafe_state_ref, usize>) -> () {
            match self.task_args_enum {
                TaskArgsEnum::Increment(atomic_counter) => {
                    // Increment the state and the action counter atomically
                    *state += 1;
                    atomic_counter.fetch_add(1, Ordering::Relaxed);
                    if *state != atomic_counter.load(Ordering::Relaxed) {
                        // If state is not expected, we decrement it to fail test later
                        *state -= 1;
                    }
                }
                TaskArgsEnum::SetStateSnapshot(state_ref) => {
                    *state_ref = *state;
                }
            }
        }
    }

    let mut actual_state = 0;

    {
        let non_blocking_mutex = NonBlockingMutex::new(max_concurrent_thread_count, 0);

        scope(|scope| {
            for _ in 0..max_concurrent_thread_count {
                scope.spawn(|| {
                    for _i in 0..operation_count {
                        non_blocking_mutex
                            .run_if_first_or_schedule_on_first(Task::new_increment(&task_counter));
                    }
                });
            }
        });

        non_blocking_mutex
            .run_if_first_or_schedule_on_first(Task::new_set_state_snapshot(&mut actual_state));
    }

    // Check that the final state is equal to the expected state
    let expected_state = operation_count * max_concurrent_thread_count;
    assert_eq!(expected_state, actual_state);

    // Check that the action counter is equal to the expected state
    assert_eq!(expected_state, task_counter.load(Ordering::Relaxed));
}
