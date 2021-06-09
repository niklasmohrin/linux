#![no_std]
#![feature(allocator_api, global_asm)]

use alloc::boxed::Box;
use core::{mem, ptr};

use kernel::file::File;
use kernel::file_operations::{FileOperations, SeekFrom};
use kernel::fs::kiocb::Kiocb;
use kernel::fs::super_operations::{Kstatfs, SeqFile, SuperOperations};
use kernel::iov_iter::IovIter;
use kernel::{bindings, c_types::*, prelude::*, str::CStr, Error, Mode};

// should be renamed at some point
use kernel::fs::{
    dentry::Dentry,
    inode::{Inode, UpdateATime, UpdateCTime, UpdateMTime},
    libfs_functions,
    super_block::SuperBlock,
    FileSystem, FileSystemBase, FileSystemType, DEFAULT_ADDRESS_SPACE_OPERATIONS,
    DEFAULT_INODE_OPERATIONS,
};

const PAGE_SHIFT: u32 = 12; // x86 (maybe)
const MAX_LFS_FILESIZE: c_longlong = c_longlong::MAX;
const BS2RAMFS_MAGIC: u64 = 0x858458f6; // ~~one less than~~ ramfs, should not clash with anything (maybe)

extern "C" {
    fn rust_helper_mapping_set_unevictable(mapping: *mut bindings::address_space);
    fn rust_helper_mapping_set_gfp_mask(
        mapping: *mut bindings::address_space,
        mask: bindings::gfp_t,
    );
    static RUST_HELPER_GFP_HIGHUSER: bindings::gfp_t;
}

module! {
    type: BS2Ramfs,
    name: b"bs2ramfs",
    author: b"Rust for Linux Contributors",
    description: b"RAMFS",
    license: b"GPL v2",
}

struct BS2Ramfs;

impl FileSystemBase for BS2Ramfs {
    const NAME: &'static CStr = kernel::c_str!("bs2ramfs_name");
    const FS_FLAGS: c_int = bindings::FS_USERNS_MOUNT as _;
    const OWNER: *mut bindings::module = ptr::null_mut();

    fn mount(
        _fs_type: &'_ mut FileSystemType,
        flags: c_int,
        _device_name: &CStr,
        data: Option<&mut Self::MountOptions>,
    ) -> Result<*mut bindings::dentry> {
        Self::mount_nodev(flags, data)
    }

    fn kill_super(sb: &mut SuperBlock) {
        let _ = unsafe { Box::from_raw(mem::replace(&mut sb.s_fs_info, ptr::null_mut())) };
        Self::kill_litter_super(sb);
    }

    fn fill_super(
        sb: &mut SuperBlock,
        _data: Option<&mut Self::MountOptions>,
        _silent: c_int,
    ) -> Result {
        pr_emerg!("Reached ramfs_fill_super_impl");

        sb.s_magic = BS2RAMFS_MAGIC;
        let ops = Bs2RamfsSuperOps::default();
        unsafe {
            // TODO: investigate if this really has to be set to NULL in case we run out of memory
            sb.s_root = ptr::null_mut();
            let inode = ramfs_get_inode(sb, None, Mode::S_IFDIR | ops.mount_opts.mode, 0);
            pr_emerg!("Completed ramfs_fill_super_impl::get_inode");
            sb.s_root = inode.and_then(Dentry::make_root).ok_or(Error::ENOMEM)? as *mut _ as *mut _;
        }
        pr_emerg!("(rust) s_root: {:?}", sb.s_root);
        sb.set_super_operations(ops);
        sb.s_maxbytes = MAX_LFS_FILESIZE;
        sb.s_blocksize = kernel::PAGE_SIZE as _;
        sb.s_blocksize_bits = PAGE_SHIFT as _;
        sb.s_time_gran = 1;
        pr_emerg!("SB members set");

        Ok(())
    }
}
kernel::declare_fs_type!(BS2Ramfs, BS2RAMFS_FS_TYPE);

impl KernelModule for BS2Ramfs {
    fn init() -> Result<Self> {
        pr_emerg!("bs2 ramfs in action");
        Self::register().map(move |_| Self)
    }
}

impl Drop for BS2Ramfs {
    fn drop(&mut self) {
        let _ = Self::unregister();
        pr_info!("bs2 ramfs out of action");
    }
}

struct RamfsMountOpts {
    pub mode: Mode,
}

impl Default for RamfsMountOpts {
    fn default() -> Self {
        Self {
            mode: Mode::from_int(0o775),
        }
    }
}

unsafe extern "C" fn ramfs_show_options(
    _m: *mut bindings::seq_file,
    _root: *mut bindings::dentry,
) -> c_int {
    pr_emerg!("ramfs show options, doing nothing");
    0
}

