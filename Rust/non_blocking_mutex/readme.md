# `NonBlockingMutex`

`NonBlockingMutex` is needed to run actions
atomically without thread blocking, or context
switch, or spin lock contention, or rescheduling
on some scheduler

`NonBlockingMutex` is faster than `std::sync::Mutex`(both blocking and spinning)
when contention is high enough

Notice that `NonBlockingMutex` doesn't guarantee order
of execution, only atomicity of operations is guaranteed

# Examples
```rust
use non_blocking_mutex::NonBlockingMutex;
use std::thread::{available_parallelism};

let max_concurrent_thread_count = available_parallelism().unwrap().get();

let non_blocking_mutex = NonBlockingMutex::new(max_concurrent_thread_count, 0);
non_blocking_mutex.run_if_first_or_schedule_on_first(|mut state| {
    *state += 1;
});
```

# Benchmarks
## Single fast operation in single thread without contention
| benchmark_name                          |      time |
|:----------------------------------------|----------:|
| increment_once_without_mutex            |  0.228 ns |
| increment_once_under_non_blocking_mutex |  9.445 ns |
| increment_once_under_mutex_blockingly   |  8.851 ns |
| increment_once_under_mutex_spinny       | 10.603 ns |

## Emulating expensive operation by spinning N times under lock with many threads and highest contention
| Benchmark name                                  | Operation count per thread | Spin under lock count | Concurrent thread count | average_time |
|:------------------------------------------------|---------------------------:|----------------------:|------------------------:|-------------:|
| increment_under_non_blocking_mutex_concurrently |                      1_000 |                     0 |                      24 |     3.408 ms |
| increment_under_mutex_blockingly_concurrently   |                      1_000 |                     0 |                      24 |     1.072 ms |
| increment_under_mutex_spinny_concurrently       |                      1_000 |                     0 |                      24 |     4.376 ms |
| increment_under_non_blocking_mutex_concurrently |                     10_000 |                     0 |                      24 |    42.584 ms |
| increment_under_mutex_blockingly_concurrently   |                     10_000 |                     0 |                      24 |    14.960 ms |
| increment_under_mutex_spinny_concurrently       |                     10_000 |                     0 |                      24 |    94.658 ms |
| increment_under_non_blocking_mutex_concurrently |                      1_000 |                    10 |                      24 |    12.280 ms |
| increment_under_mutex_blockingly_concurrently   |                      1_000 |                    10 |                      24 |     8.345 ms |
| increment_under_mutex_spinny_concurrently       |                      1_000 |                    10 |                      24 |    34.977 ms |
| increment_under_non_blocking_mutex_concurrently |                     10_000 |                    10 |                      24 |    70.013 ms |
| increment_under_mutex_blockingly_concurrently   |                     10_000 |                    10 |                      24 |    84.143 ms |
| increment_under_mutex_spinny_concurrently       |                     10_000 |                    10 |                      24 |    349.07 ms |
| increment_under_non_blocking_mutex_concurrently |                      1_000 |                   100 |                      24 |    44.670 ms |
| increment_under_mutex_blockingly_concurrently   |                      1_000 |                   100 |                      24 |    47.335 ms |
| increment_under_mutex_spinny_concurrently       |                      1_000 |                   100 |                      24 |   117.570 ms |
| increment_under_non_blocking_mutex_concurrently |                     10_000 |                   100 |                      24 |   378.230 ms |
| increment_under_mutex_blockingly_concurrently   |                     10_000 |                   100 |                      24 |   801.090 ms |
| increment_under_mutex_spinny_concurrently       |                     10_000 |                   100 |                      24 |  1200.400 ms |