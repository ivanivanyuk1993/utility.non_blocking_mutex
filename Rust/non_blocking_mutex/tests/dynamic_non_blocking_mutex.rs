use non_blocking_mutex::dynamic_non_blocking_mutex::DynamicNonBlockingMutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{available_parallelism, scope};

#[test]
fn can_use_Fn() {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();

    let count_of_calls = AtomicUsize::new(0);
    let count_of_calls_ref = &count_of_calls;
    let mut count = 0;
    let mut last_state = 0;

    let increment = |count_to_increment: &mut usize| {
        let incremented_count_of_calls = count_of_calls_ref.fetch_add(1, Ordering::Relaxed) + 1;
        *count_to_increment += incremented_count_of_calls;
    };

    {
        let non_blocking_mutex = DynamicNonBlockingMutex::new(max_concurrent_thread_count, 0usize);

        increment(&mut count);
        increment(&mut count);

        non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|mut state_count| {
            increment(&mut state_count);
            increment(&mut state_count);
        });

        non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|mut state_count| {
            increment(&mut state_count);
            increment(&mut state_count);
        });

        non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|state_count| {
            last_state = *state_count;
        });
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
    let mut state_1 = 0usize;
    let mut state_2 = 0usize;

    let increment = |count_to_increment: &mut usize| {
        let incremented_count_of_calls = count_of_calls_ref.fetch_add(1, Ordering::Relaxed) + 1;
        *count_to_increment += incremented_count_of_calls;
    };

    {
        let non_blocking_mutex_arc =
            Arc::new(DynamicNonBlockingMutex::new(max_concurrent_thread_count, 0));
        let non_blocking_mutex = non_blocking_mutex_arc.as_ref();
        let non_blocking_mutex_arc_clone = non_blocking_mutex_arc.clone();
        let non_blocking_mutex_arc_clone_2 = non_blocking_mutex_arc.clone();

        increment(&mut count);
        increment(&mut count);

        non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|mut state_count| {
            increment(&mut state_count);
            increment(&mut state_count);
            non_blocking_mutex_arc_clone.run_fn_once_if_first_or_schedule_on_first(|mut state_count_2| {
                increment(&mut state_count_2);
                increment(&mut state_count_2);
                state_1 = *state_count_2;
            });
            drop(non_blocking_mutex_arc_clone);
        });

        non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|mut state_count| {
            increment(&mut state_count);
            increment(&mut state_count);
            non_blocking_mutex_arc_clone_2.run_fn_once_if_first_or_schedule_on_first(
                |mut state_count_2| {
                    increment(&mut state_count_2);
                    increment(&mut state_count_2);
                    state_2 = *state_count_2;
                },
            );
            drop(non_blocking_mutex_arc_clone_2);
        });

        non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|state_count| {
            last_state = *state_count;
        });
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

    let mut increment = |count_to_increment: &mut usize| {
        *count_of_calls_ref += 1;
        *count_to_increment += *count_of_calls_ref;
    };

    {
        let non_blocking_mutex = DynamicNonBlockingMutex::new(max_concurrent_thread_count, 0);

        increment(&mut count);
        increment(&mut count);

        non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|mut state_count| {
            increment(&mut state_count);
            increment(&mut state_count);
        });

        non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|state_count| {
            last_state = *state_count;
        });
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

    let mut increment = |count_to_increment: &mut usize| {
        *count_of_calls_ref += 1;
        *count_to_increment += *count_of_calls_ref;
    };

    {
        let non_blocking_mutex_arc =
            Arc::new(DynamicNonBlockingMutex::new(max_concurrent_thread_count, 0));
        let non_blocking_mutex = non_blocking_mutex_arc.as_ref();
        let non_blocking_mutex_arc_clone = non_blocking_mutex_arc.clone();

        increment(&mut count);
        increment(&mut count);

        non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|mut state_count| {
            increment(&mut state_count);
            increment(&mut state_count);
            non_blocking_mutex_arc_clone.run_fn_once_if_first_or_schedule_on_first(
                |mut state_count_2| {
                    increment(&mut state_count_2);
                    increment(&mut state_count_2);
                },
            );
            drop(non_blocking_mutex_arc_clone);
        });

        non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|state_count| {
            last_state = *state_count;
        });
    }

    assert_eq!(count_of_calls, 6);
    assert_eq!(count, 3);
    assert_eq!(last_state, 18);
}

