# `NonBlockingMutex`

[<img alt="github" src="https://img.shields.io/badge/github-non_blocking_mutex-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/ivanivanyuk1993/utility.non_blocking_mutex)
[<img alt="crates.io" src="https://img.shields.io/crates/v/non_blocking_mutex.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/non_blocking_mutex)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-non_blocking_mutex-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/non_blocking_mutex/latest/non_blocking_mutex/non_blocking_mutex/struct.NonBlockingMutex.html#impl-NonBlockingMutex%3C'captured_variables,+State,+TNonBlockingMutexTask%3E)
[![Build and test Rust](https://github.com/ivanivanyuk1993/utility.non_blocking_mutex/actions/workflows/rust.yml/badge.svg)](https://github.com/ivanivanyuk1993/utility.non_blocking_mutex/actions/workflows/rust.yml)

## Why you should use `NonBlockingMutex`

`NonBlockingMutex` is currently the fastest way to do
expensive calculations under lock, or do cheap calculations
under lock when concurrency/load/contention is very high -
see benchmarks in directory `benches` and run them with
```bash
cargo bench
```

## Installation

```bash
cargo add non_blocking_mutex
```

## Examples
### Optimized for 1 type of `NonBlockingMutexTask`
```rust
use non_blocking_mutex::mutex_guard::MutexGuard;
use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
use std::thread::{available_parallelism};

/// How many threads can physically access [NonBlockingMutex]
/// simultaneously, needed for computing `shard_count` of [ShardedQueue],
/// used to store queue of tasks
let max_concurrent_thread_count = available_parallelism().unwrap().get();

let non_blocking_mutex = NonBlockingMutex::new(max_concurrent_thread_count, 0);
/// Will infer exact type and size(0) of this [FnOnce] and
/// make sized [NonBlockingMutex] which takes only this exact [FnOnce]
/// without ever requiring [Box]-ing or dynamic dispatch
non_blocking_mutex.run_if_first_or_schedule_on_first(|mut state: MutexGuard<usize>| {
    *state += 1;
});
```

### Easy to use with any `FnOnce`, but may `Box` tasks and use dynamic dispatch when can't acquire lock on first try
```rust
use non_blocking_mutex::dynamic_non_blocking_mutex::DynamicNonBlockingMutex;
use std::thread::{available_parallelism, scope};

let mut state_snapshot_before_increment = 0;
let mut state_snapshot_after_increment = 0;

let mut state_snapshot_before_decrement = 0;
let mut state_snapshot_after_decrement = 0;

{
    /// How many threads can physically access [NonBlockingMutex]
    /// simultaneously, needed for computing `shard_count` of [ShardedQueue],
    /// used to store queue of tasks
    let max_concurrent_thread_count = available_parallelism().unwrap().get();

    /// Will work with any [FnOnce] and is easy to use,
    /// but will [Box] tasks and use dynamic dispatch
    /// when can't acquire lock on first try
    let non_blocking_mutex = DynamicNonBlockingMutex::new(max_concurrent_thread_count, 0);

    scope(|scope| {
        scope.spawn(|| {
            non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|mut state| {
                *(&mut state_snapshot_before_increment) = *state;
                *state += 1;
                *(&mut state_snapshot_after_increment) = *state;
            });
            non_blocking_mutex.run_fn_once_if_first_or_schedule_on_first(|mut state| {
                *(&mut state_snapshot_before_decrement) = *state;
                *state -= 1;
                *(&mut state_snapshot_after_decrement) = *state;
            });
        });
    });
}

assert_eq!(state_snapshot_before_increment, 0);
assert_eq!(state_snapshot_after_increment, 1);

assert_eq!(state_snapshot_before_decrement, 1);
assert_eq!(state_snapshot_after_decrement, 0);
```

### Optimized for multiple known types of [NonBlockingMutexTask] which capture variables
```rust
use non_blocking_mutex::mutex_guard::MutexGuard;
use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
use non_blocking_mutex::non_blocking_mutex_task::NonBlockingMutexTask;
use std::thread::{available_parallelism, scope};

let mut state_snapshot_before_increment = 0;
let mut state_snapshot_after_increment = 0;

let mut state_snapshot_before_decrement = 0;
let mut state_snapshot_after_decrement = 0;

{
    /// How many threads can physically access [NonBlockingMutex]
    /// simultaneously, needed for computing `shard_count` of [ShardedQueue],
    /// used to store queue of tasks
    let max_concurrent_thread_count = available_parallelism().unwrap().get();

    /// Will infer exact type and size of struct [Task] and
    /// make sized [NonBlockingMutex] which takes only [Task]
    /// without ever requiring [Box]-ing or dynamic dispatch
    let non_blocking_mutex = NonBlockingMutex::new(max_concurrent_thread_count, 0);

    scope(|scope| {
        scope.spawn(|| {
            non_blocking_mutex.run_if_first_or_schedule_on_first(
                Task::new_increment_and_store_snapshots(
                    &mut state_snapshot_before_increment,
                    &mut state_snapshot_after_increment,
                ),
            );
            non_blocking_mutex.run_if_first_or_schedule_on_first(
                Task::new_decrement_and_store_snapshots(
                    &mut state_snapshot_before_decrement,
                    &mut state_snapshot_after_decrement,
                ),
            );
        });
    });
}

assert_eq!(state_snapshot_before_increment, 0);
assert_eq!(state_snapshot_after_increment, 1);

assert_eq!(state_snapshot_before_decrement, 1);
assert_eq!(state_snapshot_after_decrement, 0);

struct SnapshotsBeforeAndAfterChangeRefs<
    'snapshot_before_change_ref,
    'snapshot_after_change_ref,
> {
    /// Where to write snapshot of `State` before applying function to `State`
    snapshot_before_change_ref: &'snapshot_before_change_ref mut usize,
    /// Where to write snapshot of `State` after applying function to `State
    snapshot_after_change_ref: &'snapshot_after_change_ref mut usize,
}

enum TaskType<'snapshot_before_change_ref, 'snapshot_after_change_ref> {
    IncrementAndStoreSnapshots(
        SnapshotsBeforeAndAfterChangeRefs<
            'snapshot_before_change_ref,
            'snapshot_after_change_ref,
        >,
    ),
    DecrementAndStoreSnapshots(
        SnapshotsBeforeAndAfterChangeRefs<
            'snapshot_before_change_ref,
            'snapshot_after_change_ref,
        >,
    ),
}

struct Task<'snapshot_before_change_ref, 'snapshot_after_change_ref> {
    task_type: TaskType<'snapshot_before_change_ref, 'snapshot_after_change_ref>,
}

impl<'snapshot_before_change_ref, 'snapshot_after_change_ref>
    Task<'snapshot_before_change_ref, 'snapshot_after_change_ref>
{
    fn new_increment_and_store_snapshots(
        // Where to write snapshot of `State` before applying function to `State`
        snapshot_before_change_ref: &'snapshot_before_change_ref mut usize,
        // Where to write snapshot of `State` after applying function to `State
        snapshot_after_change_ref: &'snapshot_after_change_ref mut usize,
    ) -> Self {
        Self {
            task_type: TaskType::IncrementAndStoreSnapshots(
                SnapshotsBeforeAndAfterChangeRefs {
                    /// Where to write snapshot of `State` before applying function to `State`
                    snapshot_before_change_ref,
                    /// Where to write snapshot of `State` after applying function to `State
                    snapshot_after_change_ref,
                },
            ),
        }
    }

    fn new_decrement_and_store_snapshots(
        // Where to write snapshot of `State` before applying function to `State`
        snapshot_before_change_ref: &'snapshot_before_change_ref mut usize,
        // Where to write snapshot of `State` after applying function to `State
        snapshot_after_change_ref: &'snapshot_after_change_ref mut usize,
    ) -> Self {
        Self {
            task_type: TaskType::DecrementAndStoreSnapshots(
                SnapshotsBeforeAndAfterChangeRefs {
                    /// Where to write snapshot of `State` before applying function to `State`
                    snapshot_before_change_ref,
                    /// Where to write snapshot of `State` after applying function to `State
                    snapshot_after_change_ref,
                },
            ),
        }
    }
}

impl<'snapshot_before_change_ref, 'snapshot_after_change_ref> NonBlockingMutexTask<usize>
    for Task<'snapshot_before_change_ref, 'snapshot_after_change_ref>
{
    fn run_with_state(self, mut state: MutexGuard<usize>) {
        match self.task_type {
            TaskType::IncrementAndStoreSnapshots(SnapshotsBeforeAndAfterChangeRefs {
                snapshot_before_change_ref,
                snapshot_after_change_ref,
            }) => {
                *snapshot_before_change_ref = *state;
                *state += 1;
                *snapshot_after_change_ref = *state;
            }
            TaskType::DecrementAndStoreSnapshots(SnapshotsBeforeAndAfterChangeRefs {
                snapshot_before_change_ref,
                snapshot_after_change_ref,
            }) => {
                *snapshot_before_change_ref = *state;
                *state -= 1;
                *snapshot_after_change_ref = *state;
            }
        }
    }
}
```

## Why you may want to not use `NonBlockingMutex`

- `NonBlockingMutex` forces first thread to enter synchronized block to
do all tasks(including added while it is running,
potentially running forever if tasks are being added forever)

- It is more difficult to continue execution on same thread after
synchronized logic is run, you need to schedule continuation on some
scheduler when you want to continue after end of synchronized logic
in new thread or introduce other synchronization primitives,
like channels, or `WaitGroup`-s, or similar

- `NonBlockingMutex` performs worse than `std::sync::Mutex` when
concurrency/load/contention is low

- Similar to `std::sync::Mutex`, `NonBlockingMutex` doesn't guarantee
order of execution, only atomicity of operations is guaranteed

## Benchmarks
See benchmark logic in directory `benches` and reproduce results by running
```bash
cargo bench
```
### Single fast operation in single thread without contention
Dynamic `NonBlockingMutex` performs only a little bit slower than `Mutex`
when there is only 1 thread and 1 operation
(because `NonBlockingMutex` doesn't `Box` and store in `ShardedQueue`
first operation in loop), while `NonBlockingMutexForSizedTaskWithStaticDispatch`
outperforms other synchronization options
when there is only 1 thread and 1 operation

| benchmark_name                                  |      time |
|:------------------------------------------------|----------:|
| increment_once_without_mutex                    |  0.228 ns |
| increment_once_under_non_blocking_mutex_static  |  8.544 ns |
| increment_once_under_non_blocking_mutex_dynamic |  9.445 ns |
| increment_once_under_mutex_blockingly           |  8.851 ns |
| increment_once_under_mutex_spinny               | 10.603 ns |

### Emulating expensive operation by spinning N times under lock with many threads and highest contention
With higher contention(caused by long time under lock in our case,
but can also be caused by higher CPU count), `NonBlockingMutex`
starts to perform better than `std::sync::Mutex`

| Benchmark name                                          | Operation count per thread | Spin under lock count | Concurrent thread count | average_time |
|:--------------------------------------------------------|---------------------------:|----------------------:|------------------------:|-------------:|
| increment_under_non_blocking_mutex_concurrently_static  |                      1_000 |                     0 |                      24 |     2.313 ms |
| increment_under_non_blocking_mutex_concurrently_dynamic |                      1_000 |                     0 |                      24 |     3.408 ms |
| increment_under_mutex_blockingly_concurrently           |                      1_000 |                     0 |                      24 |     1.072 ms |
| increment_under_mutex_spinny_concurrently               |                      1_000 |                     0 |                      24 |     4.376 ms |
| increment_under_non_blocking_mutex_concurrently_static  |                     10_000 |                     0 |                      24 |    23.969 ms |
| increment_under_non_blocking_mutex_concurrently_dynamic |                     10_000 |                     0 |                      24 |    42.584 ms |
| increment_under_mutex_blockingly_concurrently           |                     10_000 |                     0 |                      24 |    14.960 ms |
| increment_under_mutex_spinny_concurrently               |                     10_000 |                     0 |                      24 |    94.658 ms |
| increment_under_non_blocking_mutex_concurrently_static  |                      1_000 |                    10 |                      24 |     9.457 ms |
| increment_under_non_blocking_mutex_concurrently_dynamic |                      1_000 |                    10 |                      24 |    12.280 ms |
| increment_under_mutex_blockingly_concurrently           |                      1_000 |                    10 |                      24 |     8.345 ms |
| increment_under_mutex_spinny_concurrently               |                      1_000 |                    10 |                      24 |    34.977 ms |
| increment_under_non_blocking_mutex_concurrently_static  |                     10_000 |                    10 |                      24 |    58.297 ms |
| increment_under_non_blocking_mutex_concurrently_dynamic |                     10_000 |                    10 |                      24 |    70.013 ms |
| increment_under_mutex_blockingly_concurrently           |                     10_000 |                    10 |                      24 |    84.143 ms |
| increment_under_mutex_spinny_concurrently               |                     10_000 |                    10 |                      24 |   349.070 ms |
| increment_under_non_blocking_mutex_concurrently_static  |                      1_000 |                   100 |                      24 |    39.569 ms |
| increment_under_non_blocking_mutex_concurrently_dynamic |                      1_000 |                   100 |                      24 |    44.670 ms |
| increment_under_mutex_blockingly_concurrently           |                      1_000 |                   100 |                      24 |    47.335 ms |
| increment_under_mutex_spinny_concurrently               |                      1_000 |                   100 |                      24 |   117.570 ms |
| increment_under_non_blocking_mutex_concurrently_static  |                     10_000 |                   100 |                      24 |   358.480 ms |
| increment_under_non_blocking_mutex_concurrently_dynamic |                     10_000 |                   100 |                      24 |   378.230 ms |
| increment_under_mutex_blockingly_concurrently           |                     10_000 |                   100 |                      24 |   801.090 ms |
| increment_under_mutex_spinny_concurrently               |                     10_000 |                   100 |                      24 |  1200.400 ms |

## Design explanation

First thread, which calls `NonBlockingMutex::run_if_first_or_schedule_on_first`,
atomically increments `task_count`, and,
if thread was first to increment `task_count` from 0 to 1,
first thread immediately executes first task,
and then atomically decrements `task_count` and checks if `task_count`
changed from 1 to 0. If `task_count` changed from 1 to 0 -
there are no more tasks and first thread can finish execution loop,
otherwise first thread gets next task from `task_queue` and runs task,
then decrements tasks count after it was run and repeats check if
`task_count` changed from 1 to 0 and running tasks until there are no more tasks left.

Not first threads also atomically increment `task_count`,
do check if they are first, `Box` task and push task `Box` to `task_queue`

This design allows us to avoid lock contention, but adds ~constant time
of `Box`-ing task and putting task `Box` into concurrent `task_queue`, and
incrementing and decrementing `task_count`, so when lock contention is low,
`NonBlockingMutex` performs worse than `std::sync::Mutex`,
but when contention is high
(because we have more CPU-s or because we want to do expensive
calculations under lock), `NonBlockingMutex` performs better
than `std::sync::Mutex`