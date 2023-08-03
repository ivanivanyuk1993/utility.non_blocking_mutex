# `NonBlockingMutex`

`NonBlockingMutex` is needed to run actions
atomically without thread blocking, or context
switch, or spin lock contention, or rescheduling
on some scheduler

`NonBlockingMutex` is faster than `std::sync::Mutex`(both blocking and spinning)
when contention is high enough

Notice that `NonBlockingMutex` doesn't guarantee order
of execution, only atomicity of operations is guaranteed