use alloc::boxed::Box;
use core::ptr;

use crate::ret_err_ptr;
use crate::{
    bindings, buffer::Buffer, c_types::*, error::from_kernel_err_ptr, prelude::*, str::CStr, Error,
    Result,
};

pub type FileSystemType = bindings::file_system_type;
pub type SuperBlock = bindings::super_block;

pub trait FileSystemBase {
    type MountOptions = c_void;

    const NAME: &'static CStr;
    const FS_FLAGS: c_int;
    const OWNER: *mut bindings::module;

    // TODO: is that a fair return type here?
    fn mount(
        fs_type: &'_ mut FileSystemType,
        flags: c_int,
        device_name: &CStr,
        data: Option<&mut Self::MountOptions>,
    ) -> Result<*mut bindings::dentry>;

    // fn kill_superblock(sb: &mut SuperBlock);

    // fn fill_superblock(sb: Box<SuperBlock>, data: Box<MountOptions>, silent: bool);

    unsafe extern "C" fn mount_raw(
        fs_type: *mut bindings::file_system_type,
        flags: c_int,
        device_name: *const c_char,
        data: *mut c_void,
    ) -> *mut bindings::dentry {
        pr_emerg!("in mount_raw");
        let fs_type = &mut *fs_type;
        let device_name = CStr::from_char_ptr(device_name);
        let data = (data as *mut Self::MountOptions).as_mut();
        ret_err_ptr!(Self::mount(fs_type, flags, device_name, data))
    }

    unsafe extern "C" fn kill_sb_raw(sb: *mut bindings::super_block) {
        // let sb = SuperBlock::from_raw(sb);
        // self.kill_superblock(sb);
        // let _ = SuperBlock::into_raw(sb);
    }

    // fn mount_bdev(flags: u32, dev_name: &CStr, data: Box<MountOptions>) {
    //     bindings::mount_bdev(
    //         Self::file_system_type(),
    //         flags,
    //         CStr::into_raw(dev_name),
    //         MountOptions::into_raw(data),
    //         self.fill_super_raw
    //     );
    // }

    // unsafe extern "C" fn fill_super_raw(sb: *mut bindings::super_block, data: *mut c_void, silent: bool) {
    //     let sb = Box::from_raw(sb);
    //     let data = MountOptions::from_raw(data);
    //     self.fill_superblock(sb, data, silent);
    //     sb.s_magic = SuperBlock::MAGIC;
    //     sb.s_blocksize = SuperBlock::BLOCKSIZE;
    //     sb.s_blocksize_bits = SuperBlock::BLOCKSIZE_BITS;
    //     let s_ops = bindings::super_operations {
    //         alloc_inode: SuperBlock::alloc_inode_raw as *mut _,
    //         drop_inode: SuperBlock::drop_inode_raw as *mut _,
    //         statfs: SuperBlock::statfs_raw as *mut _,
    //         ..Default::defaults()
    //     };
    //     sb.s_op = s_ops;
    //     // ... register SuperBlock (associated) dropt_node, statfs, constants
    //     let _ = Box::into_raw(sb);
    //     let _ = MountOptions::into_raw(data);
    // }

    fn fill_super(
        _sb: &mut SuperBlock,
        _data: Option<&mut Self::MountOptions>,
        _silent: c_int,
    ) -> Result {
        pr_emerg!("Using default FileSystem::fill_super");
        Ok(())
    }

    unsafe extern "C" fn fill_super_raw(
        sb: *mut bindings::super_block,
        data: *mut c_void,
        silent: c_int,
    ) -> c_int {
        pr_emerg!("in fill_super_raw");
        let sb = &mut *sb;
        let data = (data as *mut Self::MountOptions).as_mut();
        Self::fill_super(sb, data, silent)
            .map(|_| 0)
            .unwrap_or_else(|e| e.to_kernel_errno())
    }
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
            mount: Some(<$T as $crate::fs::FileSystemBase>::mount_raw),
            kill_sb: Some(<$T as $crate::fs::FileSystemBase>::kill_sb_raw),
            owner: <$T as $crate::fs::FileSystemBase>::OWNER,
            ..$crate::fs::DEFAULT_FS_TYPE
        };
        impl $crate::fs::DeclaredFileSystemType for $T {
            fn file_system_type() -> *mut $crate::bindings::file_system_type {
                unsafe { &mut $S as *mut _ }
            }
        }
    };
}

pub trait FileSystem: FileSystemBase + DeclaredFileSystemType {
    fn register() -> Result {
        let err = unsafe { bindings::register_filesystem(Self::file_system_type()) };
        if err == 0 {
            Ok(())
        } else {
            Err(Error::from_kernel_errno(err))
        }
    }

    fn unregister() -> Result {
        let err = unsafe { bindings::unregister_filesystem(Self::file_system_type()) };
        if err == 0 {
            Ok(())
        } else {
            Err(Error::from_kernel_errno(err))
        }
    }

    fn mount_nodev(
        flags: c_int,
        data: Option<&mut Self::MountOptions>,
    ) -> Result<*mut bindings::dentry> {
        from_kernel_err_ptr(unsafe {
            bindings::mount_nodev(
                Self::file_system_type(),
                flags,
                data.map(|p| p as *mut _ as *mut _)
                    .unwrap_or_else(ptr::null_mut),
                Some(Self::fill_super_raw),
            )
        })
    }
}

impl<T: FileSystemBase + DeclaredFileSystemType> FileSystem for T {}

pub const DEFAULT_SUPER_OPS: bindings::super_operations = bindings::super_operations {
    statfs: None,
    drop_inode: None,
    show_options: None,
    alloc_inode: None,
    destroy_inode: None,
    dirty_inode: None,
    write_inode: None,
    evict_inode: None,
    put_super: None,
    sync_fs: None,
    freeze_super: None,
    freeze_fs: None,
    thaw_super: None,
    unfreeze_fs: None,
    remount_fs: None,
    umount_begin: None,
    show_devname: None,
    show_path: None,
    show_stats: None,
    quota_read: None,
    free_inode: None,
    quota_write: None,
    get_dquots: None,
    bdev_try_to_free_page: None,
    nr_cached_objects: None,
    free_cached_objects: None,
};

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

pub const DEFAULT_INODE_OPERATIONS: bindings::inode_operations = bindings::inode_operations {
    create: None,
    lookup: None,
    link: None,
    unlink: None,
    symlink: None,
    mkdir: None,
    rmdir: None,
    mknod: None,
    rename: None,
    listxattr: None,
    fiemap: None,
    update_time: None,
    tmpfile: None,
    set_acl: None,
    get_link: None,
    permission: None,
    get_acl: None,
    readlink: None,
    setattr: None,
    getattr: None,
    atomic_open: None,
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

pub const DEFAULT_FILE_OPERATIONS: bindings::file_operations = bindings::file_operations {
    owner: ptr::null_mut(),
    llseek: None,
    read: None,
    write: None,
    read_iter: None,
    write_iter: None,
    iopoll: None,
    iterate: None,
    iterate_shared: None,
    poll: None,
    unlocked_ioctl: None,
    compat_ioctl: None,
    mmap: None,
    mmap_supported_flags: 0,
    open: None,
    flush: None,
    release: None,
    fsync: None,
    fasync: None,
    lock: None,
    sendpage: None,
    get_unmapped_area: None,
    check_flags: None,
    flock: None,
    splice_write: None,
    splice_read: None,
    setlease: None,
    fallocate: None,
    show_fdinfo: None,
    copy_file_range: None,
    remap_file_range: None,
    fadvise: None,
};
