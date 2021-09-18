// SPDX-License-Identifier: GPL-2.0

//! Kernel types.
//!
//! C header: [`include/linux/types.h`](../../../../include/linux/types.h)

use crate::{
    bindings, c_types,
    sync::{Ref, RefBorrow},
};
use alloc::{boxed::Box, sync::Arc};
use core::{ops::Deref, pin::Pin, ptr::NonNull};

pub type UserNamespace = bindings::user_namespace;
pub type Iattr = bindings::iattr;
pub type Path = bindings::path;
pub type Kstat = bindings::kstat;
pub type Dev = bindings::dev_t;
pub type Page = bindings::page;
pub type AddressSpace = bindings::address_space;

macro_rules! impl_flag_methods {
    ($T:ty, $V:ty) => {
        impl $T {
            pub const fn from_int(val: $V) -> Self {
                Self(val)
            }
            pub const fn into_int(self) -> $V {
                self.0
            }
            pub const fn is_empty(&self) -> bool {
                self.0 == 0
            }
            pub const fn has(self, other: Self) -> bool {
                self.0 & other.0 != 0
            }
            pub const fn with(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }
            pub const fn without(self, other: Self) -> Self {
                Self(self.0 & !other.0)
            }
        }
    };
}

pub struct FileSystemFlags(c_types::c_int);

#[rustfmt::skip]
impl FileSystemFlags {
    /// Not a virtual file system. An actual underlying block device is required.
    pub const FS_REQUIRES_DEV: Self         = Self::from_int(bindings::FS_REQUIRES_DEV as _);

    /// Mount data is binary, and cannot be handled by the standard option parser
    pub const FS_BINARY_MOUNTDATA: Self     = Self::from_int(bindings::FS_BINARY_MOUNTDATA as _);

    /// Has subtype
    pub const FS_HAS_SUBTYPE: Self          = Self::from_int(bindings::FS_HAS_SUBTYPE as _);

    /// Can be mounted by userns root
    pub const FS_USERNS_MOUNT: Self         = Self::from_int(bindings::FS_USERNS_MOUNT as _);

    /// Disable fanotify permission events
    pub const FS_DISALLOW_NOTIFY_PERM: Self = Self::from_int(bindings::FS_DISALLOW_NOTIFY_PERM as _);

    /// FS has been updated to handle vfs idmappings
    pub const FS_ALLOW_IDMAP: Self          = Self::from_int(bindings::FS_ALLOW_IDMAP as _);

    /// Remove once all fs converted
    pub const FS_THP_SUPPORT: Self          = Self::from_int(bindings::FS_THP_SUPPORT as _);

    /// FS will handle d_move() during rename() internally
    pub const FS_RENAME_DOES_D_MOVE: Self   = Self::from_int(bindings::FS_RENAME_DOES_D_MOVE as _);
}

impl_flag_methods!(FileSystemFlags, c_types::c_int);

/// Permissions.
///
/// C header: [`include/uapi/linux/stat.h`](../../../../include/uapi/linux/stat.h)
///
/// C header: [`include/linux/stat.h`](../../../../include/linux/stat.h)
pub struct Mode(bindings::umode_t);

impl Mode {
    /// Creates a [`Mode`] from an integer.
    pub fn from_int(m: u16) -> Mode {
        Mode(m)
    }

    /// Returns the mode as an integer.
    pub fn as_int(&self) -> u16 {
        self.0
    }
}

/// Used to convert an object into a raw pointer that represents it.
///
/// It can eventually be converted back into the object. This is used to store objects as pointers
/// in kernel data structures, for example, an implementation of [`FileOperations`] in `struct
/// file::private_data`.
pub trait PointerWrapper {
    /// Type of values borrowed between calls to [`PointerWrapper::into_pointer`] and
    /// [`PointerWrapper::from_pointer`].
    type Borrowed: Deref;

    /// Returns the raw pointer.
    fn into_pointer(self) -> *const c_types::c_void;

    /// Returns a borrowed value.
    ///
    /// # Safety
    ///
    /// `ptr` must have been returned by a previous call to [`PointerWrapper::into_pointer`].
    /// Additionally, [`PointerWrapper::from_pointer`] can only be called after *all* values
    /// returned by [`PointerWrapper::borrow`] have been dropped.
    unsafe fn borrow(ptr: *const c_types::c_void) -> Self::Borrowed;

    /// Returns the instance back from the raw pointer.
    ///
    /// # Safety
    ///
    /// The passed pointer must come from a previous call to [`PointerWrapper::into_pointer()`].
    unsafe fn from_pointer(ptr: *const c_types::c_void) -> Self;
}

impl<T> PointerWrapper for Box<T> {
    type Borrowed = UnsafeReference<T>;

    fn into_pointer(self) -> *const c_types::c_void {
        Box::into_raw(self) as _
    }

    unsafe fn borrow(ptr: *const c_types::c_void) -> Self::Borrowed {
        // SAFETY: The safety requirements for this function ensure that the object is still alive,
        // so it is safe to dereference the raw pointer.
        // The safety requirements also ensure that the object remains alive for the lifetime of
        // the returned value.
        unsafe { UnsafeReference::new(&*ptr.cast()) }
    }

    unsafe fn from_pointer(ptr: *const c_types::c_void) -> Self {
        // SAFETY: The passed pointer comes from a previous call to [`Self::into_pointer()`].
        unsafe { Box::from_raw(ptr as _) }
    }
}

