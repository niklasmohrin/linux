// SPDX-License-Identifier: GPL-2.0

//! Kernel errors.
//!
//! C header: [`include/uapi/asm-generic/errno-base.h`](../../../include/uapi/asm-generic/errno-base.h)

use crate::{bindings, c_types, declare_constant_from_bindings, macros::DO_NEGATE, str::CStr};
use alloc::{alloc::AllocError, collections::TryReserveError};
use core::convert::From;
use core::fmt;
use core::num::TryFromIntError;
use core::str::{self, Utf8Error};

/// Generic integer kernel error.
///
/// The kernel defines a set of integer generic error codes based on C and
/// POSIX ones. These codes may have a more specific meaning in some contexts.
///
/// # Invariants
///
/// The value is a valid `errno` (i.e. `>= -MAX_ERRNO && < 0`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Error(c_types::c_int);

impl Error {
    /// Creates an [`Error`] from a kernel error code.
    ///
    /// It is a bug to pass an out-of-range `errno`. `EINVAL` would
    /// be returned in such a case.
    pub(crate) fn from_kernel_errno(errno: c_types::c_int) -> Error {
        if errno < -(bindings::MAX_ERRNO as i32) || errno >= 0 {
            // TODO: make it a `WARN_ONCE` once available.
            crate::pr_warn!(
                "attempted to create `Error` with out of range `errno`: {}",
                errno
            );
            return Error::EINVAL;
        }

        // INVARIANT: the check above ensures the type invariant
        // will hold.
        Error(errno)
    }

    /// Creates an [`Error`] from a kernel error code.
    ///
    /// # Safety
    ///
    /// `errno` must be within error code range (i.e. `>= -MAX_ERRNO && < 0`).
    pub(crate) unsafe fn from_kernel_errno_unchecked(errno: c_types::c_int) -> Error {
        // INVARIANT: the contract ensures the type invariant
        // will hold.
        Error(errno)
    }

    /// Returns the kernel error code.
    pub fn to_kernel_errno(self) -> c_types::c_int {
        self.0
    }

    pub fn as_err_ptr<T>(&self) -> *mut T {
        self.0 as *mut _
    }
}

macro_rules! declare_error {
    ($name:ident, $doc:expr) => {
        declare_constant_from_bindings!($name, $doc, i32, DO_NEGATE);
    };
}

#[macro_export]
macro_rules! ret_err_ptr {
    ($ex:expr) => {
        match $ex {
            Ok(val) => val,
            Err(err) => return err.as_err_ptr(),
        }
    };
}

