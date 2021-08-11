use core::{
    mem,
    ops::{Deref, DerefMut},
};

/// This trait gives you implementations of
///     - `AsRef`
///     - `AsMut`
///     - `Deref`
///     - `DerefMut`
///
/// # SAFETY
///
/// This trait should only be implemented on tuple structs with `repr(transparent)`. The methods
/// `inner` and `inner_mut` should both return the contained field.
pub(crate) unsafe trait ObjectWrapper {
    type Wrapped;

    fn inner(&self) -> &Self::Wrapped;
    fn inner_mut(&mut self) -> &mut Self::Wrapped;
    fn as_ptr(&self) -> *const Self::Wrapped {
        self.inner() as *const _
    }
    fn as_ptr_mut(&mut self) -> *mut Self::Wrapped {
        self.inner_mut() as *mut _
    }
}

impl<T: ObjectWrapper> Deref for T {
    type Target = T::Wrapped;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}
impl<T: ObjectWrapper> DerefMut for T {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner_mut()
    }
}
impl<T: ObjectWrapper> AsRef<T::Wrapped> for T {
    fn as_ref(&self) -> &T::Wrapped {
        unsafe { mem::transmute(self) }
    }
}
impl<T: ObjectWrapper> AsMut<T::Wrapped> for T {
    fn as_mut(&mut self) -> &mut T::Wrapped {
        unsafe { mem::transmute(self) }
    }
}
