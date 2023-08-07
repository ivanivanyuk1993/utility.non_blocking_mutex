use std::cell::UnsafeCell;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// Code was mostly taken from [std::sync::MutexGuard], it is expected to protect [State]
/// from moving out of synchronized block
pub struct MutexGuard<'unsafe_state_ref, State: ?Sized + 'unsafe_state_ref> {
    unsafe_state: &'unsafe_state_ref UnsafeCell<State>,
    /// Adding it to ensure that [MutexGuard] implements [Send] and [Sync] in same cases
    /// as [std::sync::MutexGuard] and protects [State] from going out of synchronized
    /// execution loop
    ///
    /// todo remove when this error is no longer actual
    ///  negative trait bounds are not yet fully implemented; use marker types for now [E0658]
    _phantom_unsend: PhantomData<std::sync::MutexGuard<'unsafe_state_ref, State>>,
}

// todo uncomment when this error is no longer actual
//  negative trait bounds are not yet fully implemented; use marker types for now [E0658]
// impl<'unsafe_state_ref, State: ?Sized> !Send for MutexGuard<'unsafe_state_ref, State> {}
unsafe impl<'unsafe_state_ref, State: ?Sized + Sync> Sync for MutexGuard<'unsafe_state_ref, State> {}

impl<'unsafe_state_ref, State: ?Sized> MutexGuard<'unsafe_state_ref, State> {
    #[inline]
    pub unsafe fn new(unsafe_state: &'unsafe_state_ref UnsafeCell<State>) -> Self {
        Self {
            unsafe_state,
            _phantom_unsend: PhantomData,
        }
    }
}

impl<'unsafe_state_ref, State: ?Sized> Deref for MutexGuard<'unsafe_state_ref, State> {
    type Target = State;

    #[inline]
    fn deref(&self) -> &State {
        unsafe { &*self.unsafe_state.get() }
    }
}

impl<'unsafe_state_ref, State: ?Sized> DerefMut for MutexGuard<'unsafe_state_ref, State> {
    #[inline]
    fn deref_mut(&mut self) -> &mut State {
        unsafe { &mut *self.unsafe_state.get() }
    }
}

impl<'unsafe_state_ref, State: ?Sized + Debug> Debug for MutexGuard<'unsafe_state_ref, State> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&**self, f)
    }
}

impl<'unsafe_state_ref, State: ?Sized + Display> Display for MutexGuard<'unsafe_state_ref, State> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}
