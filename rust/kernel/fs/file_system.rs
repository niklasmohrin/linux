use core::ptr;
use core::marker::PhantomData;
use core::pin::Pin;
use core::str::from_utf8;

use alloc::boxed::Box;

use crate::{
    bindings, c_types::*, error::KernelResultExt, fs::super_block::SuperBlock, str::CStr,
    types::FileSystemFlags, Result, pr_warn, Error
};

pub type FileSystemType = bindings::file_system_type;

pub trait FileSystemBase {
    type MountOptions = c_void;

    const NAME: &'static CStr;
    const FS_FLAGS: FileSystemFlags;
    const OWNER: *mut bindings::module = ptr::null_mut();

    fn mount(
        fs_type: &'_ mut FileSystemType,
        flags: c_int,
        device_name: &CStr,
        data: Option<&mut Self::MountOptions>,
    ) -> Result<*mut bindings::dentry>;

    fn kill_super(sb: &mut SuperBlock);

    fn fill_super(
        sb: &mut SuperBlock,
        data: Option<&mut Self::MountOptions>,
        silent: c_int,
    ) -> Result;
}

pub trait DeclaredFileSystemType: FileSystemBase {
    fn file_system_type() -> *mut bindings::file_system_type;
}

#[macro_export]
macro_rules! declare_fs_type {
    ($T:ty, $S:ident) => {
        static mut $S: $crate::bindings::file_system_type = $crate::bindings::file_system_type {
            name: <$T as $crate::fs::file_system::FileSystemBase>::NAME.as_char_ptr() as *const _,
            fs_flags: <$T as $crate::fs::file_system::FileSystemBase>::FS_FLAGS.into_int(),
            owner: <$T as $crate::fs::file_system::FileSystemBase>::OWNER,
            mount: Some($crate::fs::file_system::mount_callback::<$T>),
            kill_sb: Some($crate::fs::file_system::kill_superblock_callback::<$T>),
            ..$crate::fs::file_system::DEFAULT_FS_TYPE
        };
        impl $crate::fs::file_system::DeclaredFileSystemType for $T {
            fn file_system_type() -> *mut $crate::bindings::file_system_type {
                unsafe { &mut $S as *mut _ }
            }
        }
    };
}

pub unsafe extern "C" fn mount_callback<T: FileSystemBase>(
    fs_type: *mut bindings::file_system_type,
    flags: c_int,
    device_name: *const c_char,
    data: *mut c_void,
) -> *mut bindings::dentry {
    unsafe {
        let fs_type = &mut *fs_type;
        let device_name = CStr::from_char_ptr(device_name);
        let data = (data as *mut T::MountOptions).as_mut();
        T::mount(fs_type, flags, device_name, data).unwrap_or_err_ptr()
    }
}

pub unsafe extern "C" fn kill_superblock_callback<T: FileSystemBase>(
    sb: *mut bindings::super_block,
) {
    unsafe {
        let sb = sb
            .as_mut()
            .expect("kill_superblock got NULL super block")
            .as_mut();
        T::kill_super(sb);
    }
}

pub const DEFAULT_FS_TYPE: bindings::file_system_type = bindings::file_system_type {
    name: ptr::null(),
    fs_flags: 0,
    init_fs_context: None,
    parameters: ptr::null(),
    mount: None,
    kill_sb: None,
    owner: ptr::null_mut(),
    next: ptr::null_mut(),
    fs_supers: bindings::hlist_head {
        first: ptr::null_mut(),
    },
    s_lock_key: bindings::lock_class_key {},
    s_umount_key: bindings::lock_class_key {},
    s_vfs_rename_key: bindings::lock_class_key {},
    s_writers_key: [bindings::lock_class_key {}; 3],
    i_lock_key: bindings::lock_class_key {},
    i_mutex_key: bindings::lock_class_key {},
    i_mutex_dir_key: bindings::lock_class_key {},
};

pub struct Registration<T: FileSystemBase> {
    phantom: PhantomData<T>,
    fs_type: FileSystemType,
}

//Pin self
impl<T: FileSystemBase> Registration<T> {
    fn new(fs_type: FileSystemType ) -> Self {
        Self {
            phantom: PhantomData,
            fs_type: fs_type,
        }
    }

    pub fn new_pinned() -> Result<Pin<Box<Self>>> {
        let mut c_fs_type = FileSystemType::default(); // may use DEFAULT_FS_TYPE?
        c_fs_type.mount = Some(mount_callback::<T>);
        c_fs_type.kill_sb = Some(kill_superblock_callback::<T>);
        c_fs_type.owner = T::OWNER;
        c_fs_type.name = T::NAME.as_char_ptr();
        c_fs_type.fs_flags = T::FS_FLAGS.into_int();

        Ok(Pin::from(Box::try_new(Self::new(c_fs_type))?))
    }

    pub fn register(&mut self) -> Result {
        let err = unsafe { bindings::register_filesystem(&mut self.fs_type) };
        if err != 0 {
            return Err(Error::from_kernel_errno(err));
        }

        Ok(())
    }

    fn unregister(&mut self) -> Result {
        let err = unsafe { bindings::unregister_filesystem(&mut self.fs_type) };
        if err != 0 {
            return Err(Error::from_kernel_errno(err));
        }

        Ok(())
    }
}

impl<T: FileSystemBase> Drop for Registration<T> {
    fn drop(&mut self) {
        if let Err(_) = self.unregister() {
            let fs_name = from_utf8(T::NAME.as_bytes()).unwrap();
            pr_warn!("Unregister filesystem {} failed", fs_name);
        }
    }
}