#[rustfmt::skip]
impl Error {
    // See `man 3 errno`.
    declare_error!(E2BIG,            "Argument list too long (POSIX.1-2001).");
    declare_error!(EACCES,           "Permission denied (POSIX.1-2001).");
    declare_error!(EADDRINUSE,       "Address already in use (POSIX.1-2001).");
    declare_error!(EADDRNOTAVAIL,    "Address not available (POSIX.1-2001).");
    declare_error!(EAFNOSUPPORT,     "Address family not supported (POSIX.1-2001).");
    declare_error!(EAGAIN,           "Resource temporarily unavailable  (may  be  the  same  value  as  EWOULDBLOCK) (POSIX.1-2001).");
    declare_error!(EALREADY,         "Connection already in progress (POSIX.1-2001).");
    declare_error!(EBADE,            "Invalid exchange.");
    declare_error!(EBADF,            "Bad file descriptor (POSIX.1-2001).");
    declare_error!(EBADFD,           "File descriptor in bad state.");
    declare_error!(EBADMSG,          "Bad message (POSIX.1-2001).");
    declare_error!(EBADR,            "Invalid request descriptor.");
    declare_error!(EBADRQC,          "Invalid request code.");
    declare_error!(EBADSLT,          "Invalid slot.");
    declare_error!(EBUSY,            "Device or resource busy (POSIX.1-2001).");
    declare_error!(ECANCELED,        "Operation canceled (POSIX.1-2001).");
    declare_error!(ECHILD,           "No child processes (POSIX.1-2001).");
    declare_error!(ECHRNG,           "Channel number out of range.");
    declare_error!(ECOMM,            "Communication error on send.");
    declare_error!(ECONNABORTED,     "Connection aborted (POSIX.1-2001).");
    declare_error!(ECONNREFUSED,     "Connection refused (POSIX.1-2001).");
    declare_error!(ECONNRESET,       "Connection reset (POSIX.1-2001).");
    declare_error!(EDEADLK,          "Resource deadlock avoided (POSIX.1-2001).");
    declare_error!(EDEADLOCK,        "On  most  architectures,  a synonym for EDEADLK.  On some architectures (e.g., Linux MIPS, PowerPC, SPARC), it is a separate error code \"File  locking  dead-lock error\".");
    declare_error!(EDESTADDRREQ,     "Destination address required (POSIX.1-2001).");
    declare_error!(EDOM,             "Mathematics argument out of domain of function (POSIX.1, C99).");
    declare_error!(EDQUOT,           "Disk quota exceeded (POSIX.1-2001).");
    declare_error!(EEXIST,           "File exists (POSIX.1-2001).");
    declare_error!(EFAULT,           "Bad address (POSIX.1-2001).");
    declare_error!(EFBIG,            "File too large (POSIX.1-2001).");
    declare_error!(EHOSTDOWN,        "Host is down.");
    declare_error!(EHOSTUNREACH,     "Host is unreachable (POSIX.1-2001).");
    declare_error!(EHWPOISON,        "Memory page has hardware error.");
    declare_error!(EIDRM,            "Identifier removed (POSIX.1-2001).");
    declare_error!(EILSEQ,           "Invalid or incomplete multibyte or wide character (POSIX.1, C99).");
    declare_error!(EINPROGRESS,      "Operation in progress (POSIX.1-2001).");
    declare_error!(EINTR,            "Interrupted function call (POSIX.1-2001); see signal(7).");
    declare_error!(EINVAL,           "Invalid argument (POSIX.1-2001).");
    declare_error!(EIO,              "Input/output error (POSIX.1-2001).");
    declare_error!(EISCONN,          "Socket is connected (POSIX.1-2001).");
    declare_error!(EISDIR,           "Is a directory (POSIX.1-2001).");
    declare_error!(EISNAM,           "Is a named type file.");
    declare_error!(EKEYEXPIRED,      "Key has expired.");
    declare_error!(EKEYREJECTED,     "Key was rejected by service.");
    declare_error!(EKEYREVOKED,      "Key has been revoked.");
    declare_error!(EL2HLT,           "Level 2 halted.");
    declare_error!(EL2NSYNC,         "Level 2 not synchronized.");
    declare_error!(EL3HLT,           "Level 3 halted.");
    declare_error!(EL3RST,           "Level 3 reset.");
    declare_error!(ELIBACC,          "Cannot access a needed shared library.");
    declare_error!(ELIBBAD,          "Accessing a corrupted shared library.");
    declare_error!(ELIBMAX,          "Attempting to link in too many shared libraries.");
    declare_error!(ELIBSCN,          ".lib section in a.out corrupted");
    declare_error!(ELIBEXEC,         "Cannot exec a shared library directly.");
    declare_error!(ELNRNG,           "Link number out of range.");
    declare_error!(ELOOP,            "Too many levels of symbolic links (POSIX.1-2001).");
    declare_error!(EMEDIUMTYPE,      "Wrong medium type.");
    declare_error!(EMFILE,           "Too  many  open  files  (POSIX.1-2001).   Commonly  caused  by  exceeding  the RLIMIT_NOFILE resource limit described in getrlimit(2).  Can also be caused by exceeding the limit specified in /proc/sys/fs/nr_open.");
    declare_error!(EMLINK,           "Too many links (POSIX.1-2001).");
    declare_error!(EMSGSIZE,         "Message too long (POSIX.1-2001).");
    declare_error!(EMULTIHOP,        "Multihop attempted (POSIX.1-2001).");
    declare_error!(ENAMETOOLONG,     "Filename too long (POSIX.1-2001).");
    declare_error!(ENETDOWN,         "Network is down (POSIX.1-2001).");
    declare_error!(ENETRESET,        "Connection aborted by network (POSIX.1-2001).");
    declare_error!(ENETUNREACH,      "Network unreachable (POSIX.1-2001).");
    declare_error!(ENFILE,           "Too many open files in system (POSIX.1-2001).  On Linux, this  is  probably  a result of encountering the /proc/sys/fs/file-max limit (see proc(5)).");
    declare_error!(ENOANO,           "No anode.");
    declare_error!(ENOBUFS,          "No buffer space available (POSIX.1 (XSI STREAMS option)).");
    declare_error!(ENODATA,          "No message is available on the STREAM head read queue (POSIX.1-2001).");
    declare_error!(ENODEV,           "No such device (POSIX.1-2001).");
    declare_error!(ENOENT,           "No such file or directory (POSIX.1-2001).");
    declare_error!(ENOEXEC,          "Exec format error (POSIX.1-2001).");
    declare_error!(ENOKEY,           "Required key not available.");
    declare_error!(ENOLCK,           "No locks available (POSIX.1-2001).");
    declare_error!(ENOLINK,          "Link has been severed (POSIX.1-2001).");
    declare_error!(ENOMEDIUM,        "No medium found.");
    declare_error!(ENOMEM,           "Not enough space/cannot allocate memory (POSIX.1-2001).");
    declare_error!(ENOMSG,           "No message of the desired type (POSIX.1-2001).");
    declare_error!(ENONET,           "Machine is not on the network.");
    declare_error!(ENOPKG,           "Package not installed.");
    declare_error!(ENOPROTOOPT,      "Protocol not available (POSIX.1-2001).");
    declare_error!(ENOSPC,           "No space left on device (POSIX.1-2001).");
    declare_error!(ENOSR,            "No STREAM resources (POSIX.1 (XSI STREAMS option)).");
    declare_error!(ENOSTR,           "Not a STREAM (POSIX.1 (XSI STREAMS option)).");
    declare_error!(ENOSYS,           "Function not implemented (POSIX.1-2001).");
    declare_error!(ENOTBLK,          "Block device required.");
    declare_error!(ENOTCONN,         "The socket is not connected (POSIX.1-2001).");
    declare_error!(ENOTDIR,          "Not a directory (POSIX.1-2001).");
    declare_error!(ENOTEMPTY,        "Directory not empty (POSIX.1-2001).");
    declare_error!(ENOTRECOVERABLE,  "State not recoverable (POSIX.1-2008).");
    declare_error!(ENOTSOCK,         "Not a socket (POSIX.1-2001).");
    declare_error!(ENOTTY,           "Inappropriate I/O control operation (POSIX.1-2001).");
    declare_error!(ENOTUNIQ,         "Name not unique on network.");
    declare_error!(ENXIO,            "No such device or address (POSIX.1-2001).");
    declare_error!(EOPNOTSUPP,       "Operation not supported on socket (POSIX.1-2001).");
    declare_error!(EOVERFLOW,        "Value too large to be stored in data type (POSIX.1-2001).");
    declare_error!(EOWNERDEAD,       "Owner died (POSIX.1-2008).");
    declare_error!(EPERM,            "Operation not permitted (POSIX.1-2001).");
    declare_error!(EPFNOSUPPORT,     "Protocol family not supported.");
    declare_error!(EPIPE,            "Broken pipe (POSIX.1-2001).");
    declare_error!(EPROTO,           "Protocol error (POSIX.1-2001).");
    declare_error!(EPROTONOSUPPORT,  "Protocol not supported (POSIX.1-2001).");
    declare_error!(EPROTOTYPE,       "Protocol wrong type for socket (POSIX.1-2001).");
    declare_error!(ERANGE,           "Result too large (POSIX.1, C99).");
    declare_error!(EREMCHG,          "Remote address changed.");
    declare_error!(EREMOTE,          "Object is remote.");
    declare_error!(EREMOTEIO,        "Remote I/O error.");
    declare_error!(ERESTART,         "Interrupted system call should be restarted.");
    declare_error!(ERFKILL,          "Operation not possible due to RF-kill.");
    declare_error!(EROFS,            "Read-only filesystem (POSIX.1-2001).");
    declare_error!(ESHUTDOWN,        "Cannot send after transport endpoint shutdown.");
    declare_error!(ESPIPE,           "Invalid seek (POSIX.1-2001).");
    declare_error!(ESOCKTNOSUPPORT,  "Socket type not supported.");
    declare_error!(ESRCH,            "No such process (POSIX.1-2001).");
    declare_error!(ESTALE,           "Stale file handle (POSIX.1-2001).");
    declare_error!(ESTRPIPE,         "Streams pipe error.");
    declare_error!(ETIME,            "Timer expired (POSIX.1 (XSI STREAMS option)).");
    declare_error!(ETIMEDOUT,        "Connection timed out (POSIX.1-2001).");
    declare_error!(ETOOMANYREFS,     "Too many references: cannot splice.");
    declare_error!(ETXTBSY,          "Text file busy (POSIX.1-2001).");
    declare_error!(EUCLEAN,          "Structure needs cleaning.");
    declare_error!(EUNATCH,          "Protocol driver not attached.");
    declare_error!(EUSERS,           "Too many users.");
    declare_error!(EWOULDBLOCK,      "Operation would block (may be same value as EAGAIN) (POSIX.1-2001).");
    declare_error!(EXDEV,            "Improper link (POSIX.1-2001).");
    declare_error!(EXFULL,           "Exchange full.");
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        extern "C" {
            fn rust_helper_errname(err: c_types::c_int) -> *const c_types::c_char;
        }
        // SAFETY: FFI call.
        let name = unsafe { rust_helper_errname(-self.0) };