#[derive(Default)]
struct Bs2RamfsFileOps;

impl FileOperations for Bs2RamfsFileOps {
    kernel::declare_file_operations!(
        read_iter,
        write_iter,
        mmap,
        fsync,
        splice_read,
        splice_write,
        seek,
        get_unmapped_area
    );

    fn read_iter(&self, iocb: &mut Kiocb, iter: &mut IovIter) -> Result<usize> {
        libfs_functions::generic_file_read_iter(iocb, iter)
    }

    fn write_iter(&self, iocb: &mut Kiocb, iter: &mut IovIter) -> Result<usize> {
        libfs_functions::generic_file_write_iter(iocb, iter)
    }

    fn mmap(&self, file: &File, vma: &mut bindings::vm_area_struct) -> Result {
        libfs_functions::generic_file_mmap(file, vma)
    }

    fn fsync(&self, file: &File, start: u64, end: u64, datasync: bool) -> Result<u32> {
        libfs_functions::noop_fsync(file, start, end, datasync)
    }

    fn get_unmapped_area(
        &self,
        _file: &File,
        _addr: u64,
        _len: u64,
        _pgoff: u64,
        _flags: u64,
    ) -> Result<u64> {
        pr_emerg!(
            "AKAHSDkADKHAKHD WE ARE ABOUT TO PANIC (IN MMU_GET_UNMAPPED_AREA;;;; LOOK HERE COME ON"
        );
        unimplemented!()
    }

    fn seek(&self, file: &File, pos: SeekFrom) -> Result<u64> {
        libfs_functions::generic_file_llseek(file, pos)
    }

    fn splice_read(
        &self,
        file: &File,
        pos: *mut i64,
        pipe: &mut bindings::pipe_inode_info,
        len: usize,
        flags: u32,
    ) -> Result<usize> {
        libfs_functions::generic_file_splice_read(file, pos, pipe, len, flags)
    }

    fn splice_write(
        &self,
        pipe: &mut bindings::pipe_inode_info,
        file: &File,
        pos: *mut i64,
        len: usize,
        flags: u32,
    ) -> Result<usize> {
        libfs_functions::iter_file_splice_write(pipe, file, pos, len, flags)
    }
}

#[derive(Default)]
struct Bs2RamfsSuperOps {
    mount_opts: RamfsMountOpts,
}

impl SuperOperations for Bs2RamfsSuperOps {
    kernel::declare_super_operations!(statfs, drop_inode, show_options);

    fn drop_inode(&self, inode: &mut Inode) -> Result {
        libfs_functions::generic_delete_inode(inode)
    }

    fn statfs(&self, root: &mut Dentry, buf: &mut Kstatfs) -> Result {
        libfs_functions::simple_statfs(root, buf)
    }

    fn show_options(&self, _s: &mut SeqFile, _root: &mut Dentry) -> Result {
        pr_emerg!("ramfs show options, doing nothing");
        Ok(())
    }
}

static RAMFS_AOPS: bindings::address_space_operations = bindings::address_space_operations {
    readpage: Some(bindings::simple_readpage),
    write_begin: Some(bindings::simple_write_begin),
    write_end: Some(bindings::simple_write_end),
    set_page_dirty: Some(bindings::__set_page_dirty_nobuffers),
    ..DEFAULT_ADDRESS_SPACE_OPERATIONS
};

static RAMFS_FILE_INODE_OPS: bindings::inode_operations = bindings::inode_operations {
    setattr: Some(bindings::simple_setattr),
    getattr: Some(bindings::simple_getattr),
    ..DEFAULT_INODE_OPERATIONS
};

#[no_mangle]
pub unsafe fn ramfs_get_inode<'a>(
    sb: &'a mut SuperBlock,
    dir: Option<&'_ mut Inode>,
    mode: Mode,
    dev: bindings::dev_t,
) -> Option<&'a mut Inode> {
    Inode::new(sb).map(|inode| {
        inode.i_ino = Inode::next_ino() as _;
        inode.init_owner(&mut bindings::init_user_ns, dir, mode);

        (*inode.i_mapping).a_ops = &RAMFS_AOPS;
        rust_helper_mapping_set_gfp_mask(inode.i_mapping, RUST_HELPER_GFP_HIGHUSER);
        rust_helper_mapping_set_unevictable(inode.i_mapping);

        inode.update_acm_time(UpdateATime::Yes, UpdateCTime::Yes, UpdateMTime::Yes);
        match mode & Mode::S_IFMT {
            Mode::S_IFREG => {
                inode.i_op = &RAMFS_FILE_INODE_OPS;
                inode.set_file_operations::<Bs2RamfsFileOps>();
            }
            Mode::S_IFDIR => {
                inode.i_op = &RAMFS_DIR_INODE_OPS;
                inode.__bindgen_anon_3.i_fop = &bindings::simple_dir_operations;
                inode.inc_nlink();
            }
            Mode::S_IFLNK => {
                inode.i_op = &bindings::page_symlink_inode_operations;
                inode.nohighmem();
            }
            _ => {
                inode.init_special(mode, dev);
            }
        }

        inode
    })
}

