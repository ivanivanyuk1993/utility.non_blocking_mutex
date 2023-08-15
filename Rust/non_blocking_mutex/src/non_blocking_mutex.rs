use crate::mutex_guard::MutexGuard;
use crate::non_blocking_mutex_task::NonBlockingMutexTask;
use crossbeam_utils::CachePadded;
use sharded_queue::ShardedQueue;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct NonBlockingMutex<State: ?Sized, TNonBlockingMutexTask: NonBlockingMutexTask<State>> {
    pub(crate) task_count: CachePadded<AtomicUsize>,
    pub(crate) task_queue: ShardedQueue<TNonBlockingMutexTask>,
    pub(crate) unsafe_state: UnsafeCell<State>,
}

/// # [NonBlockingMutex]
///
/// ## Why you should use [NonBlockingMutex]
///
/// [NonBlockingMutex] is currently the fastest way to do
/// expensive calculations under lock, or do cheap calculations
/// under lock when concurrency/load/contention is very high -
/// see benchmarks in directory `benches` and run them with
/// ```bash
/// cargo bench
/// ```
///
/// ## Example
/// ```rust
/// use std::thread::{available_parallelism};
/// use non_blocking_mutex::mutex_guard::MutexGuard;
/// use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
///
/// /// How many threads can physically access [NonBlockingMutex]
/// /// simultaneously, needed for computing `shard_count` of [ShardedQueue],
/// /// used to store queue of tasks
/// let max_concurrent_thread_count = available_parallelism().unwrap().get();
///
/// let non_blocking_mutex = NonBlockingMutex::new(max_concurrent_thread_count, 0);
/// non_blocking_mutex.run_if_first_or_schedule_on_first(|mut state: MutexGuard<usize>| {
///     *state += 1;
/// });
/// ```
///
/// ## Why you may want to not use [NonBlockingMutex]
///
/// - [NonBlockingMutex] forces first thread to enter synchronized block to
/// do all tasks(including added while it is running,
/// potentially running forever if tasks are being added forever)
///
/// - It is more difficult to continue execution on same thread after
/// synchronized logic is run, you need to schedule continuation on some
/// scheduler when you want to continue after end of synchronized logic
/// in new thread or introduce other synchronization primitives,
/// like channels, or `WaitGroup`-s, or similar
///
/// - [NonBlockingMutex] performs worse than [std::sync::Mutex] when
/// concurrency/load/contention is low
///
/// - Similar to [std::sync::Mutex], [NonBlockingMutex] doesn't guarantee
/// order of execution, only atomicity of operations is guaranteed
///
/// ## Design explanation
///
/// First thread, which calls [NonBlockingMutex::run_if_first_or_schedule_on_first],
/// atomically increments `task_count`, and,
/// if thread was first to increment `task_count` from 0 to 1,
/// first thread immediately executes first task,
/// and then atomically decrements `task_count` and checks if `task_count`
/// changed from 1 to 0. If `task_count` changed from 1 to 0 -
/// there are no more tasks and first thread can finish execution loop,
/// otherwise first thread gets next task from `task_queue` and runs task,
/// then decrements tasks count after it was run and repeats check if
/// `task_count` changed from 1 to 0 and running tasks until there are no more tasks left.
///
/// Not first threads also atomically increment `task_count`,
/// do check if they are first, [Box] task and push task [Box] to `task_queue`
///
/// This design allows us to avoid lock contention, but adds ~constant time
/// of [Box]-ing task and putting task [Box] into concurrent `task_queue`, and
/// incrementing and decrementing `task_count`, so when lock contention is low,
/// [NonBlockingMutex] performs worse than [std::sync::Mutex],
/// but when contention is high
/// (because we have more CPU-s or because we want to do expensive
/// calculations under lock), [NonBlockingMutex] performs better
/// than [std::sync::Mutex]
impl<State, TNonBlockingMutexTask: NonBlockingMutexTask<State>>
    NonBlockingMutex<State, TNonBlockingMutexTask>
{
    /// # Arguments
    ///
    /// * `max_concurrent_thread_count` - how many threads can physically access
    /// [NonBlockingMutex] simultaneously, needed for computing `shard_count` of [ShardedQueue],
    /// used to store queue of tasks
    pub fn new(max_concurrent_thread_count: usize, state: State) -> Self {
        Self {
            task_count: CachePadded::new(AtomicUsize::new(0)),
            task_queue: ShardedQueue::new(max_concurrent_thread_count),
            unsafe_state: UnsafeCell::new(state),
        }
    }

    /// Please don't forget that order of execution is not guaranteed. Atomicity of operations is guaranteed,
    /// but order can be random
    pub fn run_if_first_or_schedule_on_first(&self, task: TNonBlockingMutexTask) {
        if self.task_count.fetch_add(1, Ordering::Acquire) != 0 {
            self.task_queue.push_back(task);
        } else {
            // If we acquired first lock, run should be executed immediately and run loop started
            task.run_with_state(unsafe { MutexGuard::new(&self.unsafe_state) });
            /// Note that if [`fetch_sub`] != 1
            /// => some thread entered first if block in method
            /// => [ShardedQueue::push_back] is guaranteed to be called
            /// => [ShardedQueue::pop_front_or_spin_wait_item] will not deadlock while spins until it gets item
            ///
            /// Notice that we run action first, and only then decrement count
            /// with releasing(pushing) memory changes, even if it looks otherwise
            while self.task_count.fetch_sub(1, Ordering::Release) != 1 {
                self.task_queue
                    .pop_front_or_spin_wait_item()
                    .run_with_state(unsafe { MutexGuard::new(&self.unsafe_state) });
            }
        }
    }
}

