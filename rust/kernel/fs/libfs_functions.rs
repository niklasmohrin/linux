use crate::bindings;
use crate::c_types::*;
use crate::error::Error;
use crate::file::File;
use crate::file_operations::SeekFrom;
use crate::fs::dentry::Dentry;
use crate::fs::from_kernel_err_ptr;
use crate::fs::inode::Inode;
use crate::fs::kiocb::Kiocb;
use crate::fs::super_block::SuperBlock;
use crate::fs::super_operations::Kstatfs;
use crate::fs::DeclaredFileSystemType;
use crate::iov_iter::IovIter;
use crate::Result;
use core::ptr;

pub fn generic_file_read_iter(iocb: &mut Kiocb, iter: &mut IovIter) -> Result<usize> {
    Error::parse_int(unsafe { bindings::generic_file_read_iter(iocb.as_ptr_mut(), iter.ptr) as _ })
}

pub fn generic_file_write_iter(iocb: &mut Kiocb, iter: &mut IovIter) -> Result<usize> {
    Error::parse_int(unsafe { bindings::generic_file_write_iter(iocb.as_ptr_mut(), iter.ptr) as _ })
}

pub fn generic_file_mmap(file: &File, vma: &mut bindings::vm_area_struct) -> Result {
    Error::parse_int(unsafe { bindings::generic_file_mmap(file.ptr, vma as *mut _) }).map(|_| ())
}

pub fn noop_fsync(file: &File, start: u64, end: u64, datasync: bool) -> Result<u32> {
    let start = start as _;
    let end = end as _;
    let datasync = if datasync { 1 } else { 0 };
    let res = unsafe { bindings::noop_fsync(file.ptr, start, end, datasync) };
    if res == 0 {
        Ok(0)
    } else {
        Err(Error::EINVAL)
        // Err(Error::from_kernel_errno(bindings::errno))
    }
}

pub fn generic_file_llseek(file: &File, pos: SeekFrom) -> Result<u64> {
    let (offset, whence) = pos.into_pos_and_whence();
    Error::parse_int(
        unsafe { bindings::generic_file_llseek(file.ptr, offset as _, whence as _) } as _,
    )
}

pub fn generic_file_splice_read(
    file: &File,
    pos: *mut i64,
    pipe: &mut bindings::pipe_inode_info,
    len: usize,
    flags: u32,
) -> Result<usize> {
    Error::parse_int(unsafe {
        bindings::generic_file_splice_read(file.ptr, pos, pipe as *mut _, len, flags) as _
    })
}

pub fn iter_file_splice_write(
    pipe: &mut bindings::pipe_inode_info,
    file: &File,
    pos: *mut i64,
    len: usize,
    flags: u32,
) -> Result<usize> {
    Error::parse_int(unsafe {
        bindings::iter_file_splice_write(pipe as *mut _, file.ptr, pos, len, flags) as _
    })
}

pub fn generic_delete_inode(inode: &mut Inode) -> Result {
    Error::parse_int(unsafe { bindings::generic_delete_inode(inode.as_ptr_mut()) }).map(|_| ())
}

pub fn simple_statfs(root: &mut Dentry, buf: &mut Kstatfs) -> Result {
    Error::parse_int(unsafe { bindings::simple_statfs(root.as_ptr_mut(), buf as *mut _) })
        .map(|_| ())
}

pub fn register_filesystem<T: DeclaredFileSystemType>() -> Result {
    Error::parse_int(unsafe { bindings::register_filesystem(T::file_system_type()) }).map(|_| ())
}
pub fn unregister_filesystem<T: DeclaredFileSystemType>() -> Result {
    Error::parse_int(unsafe { bindings::unregister_filesystem(T::file_system_type()) }).map(|_| ())
}

pub fn mount_nodev<T: DeclaredFileSystemType>(
    flags: c_int,
    data: Option<&mut T::MountOptions>,
) -> Result<*mut bindings::dentry> {
    from_kernel_err_ptr(unsafe {
        bindings::mount_nodev(
            T::file_system_type(),
            flags,
            data.map(|p| p as *mut _ as *mut _)
                .unwrap_or_else(ptr::null_mut),
            Some(T::fill_super_raw),
        )
    })
}

pub fn kill_litter_super(sb: &mut SuperBlock) {
    unsafe {
        bindings::kill_litter_super(sb.as_ptr_mut());
    }
}
