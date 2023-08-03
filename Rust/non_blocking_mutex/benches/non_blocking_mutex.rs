use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nameof::name_of;
use non_blocking_mutex::{MutexGuard, NonBlockingMutex};
use std::hint;
use std::sync::Mutex;
use std::thread::{available_parallelism, scope};

fn increment_once_without_mutex(number: &mut usize) {
    *number += 1;
    black_box(number);
}

fn increment_once_under_non_blocking_mutex(non_blocking_mutex: &NonBlockingMutex<usize>) {
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

fn increment_under_non_blocking_mutex_concurrently(operation_count: usize, spin_wait_count: usize) {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();
    let non_blocking_mutex = NonBlockingMutex::new(max_concurrent_thread_count, 0);

    scope(|scope| {
        for _ in 0..max_concurrent_thread_count {
            scope.spawn(|| {
                for _i in 0..operation_count {
                    non_blocking_mutex.run_if_first_or_schedule_on_first(move |mut state| {
                        *state += 1;
                        black_box(*state);

                        let mut spin_wait_count_left = spin_wait_count;
                        while spin_wait_count_left != 0 {
                            hint::spin_loop();
                            spin_wait_count_left -= 1;
                        }
                    })
                }
            });
        }
    });
}

fn increment_under_mutex_blockingly_concurrently(operation_count: usize, spin_wait_count: usize) {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();
    let state_mutex = Mutex::new(0);

    scope(|scope| {
        for _ in 0..max_concurrent_thread_count {
            scope.spawn(|| {
                for _i in 0..operation_count {
                    let mut state = state_mutex.lock().unwrap();

                    *state += 1;
                    black_box(*state);

                    let mut spin_wait_count_left = spin_wait_count;
                    while spin_wait_count_left != 0 {
                        hint::spin_loop();
                        spin_wait_count_left -= 1;
                    }
                }
            });
        }
    });
}

fn increment_under_mutex_spinny_concurrently(operation_count: usize, spin_wait_count: usize) {
    let max_concurrent_thread_count = available_parallelism().unwrap().get();
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

                            let mut spin_wait_count_left = spin_wait_count;
                            while spin_wait_count_left != 0 {
                                hint::spin_loop();
                                spin_wait_count_left -= 1;
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
        bench_fn: fn(usize, usize),
        bench_fn_name: String,
    }

    let bench_fn_and_name_list = [
        BenchFnAndName {
            bench_fn: increment_under_non_blocking_mutex_concurrently,
            bench_fn_name: String::from(name_of!(increment_under_non_blocking_mutex_concurrently)),
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

    let operation_count_list = [1e3 as usize, 1e4 as usize];

    let spin_wait_count_list = [0usize, 1e1 as usize, 1e2 as usize];

    let max_concurrent_thread_count = available_parallelism().unwrap().get();

    criterion.bench_function(name_of!(increment_once_without_mutex), |bencher| {
        let ref mut number = black_box(0);
        bencher.iter(|| increment_once_without_mutex(number))
    });
    criterion.bench_function(
        name_of!(increment_once_under_non_blocking_mutex),
        |bencher| {
            let non_blocking_mutex =
                black_box(NonBlockingMutex::new(max_concurrent_thread_count, 0));
            bencher.iter(|| increment_once_under_non_blocking_mutex(&non_blocking_mutex))
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

    for spin_wait_count in spin_wait_count_list {
        for operation_count in operation_count_list {
            for bench_fn_and_name in bench_fn_and_name_list.iter() {
                let (bench_fn, bench_fn_name) =
                    (bench_fn_and_name.bench_fn, &bench_fn_and_name.bench_fn_name);
                criterion.bench_function(
                    &format!("{bench_fn_name}_{operation_count}_{spin_wait_count}"),
                    |bencher| {
                        bencher.iter(|| {
                            bench_fn(black_box(operation_count), black_box(spin_wait_count))
                        })
                    },
                );
            }
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