impl<T> PointerWrapper for Ref<T> {
    type Borrowed = RefBorrow<T>;

    fn into_pointer(self) -> *const c_types::c_void {
        Ref::into_usize(self) as _
    }

    unsafe fn borrow(ptr: *const c_types::c_void) -> Self::Borrowed {
        // SAFETY: The safety requirements for this function ensure that the underlying object
        // remains valid for the lifetime of the returned value.
        unsafe { Ref::borrow_usize(ptr as _) }
    }

    unsafe fn from_pointer(ptr: *const c_types::c_void) -> Self {
        // SAFETY: The passed pointer comes from a previous call to [`Self::into_pointer()`].
        unsafe { Ref::from_usize(ptr as _) }
    }
}

impl<T> PointerWrapper for Arc<T> {
    type Borrowed = UnsafeReference<T>;

    fn into_pointer(self) -> *const c_types::c_void {
        Arc::into_raw(self) as _
    }

    unsafe fn borrow(ptr: *const c_types::c_void) -> Self::Borrowed {
        // SAFETY: The safety requirements for this function ensure that the object is still alive,
        // so it is safe to dereference the raw pointer.
        // The safety requirements also ensure that the object remains alive for the lifetime of
        // the returned value.
        unsafe { UnsafeReference::new(&*ptr.cast()) }
    }

    unsafe fn from_pointer(ptr: *const c_types::c_void) -> Self {
        // SAFETY: The passed pointer comes from a previous call to [`Self::into_pointer()`].
        unsafe { Arc::from_raw(ptr as _) }
    }
}

/// A reference with manually-managed lifetime.
///
/// # Invariants
///
/// There are no mutable references to the underlying object, and it remains valid for the lifetime
/// of the [`UnsafeReference`] instance.
pub struct UnsafeReference<T: ?Sized> {
    ptr: NonNull<T>,
}

impl<T: ?Sized> UnsafeReference<T> {
    /// Creates a new [`UnsafeReference`] instance.
    ///
    /// # Safety
    ///
    /// Callers must ensure the following for the lifetime of the returned [`UnsafeReference`]
    /// instance:
    /// 1. That `obj` remains valid;
    /// 2. That no mutable references to `obj` are created.
    unsafe fn new(obj: &T) -> Self {
        // INVARIANT: The safety requirements of this function ensure that the invariants hold.
        Self {
            ptr: NonNull::from(obj),
        }
    }
}

impl<T: ?Sized> Deref for UnsafeReference<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: By the type invariant, the object is still valid and alive, and there are no
        // mutable references to it.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: PointerWrapper + Deref> PointerWrapper for Pin<T> {
    type Borrowed = T::Borrowed;

    fn into_pointer(self) -> *const c_types::c_void {
        // SAFETY: We continue to treat the pointer as pinned by returning just a pointer to it to
        // the caller.
        let inner = unsafe { Pin::into_inner_unchecked(self) };
        inner.into_pointer()
    }

    unsafe fn borrow(ptr: *const c_types::c_void) -> Self::Borrowed {
        // SAFETY: The safety requirements for this function are the same as the ones for
        // `T::borrow`.
        unsafe { T::borrow(ptr) }
    }

    unsafe fn from_pointer(p: *const c_types::c_void) -> Self {
        // SAFETY: The object was originally pinned.
        // The passed pointer comes from a previous call to `inner::into_pointer()`.
        unsafe { Pin::new_unchecked(T::from_pointer(p)) }
    }
}

/// Runs a cleanup function/closure when dropped.
///
/// The [`ScopeGuard::dismiss`] function prevents the cleanup function from running.
///
/// # Examples
///
/// In the example below, we have multiple exit paths and we want to log regardless of which one is
/// taken:
/// ```
/// # use kernel::prelude::*;
/// # use kernel::ScopeGuard;
/// fn example1(arg: bool) {
///     let _log = ScopeGuard::new(|| pr_info!("example1 completed\n"));
///
///     if arg {
///         return;
///     }
///
///     // Do something...
/// }
/// ```
///
/// In the example below, we want to log the same message on all early exits but a different one on
/// the main exit path:
/// ```
/// # use kernel::prelude::*;
/// # use kernel::ScopeGuard;
/// fn example2(arg: bool) {
///     let log = ScopeGuard::new(|| pr_info!("example2 returned early\n"));
///
///     if arg {
///         return;
///     }
///
///     // (Other early returns...)
///
///     log.dismiss();
///     pr_info!("example2 no early return\n");
/// }
/// ```
pub struct ScopeGuard<T: FnOnce()> {
    cleanup_func: Option<T>,
}

impl<T: FnOnce()> ScopeGuard<T> {
    /// Creates a new cleanup object with the given cleanup function.
    pub fn new(cleanup_func: T) -> Self {
        Self {
            cleanup_func: Some(cleanup_func),
        }
    }

    /// Prevents the cleanup function from running.
    pub fn dismiss(mut self) {
        self.cleanup_func.take();
    }
}

impl<T: FnOnce()> Drop for ScopeGuard<T> {
    fn drop(&mut self) {
        // Run the cleanup function if one is still present.
        if let Some(cleanup) = self.cleanup_func.take() {
            cleanup();
        }
    }
}