        if name.is_null() {
            // Print out number if no name can be found.
            return f.debug_tuple("Error").field(&-self.0).finish();
        }

        // SAFETY: `'static` string from C, and is not NULL.
        let cstr = unsafe { CStr::from_char_ptr(name) };
        // SAFETY: These strings are ASCII-only.
        let str = unsafe { str::from_utf8_unchecked(&cstr) };
        f.debug_tuple(str).finish()
    }
}

impl From<TryFromIntError> for Error {
    fn from(_: TryFromIntError) -> Error {
        Error::EINVAL
    }
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Error {
        Error::EINVAL
    }
}

impl From<TryReserveError> for Error {
    fn from(_: TryReserveError) -> Error {
        Error::ENOMEM
    }
}

/// A [`Result`] with an [`Error`] error type.
///
/// To be used as the return type for functions that may fail.
///
/// # Error codes in C and Rust
///
/// In C, it is common that functions indicate success or failure through
/// their return value; modifying or returning extra data through non-`const`
/// pointer parameters. In particular, in the kernel, functions that may fail
/// typically return an `int` that represents a generic error code. We model
/// those as [`Error`].
///
/// In Rust, it is idiomatic to model functions that may fail as returning
/// a [`Result`]. Since in the kernel many functions return an error code,
/// [`Result`] is a type alias for a [`core::result::Result`] that uses
/// [`Error`] as its error type.
///
/// Note that even if a function does not return anything when it succeeds,
/// it should still be modeled as returning a `Result` rather than
/// just an [`Error`].
pub type Result<T = ()> = core::result::Result<T, Error>;

