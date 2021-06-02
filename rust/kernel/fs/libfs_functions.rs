use crate::bindings;
use crate::error::Error;
use crate::file::File;
use crate::file_operations::SeekFrom;
use crate::Result;

pub fn generic_file_llseek(file: &File, pos: SeekFrom) -> Result<u64> {
    let (offset, whence) = pos.into_pos_and_whence();
    Error::parse_int(
        unsafe { bindings::generic_file_llseek(file.ptr, offset as _, whence as _) } as _,
    )
}
