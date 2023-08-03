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