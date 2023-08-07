use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nameof::name_of;
use non_blocking_mutex::mutex_guard::MutexGuard;
use non_blocking_mutex::non_blocking_mutex_for_sized_task_with_static_dispatch::NonBlockingMutexForSizedTaskWithStaticDispatch;
use non_blocking_mutex::NonBlockingMutex;
use std::hint;
use std::sync::Mutex;
use std::thread::{available_parallelism, scope};

fn increment_once_without_mutex(number: &mut usize) {
    *number += 1;
    black_box(number);
}

fn increment_once_under_non_blocking_mutex_static<Task: Send + FnOnce(MutexGuard<'_, usize>)>(
    non_blocking_mutex: &NonBlockingMutexForSizedTaskWithStaticDispatch<usize, Task>,
    task: Task,
) {
    non_blocking_mutex.run_if_first_or_schedule_on_first(task);
}

fn increment_once_under_non_blocking_mutex_dynamic(non_blocking_mutex: &NonBlockingMutex<usize>) {
    non_blocking_mutex.run_if_first_or_schedule_on_first(|mut state: MutexGuard<usize>| {
        *state += 1;
        black_box(state);
    });
}

fn increment_once_under_mutex_blockingly(number_mutex: &Mutex<usize>) {
    let mut number = number_mutex.lock().unwrap();
    *number += 1;
    black_box(*number);
}

fn increment_once_under_mutex_spinny(number_mutex: &Mutex<usize>) {
    loop {
        let try_lock_result = number_mutex.try_lock();

        if let Ok(mut number) = try_lock_result {
            *number += 1;
            black_box(*number);

            break;
        }
    }
}

fn increment_under_non_blocking_mutex_concurrently_static(
    max_concurrent_thread_count: usize,
    operation_count: usize,
    spin_under_lock_count: usize,
) {
    let non_blocking_mutex =
        NonBlockingMutexForSizedTaskWithStaticDispatch::new(max_concurrent_thread_count, 0);

    scope(|scope| {
        for _ in 0..max_concurrent_thread_count {
            scope.spawn(|| {
                for _i in 0..operation_count {
                    non_blocking_mutex.run_if_first_or_schedule_on_first(
                        move |mut state: MutexGuard<usize>| {
                            *state += 1;
                            black_box(*state);

                            let mut spin_under_lock_count_left = spin_under_lock_count;
                            while spin_under_lock_count_left != 0 {
                                hint::spin_loop();
                                spin_under_lock_count_left -= 1;
                            }
                        },
                    )
                }
            });
        }
    });
}

fn increment_under_non_blocking_mutex_concurrently_dynamic(
    max_concurrent_thread_count: usize,
    operation_count: usize,
    spin_under_lock_count: usize,
) {
    let non_blocking_mutex = NonBlockingMutex::new(max_concurrent_thread_count, 0);

    scope(|scope| {
        for _ in 0..max_concurrent_thread_count {
            scope.spawn(|| {
                for _i in 0..operation_count {
                    non_blocking_mutex.run_if_first_or_schedule_on_first(move |mut state| {
                        *state += 1;
                        black_box(*state);

                        let mut spin_under_lock_count_left = spin_under_lock_count;
                        while spin_under_lock_count_left != 0 {
                            hint::spin_loop();
                            spin_under_lock_count_left -= 1;
                        }
                    })
                }
            });
        }
    });
}

fn increment_under_mutex_blockingly_concurrently(
    max_concurrent_thread_count: usize,
    operation_count: usize,
    spin_under_lock_count: usize,
) {
    let state_mutex = Mutex::new(0);

    scope(|scope| {
        for _ in 0..max_concurrent_thread_count {
            scope.spawn(|| {
                for _i in 0..operation_count {
                    let mut state = state_mutex.lock().unwrap();

                    *state += 1;
                    black_box(*state);

                    let mut spin_under_lock_count_left = spin_under_lock_count;
                    while spin_under_lock_count_left != 0 {
                        hint::spin_loop();
                        spin_under_lock_count_left -= 1;
                    }
                }
            });
        }
    });
}