impl From<AllocError> for Error {
    fn from(_: AllocError) -> Error {
        Error::ENOMEM
    }
}

// # Invariant: `-bindings::MAX_ERRNO` fits in an `i16`.
crate::static_assert!(bindings::MAX_ERRNO <= -(i16::MIN as i32) as u32);

#[doc(hidden)]
pub fn from_kernel_result_helper<T>(r: Result<T>) -> T
where
    T: From<i16>,
{
    match r {
        Ok(v) => v,
        // NO-OVERFLOW: negative `errno`s are no smaller than `-bindings::MAX_ERRNO`,
        // `-bindings::MAX_ERRNO` fits in an `i16` as per invariant above,
        // therefore a negative `errno` always fits in an `i16` and will not overflow.
        Err(e) => T::from(e.to_kernel_errno() as i16),
    }
}

/// Transforms a [`crate::error::Result<T>`] to a kernel C integer result.
///
/// This is useful when calling Rust functions that return [`crate::error::Result<T>`]
/// from inside `extern "C"` functions that need to return an integer
/// error result.
///
/// `T` should be convertible to an `i16` via `From<i16>`.
///
/// # Examples
///
/// ```ignore
/// # use kernel::from_kernel_result;
/// # use kernel::c_types;
/// # use kernel::bindings;
/// unsafe extern "C" fn probe_callback(
///     pdev: *mut bindings::platform_device,
/// ) -> c_types::c_int {
///     from_kernel_result! {
///         let ptr = devm_alloc(pdev)?;
///         rust_helper_platform_set_drvdata(pdev, ptr);
///         Ok(0)
///     }
/// }
/// ```
#[macro_export]
macro_rules! from_kernel_result {
    ($($tt:tt)*) => {{
        $crate::error::from_kernel_result_helper((|| {
            $($tt)*
        })())
    }};
}

