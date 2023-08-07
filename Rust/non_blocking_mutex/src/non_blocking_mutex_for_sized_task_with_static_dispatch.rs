use crate::sized_task_with_static_dispatch::SizedTaskWithStaticDispatch;
use crate::MutexGuard;
use sharded_queue::ShardedQueue;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct NonBlockingMutexForSizedTaskWithStaticDispatch<
    State: ?Sized,
    TSizedTaskWithStaticDispatch,
> where
    for<'unsafe_state_ref> TSizedTaskWithStaticDispatch:
        SizedTaskWithStaticDispatch<'unsafe_state_ref, State>,
{
    task_count: AtomicUsize,
    task_queue: ShardedQueue<TSizedTaskWithStaticDispatch>,
    unsafe_state: UnsafeCell<State>,
}

/// # [NonBlockingMutexForSizedTaskWithStaticDispatch]
///
/// Unlike [crate::NonBlockingMutex], [NonBlockingMutexForSizedTaskWithStaticDispatch]
/// allows to not use dynamic dispatch or [Box]-es tasks
///
/// ## Example
/// ```rust
/// use non_blocking_mutex::mutex_guard::MutexGuard;
/// use non_blocking_mutex::non_blocking_mutex_for_sized_task_with_static_dispatch::NonBlockingMutexForSizedTaskWithStaticDispatch;
/// use std::thread::{available_parallelism};
///
/// let max_concurrent_thread_count = available_parallelism().unwrap().get();
///
/// let non_blocking_mutex = NonBlockingMutexForSizedTaskWithStaticDispatch::new(max_concurrent_thread_count, 0);
/// non_blocking_mutex.run_if_first_or_schedule_on_first(|mut state: MutexGuard<_>| {
///     *state += 1;
/// });
/// ```
impl<State, TSizedTaskWithStaticDispatch>
    NonBlockingMutexForSizedTaskWithStaticDispatch<State, TSizedTaskWithStaticDispatch>
where
    for<'unsafe_state_ref> TSizedTaskWithStaticDispatch:
        SizedTaskWithStaticDispatch<'unsafe_state_ref, State>,
{
    #[inline]
    pub fn new(max_concurrent_thread_count: usize, state: State) -> Self {
        Self {
            task_count: AtomicUsize::new(0),
            task_queue: ShardedQueue::new(max_concurrent_thread_count),
            unsafe_state: UnsafeCell::new(state),
        }
    }

    /// Please don't forget that order of execution is not guaranteed. Atomicity of operations is guaranteed,
    /// but order can be random
    #[inline]
    pub fn run_if_first_or_schedule_on_first(&self, task: TSizedTaskWithStaticDispatch) {
        if self.task_count.fetch_add(1, Ordering::Acquire) != 0 {
            self.task_queue.push_back(task);
        } else {
            // If we acquired first lock, run should be executed immediately and run loop started
            task.run_with_state(unsafe { MutexGuard::new(&self.unsafe_state) });
            /// Note that if [`fetch_sub`] != 1
            /// => some thread entered first if block in method
            /// => [ShardedQueue::push_back] is guaranteed to be called
            /// => [ShardedQueue::pop_front_or_spin] will not deadlock while spins until it gets item
            ///
            /// Notice that we run action first, and only then decrement count
            /// with releasing(pushing) memory changes, even if it looks otherwise
            while self.task_count.fetch_sub(1, Ordering::Release) != 1 {
                self.task_queue
                    .pop_front_or_spin()
                    .run_with_state(unsafe { MutexGuard::new(&self.unsafe_state) });
            }
        }
    }
}

/// [Send], [Sync], and [MutexGuard] logic was taken from [std::sync::Mutex]
/// and [std::sync::MutexGuard]
///
/// these are the only places where `T: Send` matters; all other
/// functionality works fine on a single thread.
unsafe impl<State: ?Sized + Send, TSizedTaskWithStaticDispatch> Send
    for NonBlockingMutexForSizedTaskWithStaticDispatch<State, TSizedTaskWithStaticDispatch>