fn increment_under_mutex_spinny_concurrently(
    max_concurrent_thread_count: usize,
    operation_count: usize,
    spin_under_lock_count: usize,
) {
    let state_mutex = Mutex::new(0);

    scope(|scope| {
        for _ in 0..max_concurrent_thread_count {
            scope.spawn(|| {
                for _i in 0..operation_count {
                    loop {
                        let try_lock_result = state_mutex.try_lock();

                        if let Ok(mut state) = try_lock_result {
                            *state += 1;
                            black_box(*state);

                            let mut spin_under_lock_count_left = spin_under_lock_count;
                            while spin_under_lock_count_left != 0 {
                                hint::spin_loop();
                                spin_under_lock_count_left -= 1;
                            }

                            break;
                        }
                    }
                }
            });
        }
    });
}

fn criterion_benchmark(criterion: &mut Criterion) {
    struct BenchFnAndName {
        bench_fn: fn(usize, usize, usize),
        bench_fn_name: String,
    }

    let bench_fn_and_name_list = [
        BenchFnAndName {
            bench_fn: increment_under_non_blocking_mutex_concurrently_static,
            bench_fn_name: String::from(name_of!(
                increment_under_non_blocking_mutex_concurrently_static
            )),
        },
        BenchFnAndName {
            bench_fn: increment_under_non_blocking_mutex_concurrently_dynamic,
            bench_fn_name: String::from(name_of!(
                increment_under_non_blocking_mutex_concurrently_dynamic
            )),
        },
        BenchFnAndName {
            bench_fn: increment_under_mutex_blockingly_concurrently,
            bench_fn_name: String::from(name_of!(increment_under_mutex_blockingly_concurrently)),
        },
        BenchFnAndName {
            bench_fn: increment_under_mutex_spinny_concurrently,
            bench_fn_name: String::from(name_of!(increment_under_mutex_spinny_concurrently)),
        },
    ];

    let operation_count_per_thread_list = [1e3 as usize, 1e4 as usize];

    let spin_under_lock_count_list = [0usize, 1e1 as usize, 1e2 as usize];

    let max_concurrent_thread_count = available_parallelism().unwrap().get();

    criterion.bench_function(name_of!(increment_once_without_mutex), |bencher| {
        let ref mut number = black_box(0);
        bencher.iter(|| increment_once_without_mutex(number))
    });
    criterion.bench_function(
        "increment_once_under_non_blocking_mutex_static",
        |bencher| {
            let non_blocking_mutex = black_box(
                NonBlockingMutexForSizedTaskWithStaticDispatch::new(max_concurrent_thread_count, 0),
            );

            let task = |mut state: MutexGuard<usize>| {
                *state += 1;
                black_box(state);
            };

            bencher
                .iter(|| increment_once_under_non_blocking_mutex_static(&non_blocking_mutex, task))
        },
    );
    criterion.bench_function(
        name_of!(increment_once_under_non_blocking_mutex_dynamic),
        |bencher| {
            let non_blocking_mutex =
                black_box(NonBlockingMutex::new(max_concurrent_thread_count, 0));
            bencher.iter(|| increment_once_under_non_blocking_mutex_dynamic(&non_blocking_mutex))
        },
    );
    criterion.bench_function(name_of!(increment_once_under_mutex_blockingly), |bencher| {
        let number_mutex = black_box(Mutex::new(0));
        bencher.iter(|| increment_once_under_mutex_blockingly(&number_mutex))
    });
    criterion.bench_function(name_of!(increment_once_under_mutex_spinny), |bencher| {
        let number_mutex = black_box(Mutex::new(0));
        bencher.iter(|| increment_once_under_mutex_spinny(&number_mutex))
    });

    for spin_under_lock_count in spin_under_lock_count_list {
        for operation_count_per_thread in operation_count_per_thread_list {
            for bench_fn_and_name in bench_fn_and_name_list.iter() {
                let (bench_fn, bench_fn_name) =
                    (bench_fn_and_name.bench_fn, &bench_fn_and_name.bench_fn_name);
                criterion.bench_function(
                    &format!("{bench_fn_name}|{operation_count_per_thread}|{spin_under_lock_count}|{max_concurrent_thread_count}"),
                    |bencher| {
                        bencher.iter(|| {
                            bench_fn(black_box(max_concurrent_thread_count), black_box(operation_count_per_thread), black_box(spin_under_lock_count))
                        })
                    },
                );
            }
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
