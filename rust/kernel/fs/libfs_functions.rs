use crate::bindings;
use crate::error::Error;
use crate::file::File;
use crate::file_operations::SeekFrom;
use crate::fs::kiocb::Kiocb;
use crate::iov_iter::IovIter;
use crate::Result;

pub fn generic_file_read_iter(iocb: &mut Kiocb, iter: &mut IovIter) -> Result<usize> {
    Error::parse_int(
        unsafe { bindings::generic_file_read_iter(iocb.as_ptr_mut(), iter.ptr) as _ }
    )
}

pub fn generic_file_write_iter(iocb: &mut Kiocb, iter: &mut IovIter) -> Result<usize> {
    Error::parse_int(
        unsafe { bindings::generic_file_write_iter(iocb.as_ptr_mut(), iter.ptr) as _ }
    )
}

pub fn generic_file_mmap(file: &File, vma: &mut bindings::vm_area_struct) -> Result {
    Error::parse_int(
        unsafe { bindings::generic_file_mmap(file.ptr, vma as *mut _) }
    ).map(|_| ())
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

pub fn generic_file_splice_read(file: &File, pos: *mut i64, pipe: &mut bindings::pipe_inode_info, len: usize, flags: u32) -> Result<usize> {
    Error::parse_int(
        unsafe { bindings::generic_file_splice_read(file.ptr, pos, pipe as *mut _, len, flags) as _ }
    )
}

pub fn iter_file_splice_write(pipe: &mut bindings::pipe_inode_info, file: &File, pos: *mut i64, len: usize, flags: u32) -> Result<usize> {
    Error::parse_int(
        unsafe { bindings::iter_file_splice_write(pipe as *mut _, file.ptr, pos, len, flags) as _ }
    )
}