unsafe extern "C" fn ramfs_mknod(
    _ns: *mut bindings::user_namespace,
    dir: *mut bindings::inode,
    dentry: *mut bindings::dentry,
    mode: bindings::umode_t,
    dev: bindings::dev_t,
) -> i32 {
    let dir = dir
        .as_mut()
        .expect("ramfs_mknod got NULL directory")
        .as_mut();
    ramfs_get_inode(
        dir.i_sb
            .as_mut()
            .expect("dir has NULL super block")
            .as_mut(),
        Some(dir),
        Mode::from_int(mode),
        dev,
    )
    .map_or(Error::ENOSPC.to_kernel_errno(), move |inode| {
        let dentry = dentry
            .as_mut()
            .expect("Called ramfs_mknod with NULL dentry")
            .as_mut();
        dentry.instantiate(inode);
        dentry.get();
        dir.update_acm_time(UpdateATime::No, UpdateCTime::Yes, UpdateMTime::Yes);
        0
    })
}

unsafe extern "C" fn ramfs_mkdir(
    ns: *mut bindings::user_namespace,
    dir: *mut bindings::inode,
    dentry: *mut bindings::dentry,
    mode: bindings::umode_t,
) -> i32 {
    let dir = dir
        .as_mut()
        .expect("ramfs_mkdir got NULL directory")
        .as_mut();
    if ramfs_mknod(
        ns,
        dir.as_ptr_mut(),
        dentry,
        mode | Mode::S_IFDIR.as_int() as bindings::umode_t,
        0,
    ) < 0
    {
        dir.inc_nlink();
    }
    0
}

unsafe extern "C" fn ramfs_create(
    ns: *mut bindings::user_namespace,
    dir: *mut bindings::inode,
    dentry: *mut bindings::dentry,
    mode: bindings::umode_t,
    _excl: bool,
) -> i32 {
    ramfs_mknod(
        ns,
        dir,
        dentry,
        mode | Mode::S_IFREG.as_int() as bindings::umode_t,
        0,
    )
}

#[no_mangle]
unsafe extern "C" fn ramfs_symlink(
    _ns: *mut bindings::user_namespace,
    dir: *mut bindings::inode,
    dentry: *mut bindings::dentry,
    symname: *const c_char,
) -> i32 {
    pr_info!("in symlink");
    let dir = dir
        .as_mut()
        .expect("ramfs_symlink got NULL directory")
        .as_mut();
    ramfs_get_inode(
        dir.i_sb
            .as_mut()
            .expect("dir had NULL super block")
            .as_mut(),
        Some(dir),
        Mode::S_IFLNK | Mode::S_IRWXUGO,
        0,
    )
    .map_or(Error::ENOSPC.to_kernel_errno(), |inode| {
        pr_info!("got inode ptr {:?}", inode.as_ptr_mut());
        let l = bindings::strlen(symname) + 1;
        pr_info!("str has len {:?}", l);
        let ret = bindings::page_symlink(inode.as_ptr_mut(), symname, l as _);
        if ret == 0 {
            pr_info!("page_symlink is ok");
            let dentry = dentry
                .as_mut()
                .expect("Called ramfs_symlink with NULL dentry")
                .as_mut();
            dentry.instantiate(inode);
            dentry.get();
            dir.update_acm_time(UpdateATime::No, UpdateCTime::Yes, UpdateMTime::Yes);
            pr_info!("current_time is ok");
        } else {
            inode.put();
            pr_info!("iput is ok");
        }
        ret
    })
}

static RAMFS_DIR_INODE_OPS: bindings::inode_operations = bindings::inode_operations {
    create: Some(ramfs_create),
    lookup: Some(bindings::simple_lookup),
    link: Some(bindings::simple_link),
    unlink: Some(bindings::simple_unlink),
    symlink: Some(ramfs_symlink),
    mkdir: Some(ramfs_mkdir),
    rmdir: Some(bindings::simple_rmdir),
    mknod: Some(ramfs_mknod),
    rename: Some(bindings::simple_rename),
    ..DEFAULT_INODE_OPERATIONS
};