/// Transform a kernel "error pointer" to a normal pointer.
///
/// Some kernel C API functions return an "error pointer" which optionally
/// embeds an `errno`. Callers are supposed to check the returned pointer
/// for errors. This function performs the check and converts the "error pointer"
/// to a normal pointer in an idiomatic fashion.
///
/// # Examples
///
/// ```ignore
/// # use kernel::prelude::*;
/// # use kernel::from_kernel_err_ptr;
/// # use kernel::c_types;
/// # use kernel::bindings;
/// fn devm_platform_ioremap_resource(
///     pdev: &mut PlatformDevice,
///     index: u32,
/// ) -> Result<*mut c_types::c_void> {
///     // SAFETY: FFI call.
///     unsafe {
///         from_kernel_err_ptr(bindings::devm_platform_ioremap_resource(
///             pdev.to_ptr(),
///             index,
///         ))
///     }
/// }
/// ```
// TODO: remove `dead_code` marker once an in-kernel client is available.
#[allow(dead_code)]
pub(crate) fn from_kernel_err_ptr<T>(ptr: *mut T) -> Result<*mut T> {
    extern "C" {
        #[allow(improper_ctypes)]
        fn rust_helper_is_err(ptr: *const c_types::c_void) -> bool;

        #[allow(improper_ctypes)]
        fn rust_helper_ptr_err(ptr: *const c_types::c_void) -> c_types::c_long;
    }

    // CAST: casting a pointer to `*const c_types::c_void` is always valid.
    let const_ptr: *const c_types::c_void = ptr.cast();
    // SAFETY: the FFI function does not deref the pointer.
    if unsafe { rust_helper_is_err(const_ptr) } {
        // SAFETY: the FFI function does not deref the pointer.
        let err = unsafe { rust_helper_ptr_err(const_ptr) };
        // CAST: if `rust_helper_is_err()` returns `true`,
        // then `rust_helper_ptr_err()` is guaranteed to return a
        // negative value greater-or-equal to `-bindings::MAX_ERRNO`,
        // which always fits in an `i16`, as per the invariant above.
        // And an `i16` always fits in an `i32`. So casting `err` to
        // an `i32` can never overflow, and is always valid.
        //
        // SAFETY: `rust_helper_is_err()` ensures `err` is a
        // negative value greater-or-equal to `-bindings::MAX_ERRNO`
        return Err(unsafe { Error::from_kernel_errno_unchecked(err as i32) });
    }
    Ok(ptr)
}