/// [Send] and [Sync] logic was taken from [std::sync::Mutex]
unsafe impl<State: ?Sized + Send, TNonBlockingMutexTask: NonBlockingMutexTask<State>> Send
    for NonBlockingMutex<State, TNonBlockingMutexTask>
{
}
unsafe impl<State: ?Sized + Send, TNonBlockingMutexTask: NonBlockingMutexTask<State>> Sync
    for NonBlockingMutex<State, TNonBlockingMutexTask>
{
}

/// [NonBlockingMutex] should implement [Send] and [Sync] in same
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
/// use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
/// fn is_send<T: Send>(t: T) {}
///
/// let send_sync = 0;
/// let non_blocking_mutex = NonBlockingMutex::new(1, send_sync);
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
/// use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// let send_sync = 0;
/// let non_blocking_mutex = NonBlockingMutex::new(1, send_sync);
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
/// use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
///
/// fn is_send<T: Send>(t: T) {}
///
/// let send_not_sync = Cell::new(0);
/// let non_blocking_mutex = NonBlockingMutex::new(1, send_not_sync);
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
/// use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// let send_not_sync = Cell::new(0);
/// let non_blocking_mutex = NonBlockingMutex::new(1, send_not_sync);
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
/// use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
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
/// let non_blocking_mutex = NonBlockingMutex::new(1, not_send_sync);
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
/// use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
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
/// let non_blocking_mutex = NonBlockingMutex::new(1, not_send_sync);
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
/// use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
///
/// fn is_send<T: Send>(t: T) {}
///
/// let t = 0;
/// let not_send_not_sync = &t as *const usize;
/// let non_blocking_mutex = NonBlockingMutex::new(1, not_send_not_sync);
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
/// use non_blocking_mutex::non_blocking_mutex::NonBlockingMutex;
///
/// fn is_sync<T: Sync>(t: T) {}
///
/// let t = 0;
/// let not_send_not_sync = &t as *const usize;
/// let non_blocking_mutex = NonBlockingMutex::new(1, not_send_not_sync);
/// non_blocking_mutex.run_if_first_or_schedule_on_first(|_: MutexGuard<_>| {});
///
/// is_sync(non_blocking_mutex);
/// ```
struct NonBlockingMutexSendSyncImplementationDocTests {}
