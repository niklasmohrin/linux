pub mod dentry;
pub mod inode;
pub mod inode_operations;
pub mod kiocb;
pub mod libfs_functions;
pub mod super_block;
pub mod super_operations;

use core::ptr;

use crate::{
    bindings, c_types::*, error::from_kernel_err_ptr, fs::super_block::SuperBlock, ret_err_ptr,
    str::CStr, Result,
};

pub type FileSystemType = bindings::file_system_type;

pub trait FileSystemBase {
    type MountOptions = c_void;

    const NAME: &'static CStr;
    const FS_FLAGS: c_int;
    const OWNER: *mut bindings::module;

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
            name: <$T as $crate::fs::FileSystemBase>::NAME.as_char_ptr() as *const _,
            fs_flags: <$T as $crate::fs::FileSystemBase>::FS_FLAGS,
            owner: <$T as $crate::fs::FileSystemBase>::OWNER,
            mount: Some($crate::fs::mount_callback::<$T>),
            kill_sb: Some($crate::fs::kill_superblock_callback::<$T>),
            ..$crate::fs::DEFAULT_FS_TYPE
        };
        impl $crate::fs::DeclaredFileSystemType for $T {
            fn file_system_type() -> *mut $crate::bindings::file_system_type {
                unsafe { &mut $S as *mut _ }
            }
        }
    };
}

// Doesn't work because we need mutable access to an associated item
// pub struct FileSystemTypeVTable<T>(PhantomData<T>);
// impl<T: FileSystemBase> FileSystemTypeVTable<T> {
//     const VTABLE: bindings::file_system_type = bindings::file_system_type {
//         name: T::NAME.as_char_ptr() as *const _,
//         fs_flags: T::FS_FLAGS,
//         mount: Some(mount_callback::<T>),
//         kill_sb: Some(kill_superblock_callback::<T>),
//         owner: T::OWNER,
//         ..DEFAULT_FS_TYPE
//     };

//     pub const fn build() -> &'static bindings::file_system_type {
//         &Self::VTABLE
//     }
// }

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
        ret_err_ptr!(T::mount(fs_type, flags, device_name, data))
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

pub const DEFAULT_ADDRESS_SPACE_OPERATIONS: bindings::address_space_operations =
    bindings::address_space_operations {
        readpage: None,
        readahead: None,
        write_begin: None,
        write_end: None,
        set_page_dirty: None,
        writepage: None,
        writepages: None,
        readpages: None,
        bmap: None,
        invalidatepage: None,
        releasepage: None,
        freepage: None,
        direct_IO: None,
        migratepage: None,
        isolate_page: None,
        putback_page: None,
        launder_page: None,
        is_partially_uptodate: None,
        is_dirty_writeback: None,
        error_remove_page: None,
        swap_activate: None,
        swap_deactivate: None,
    };

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