#[test]
fn small_state_is_expected() {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();
    let non_blocking_mutex = DynamicNonBlockingMutex::new(max_concurrent_thread_count, 0);
    let non_blocking_mutex_ref = &non_blocking_mutex;
    let operation_count = 1e4 as usize;

    scope(move |scope| {
        for _ in 0..max_concurrent_thread_count {
            scope.spawn(move || {
                for _i in 0..operation_count {
                    non_blocking_mutex_ref.run_fn_once_if_first_or_schedule_on_first(|mut state| {
                        *state += 1;
                    });
                }
            });
        }
    });

    let expected_state = operation_count * max_concurrent_thread_count;
    let actual_state_arc = Arc::new(AtomicUsize::new(0));
    let actual_state_arc_clone = actual_state_arc.clone();
    non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(move |state| {
        actual_state_arc_clone.store(*state, Ordering::Relaxed);
    });

    assert_eq!(expected_state, actual_state_arc.load(Ordering::Relaxed));
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
    let non_blocking_mutex = DynamicNonBlockingMutex::new(
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

    scope(|scope| {
        for _ in 0..max_concurrent_thread_count {
            scope.spawn(|| {
                for _i in 0..operation_count {
                    non_blocking_mutex_ref.run_fn_once_if_first_or_schedule_on_first(
                        |mut state| {
                            state.a += 1;
                            state.b += 2;
                            state.c += 3;
                            state.d += 4;
                        },
                    );
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
    let actual_state_mutex_arc = Arc::new(Mutex::new(BigState {
        a: 0,
        b: 0,
        c: 0,
        d: 0,
    }));
    let actual_state_mutex_arc_clone = actual_state_mutex_arc.clone();
    non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(move |state| {
        let mut actual_state = actual_state_mutex_arc_clone.lock().unwrap();
        *actual_state = *state;
    });

    let actual_state = actual_state_mutex_arc.lock().unwrap();

    assert_eq!(expected_state, *actual_state);
}

#[test]
fn run_count_is_expected() {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();
    let non_blocking_mutex = DynamicNonBlockingMutex::new(max_concurrent_thread_count, 0);
    let non_blocking_mutex_ref = &non_blocking_mutex;
    let operation_count = 1e4 as usize;

    // Create an atomic counter to track the number of actions executed
    let task_counter = Arc::new(AtomicUsize::new(0));

    scope(|scope| {
        for _ in 0..max_concurrent_thread_count {
            let task_counter_clone = task_counter.clone();
            scope.spawn(move || {
                for _i in 0..operation_count {
                    let task_counter_clone_clone = task_counter_clone.clone();
                    non_blocking_mutex_ref.run_fn_once_if_first_or_schedule_on_first(move |mut state| {
                        // Increment the state and the action counter atomically
                        *state += 1;
                        task_counter_clone_clone.fetch_add(1, Ordering::Relaxed);
                        if *state != task_counter_clone_clone.load(Ordering::Relaxed) {
                            // If state is not expected, we decrement it to fail test later
                            *state -= 1;
                        }
                    });
                }
            });
        }
    });

    // Check that the final state is equal to the expected state
    let expected_state = operation_count * max_concurrent_thread_count;
    let actual_state_arc = Arc::new(AtomicUsize::new(0));
    let actual_state_arc_clone = actual_state_arc.clone();
    non_blocking_mutex_ref.run_fn_once_if_first_or_schedule_on_first(move |state| {
        actual_state_arc_clone.store(*state, Ordering::Relaxed);
    });
    assert_eq!(expected_state, actual_state_arc.load(Ordering::Relaxed));

    // Check that the action counter is equal to the expected state
    assert_eq!(expected_state, task_counter.load(Ordering::Relaxed));
}