where
    for<'unsafe_state_ref> TSizedTaskWithStaticDispatch:
        SizedTaskWithStaticDispatch<'unsafe_state_ref, State>,
{
}
unsafe impl<State: ?Sized + Send, TSizedTaskWithStaticDispatch> Sync
    for NonBlockingMutexForSizedTaskWithStaticDispatch<State, TSizedTaskWithStaticDispatch>
where
    for<'unsafe_state_ref> TSizedTaskWithStaticDispatch:
        SizedTaskWithStaticDispatch<'unsafe_state_ref, State>,
{
}

impl<
        'unsafe_state_ref,
        State: ?Sized + 'unsafe_state_ref,
        TFnOnce: FnOnce(MutexGuard<'unsafe_state_ref, State>) + Send + Sized,
    > SizedTaskWithStaticDispatch<'unsafe_state_ref, State> for TFnOnce
{
    fn run_with_state(self, state: MutexGuard<'unsafe_state_ref, State>) {
        self(state);
    }
}

/// [NonBlockingMutexForSizedTaskWithStaticDispatch] should implement [Send] and [Sync] in same
/// cases as [std::sync::Mutex]
///
/// ```rust
/// fn is_send<T: Send>(t: T) {}
/// fn is_sync<T: Sync>(t: T) {}
///
/// let send_sync = 0;
///
/// is_send(send_sync);
/// is_sync(send_sync);
/// ```
/// ```rust
/// use non_blocking_mutex::mutex_guard::MutexGuard;
/// use non_blocking_mutex::non_blocking_mutex_for_sized_task_with_static_dispatch::NonBlockingMutexForSizedTaskWithStaticDispatch;
///
/// fn is_send<T: Send>(t: T) {}
///
/// let send_sync = 0;
/// let non_blocking_mutex = NonBlockingMutexForSizedTaskWithStaticDispatch::new(1, send_sync);
/// non_blocking_mutex.run_if_first_or_schedule_on_first(|_: MutexGuard<_>| {});
///
/// is_send(non_blocking_mutex);
/// ```
/// ```rust
/// use std::sync::Mutex;
///
/// fn is_send<T: Send>(t: T) {}
///
/// let send_sync = 0;
/// let mutex = Mutex::new(send_sync);
///
/// is_send(mutex);
/// ```
/// ```rust
/// use non_blocking_mutex::mutex_guard::MutexGuard;
/// use non_blocking_mutex::non_blocking_mutex_for_sized_task_with_static_dispatch::NonBlockingMutexForSizedTaskWithStaticDispatch;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// let send_sync = 0;
/// let non_blocking_mutex = NonBlockingMutexForSizedTaskWithStaticDispatch::new(1, send_sync);
/// non_blocking_mutex.run_if_first_or_schedule_on_first(|_: MutexGuard<_>| {});
///
/// is_sync(non_blocking_mutex);
/// ```
/// ```rust
/// use std::sync::Mutex;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// let send_sync = 0;
/// let mutex = Mutex::new(send_sync);
///
/// is_sync(mutex);
/// ```
///
///
///
///
///
/// ```rust
/// use std::cell::Cell;
///
/// fn is_send<T: Send>(t: T) {}
///
/// let send_not_sync = Cell::new(0);
///
/// is_send(send_not_sync);
/// ```
/// ```compile_fail
/// use std::cell::Cell;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// let send_not_sync = Cell::new(0);
///
/// is_sync(send_not_sync);
/// ```
/// ```rust
/// use std::cell::Cell;
/// use non_blocking_mutex::mutex_guard::MutexGuard;
/// use non_blocking_mutex::non_blocking_mutex_for_sized_task_with_static_dispatch::NonBlockingMutexForSizedTaskWithStaticDispatch;
///
/// fn is_send<T: Send>(t: T) {}
///
/// let send_not_sync = Cell::new(0);
/// let non_blocking_mutex = NonBlockingMutexForSizedTaskWithStaticDispatch::new(1, send_not_sync);
/// non_blocking_mutex.run_if_first_or_schedule_on_first(|_: MutexGuard<_>| {});
///
/// is_send(non_blocking_mutex);
/// ```
/// ```rust
/// use std::cell::Cell;
/// use std::sync::Mutex;
///
/// fn is_send<T: Send>(t: T) {}
///
/// let send_not_sync = Cell::new(0);
/// let mutex = Mutex::new(send_not_sync);
///
/// is_send(mutex);
/// ```
/// ```rust
/// use std::cell::Cell;
/// use non_blocking_mutex::mutex_guard::MutexGuard;
/// use non_blocking_mutex::non_blocking_mutex_for_sized_task_with_static_dispatch::NonBlockingMutexForSizedTaskWithStaticDispatch;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// let send_not_sync = Cell::new(0);
/// let non_blocking_mutex = NonBlockingMutexForSizedTaskWithStaticDispatch::new(1, send_not_sync);
/// non_blocking_mutex.run_if_first_or_schedule_on_first(|_: MutexGuard<_>| {});
///
/// is_sync(non_blocking_mutex);
/// ```
/// ```rust
/// use std::cell::Cell;
/// use std::sync::Mutex;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// let send_not_sync = Cell::new(0);
/// let mutex = Mutex::new(send_not_sync);
///
/// is_sync(mutex);
/// ```
///
///
///
///
///
/// ```compile_fail
/// use std::marker::PhantomData;
///
/// fn is_send<T: Send>(t: T) {}
///
/// struct NotSendSync<'t, T> {
///     _phantom_unsend: PhantomData<std::sync::MutexGuard<'t, T>>
/// }
/// unsafe impl<'t, T> Sync for NotSendSync<'t, T> {}
///
/// let not_send_sync = NotSendSync::<usize> {
///     _phantom_unsend: PhantomData,
/// };
///
/// is_send(not_send_sync);
/// ```
/// ```rust
/// use std::marker::PhantomData;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// struct NotSendSync<'t, T> {
///     _phantom_unsend: PhantomData<std::sync::MutexGuard<'t, T>>
/// }
/// unsafe impl<'t, T> Sync for NotSendSync<'t, T> {}
///
/// let not_send_sync = NotSendSync::<usize> {
///     _phantom_unsend: PhantomData,
/// };
///
/// is_sync(not_send_sync);
/// ```
/// ```compile_fail
/// use std::marker::PhantomData;
/// use non_blocking_mutex::mutex_guard::MutexGuard;
/// use non_blocking_mutex::non_blocking_mutex_for_sized_task_with_static_dispatch::NonBlockingMutexForSizedTaskWithStaticDispatch;
///
/// fn is_send<T: Send>(t: T) {}
///
/// struct NotSendSync<'t, T> {
///     _phantom_unsend: PhantomData<std::sync::MutexGuard<'t, T>>
/// }
/// unsafe impl<'t, T> Sync for NotSendSync<'t, T> {}
///
/// let not_send_sync = NotSendSync::<usize> {
///     _phantom_unsend: PhantomData,
/// };
/// let non_blocking_mutex = NonBlockingMutexForSizedTaskWithStaticDispatch::new(1, not_send_sync);
/// non_blocking_mutex.run_if_first_or_schedule_on_first(|_: MutexGuard<_>| {});
///
/// is_send(non_blocking_mutex);
/// ```
/// ```compile_fail
/// use std::marker::PhantomData;
/// use std::sync::Mutex;
///
/// fn is_send<T: Send>(t: T) {}
///
/// struct NotSendSync<'t, T> {
///     _phantom_unsend: PhantomData<std::sync::MutexGuard<'t, T>>
/// }
/// unsafe impl<'t, T> Sync for NotSendSync<'t, T> {}
///
/// let not_send_sync = NotSendSync::<usize> {
///     _phantom_unsend: PhantomData,
/// };
/// let mutex = Mutex::new(not_send_sync);
///
/// is_send(mutex);
/// ```
/// ```compile_fail
/// use std::marker::PhantomData;
/// use non_blocking_mutex::mutex_guard::MutexGuard;
/// use non_blocking_mutex::non_blocking_mutex_for_sized_task_with_static_dispatch::NonBlockingMutexForSizedTaskWithStaticDispatch;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// struct NotSendSync<'t, T> {
///     _phantom_unsend: PhantomData<std::sync::MutexGuard<'t, T>>
/// }
/// unsafe impl<'t, T> Sync for NotSendSync<'t, T> {}
///
/// let not_send_sync = NotSendSync::<usize> {
///     _phantom_unsend: PhantomData,
/// };
/// let non_blocking_mutex = NonBlockingMutexForSizedTaskWithStaticDispatch::new(1, not_send_sync);
/// non_blocking_mutex.run_if_first_or_schedule_on_first(|_: MutexGuard<_>| {});
///
/// is_sync(non_blocking_mutex);
/// ```
/// ```compile_fail
/// use std::marker::PhantomData;
/// use std::sync::Mutex;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// struct NotSendSync<'t, T> {
///     _phantom_unsend: PhantomData<std::sync::MutexGuard<'t, T>>
/// }
/// unsafe impl<'t, T> Sync for NotSendSync<'t, T> {}
///
/// let not_send_sync = NotSendSync::<usize> {
///     _phantom_unsend: PhantomData,
/// };
/// let mutex = Mutex::new(not_send_sync);
///
/// is_sync(mutex);
/// ```
///
///
///
///
///
/// ```compile_fail
/// fn is_send<T: Send>(t: T) {}
///
/// let t = 0;
/// let not_send_not_sync = &t as *const usize;
///
/// is_send(not_send_not_sync);
/// ```
/// ```compile_fail
/// fn is_sync<T: Sync>(t: T) {}
///
/// let t = 0;
/// let not_send_not_sync = &t as *const usize;
///
/// is_sync(not_send_not_sync);
/// ```
/// ```compile_fail
/// use non_blocking_mutex::mutex_guard::MutexGuard;
/// use non_blocking_mutex::non_blocking_mutex_for_sized_task_with_static_dispatch::NonBlockingMutexForSizedTaskWithStaticDispatch;
///
/// fn is_send<T: Send>(t: T) {}
///
/// let t = 0;
/// let not_send_not_sync = &t as *const usize;
/// let non_blocking_mutex = NonBlockingMutexForSizedTaskWithStaticDispatch::new(1, not_send_not_sync);
/// non_blocking_mutex.run_if_first_or_schedule_on_first(|_: MutexGuard<_>| {});
///
/// is_send(non_blocking_mutex);
/// ```
/// ```compile_fail
/// use std::sync::Mutex;
///
/// fn is_send<T: Send>(t: T) {}
///
/// let t = 0;
/// let not_send_not_sync = &t as *const usize;
/// let mutex = Mutex::new(not_send_not_sync);
///
/// is_send(mutex);
/// ```
/// ```compile_fail
/// use std::sync::Mutex;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// let t = 0;
/// let not_send_not_sync = &t as *const usize;
/// let mutex = Mutex::new(not_send_not_sync);
///
/// is_sync(mutex);
/// ```
/// ```compile_fail
/// use std::sync::Mutex;
/// use non_blocking_mutex::mutex_guard::MutexGuard;
/// use non_blocking_mutex::non_blocking_mutex_for_sized_task_with_static_dispatch::NonBlockingMutexForSizedTaskWithStaticDispatch;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// let t = 0;
/// let not_send_not_sync = &t as *const usize;
/// let non_blocking_mutex = NonBlockingMutexForSizedTaskWithStaticDispatch::new(1, not_send_not_sync);
/// non_blocking_mutex.run_if_first_or_schedule_on_first(|_: MutexGuard<_>| {});
///
/// is_sync(non_blocking_mutex);
/// ```
struct NonBlockingMutexForSizedTaskWithStaticDispatchSendSyncImplementationDocTests {}
