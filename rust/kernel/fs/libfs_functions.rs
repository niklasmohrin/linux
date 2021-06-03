use crate::bindings;
use crate::error::Error;
use crate::file::File;
use crate::file_operations::{SeekFrom, Kiocb};
use crate::iov_iter::IovIter;
use crate::Result;

pub fn generic_file_read_iter(iocb: &mut Kiocb, iter: &mut IovIter) -> Result<usize> {
    Error::parse_int(
        unsafe { bindings::generic_file_read_iter(iocb as *mut _, iter.ptr) as _ }
    )
}
// pub fn generic_file_read_iter(iocb: &mut Kiocb, iter: &mut IovIter) -> Result<c_types::c_ssize_t> {
// struct kiocb * iocb, struct iov_iter * iter

// }

// pub fn generic_file_write_iter(iocb: &mut Kiocb, from: &mut IovIter) -> Result<c_types::c_ssize_t> {
// //ssize_t generic_file_write_iter(struct kiocb * iocb, struct iov_iter * from);
// }

// pub fn generic_file_mmap() {

// }

pub fn noop_fsync(file: &File, start: u64, end: u64, datasync: bool) -> Result<u32> {
    let start = start as _;
    let end = start as _;
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
