#![no_std]

use core::{ops::DerefMut, ptr};

use kernel::{
    bindings,
    c_types::*,
    declare_file_operations,
    file::File,
    file_operations::{FileAllocMode, FileOperations, IoctlCommand, SeekFrom},
    fs::{
        inode::Inode, kiocb::Kiocb, libfs_functions, super_block::SuperBlock, FileSystemBase,
        FileSystemType,
    },
    iov_iter::IovIter,
    prelude::*,
    print::ExpectK,
    str::CStr,
    types::Mode,
    Error,
};

module! {
    type: BS2Fat,
    name: b"bs2fat",
    author: b"Rust for Linux Contributors",
    description: b"MS-DOS filesystem support",
    license: b"GPL v2",
}

/* Characters that are undesirable in an MS-DOS file name */
const BAD_CHARS: &[u8] = b"*?<>|\"";
const BAD_IF_STRICT: &[u8] = b"+=,; ";

struct BS2Fat;

impl FileSystemBase for BS2Fat {
    const NAME: &'static CStr = kernel::c_str!("bs2fat");
    const FS_FLAGS: c_int = bindings::FS_USERNS_MOUNT as _;
    const OWNER: *mut bindings::module = ptr::null_mut();

    fn mount(
        _fs_type: &'_ mut FileSystemType,
        flags: c_int,
        _device_name: &CStr,
        data: Option<&mut Self::MountOptions>,
    ) -> Result<*mut bindings::dentry> {
        // libfs_functions::mount_bdev::<Self>(flags, data)
        unimplemented!()
    }

    fn kill_super(sb: &mut SuperBlock) {
        unimplemented!()
    }

    fn fill_super(
        sb: &mut SuperBlock,
        _data: Option<&mut Self::MountOptions>,
        _silent: c_int,
    ) -> Result {
        // sb.s_magic = BS2FAT_MAGIC;
        unimplemented!()
    }
}

kernel::declare_fs_type!(BS2Fat, BS2FAT_FS_TYPE);

impl KernelModule for BS2Fat {
    fn init() -> Result<Self> {
        pr_emerg!("bs2 fat in action");
        libfs_functions::register_filesystem::<Self>().map(move |_| Self)
    }
}

impl Drop for BS2Fat {
    fn drop(&mut self) {
        let _ = libfs_functions::unregister_filesystem::<Self>();
        pr_info!("bs2 fat out of action");
    }
}

struct BS2FatSuperOps {
    cluster_bits: usize, // I made up the types
    cluster_size: usize,
}

struct BS2FatFileOps;

impl FileOperations for BS2FatFileOps {
    declare_file_operations!(
        // release, // always used
        read_iter,
        write_iter,
        seek,
        ioctl,
        compat_ioctl,
        fsync,
        mmap,
        splice_read,
        splice_write,
        allocate_file
    );

    fn release(_obj: Self::Wrapper, _file: &File) {
        unimplemented!()
    }

    fn read_iter(&self, iocb: &mut Kiocb, iter: &mut IovIter) -> Result<usize> {
        libfs_functions::generic_file_read_iter(iocb, iter)
    }

    fn write_iter(&self, iocb: &mut Kiocb, iter: &mut IovIter) -> Result<usize> {
        libfs_functions::generic_file_write_iter(iocb, iter)
    }

    fn seek(&self, file: &File, offset: SeekFrom) -> Result<u64> {
        libfs_functions::generic_file_llseek(file, offset)
    }

    fn ioctl(&self, _file: &File, _cmd: &mut IoctlCommand) -> Result<i32> {
        unimplemented!()
    }

    fn compat_ioctl(&self, file: &File, cmd: &mut IoctlCommand) -> Result<i32> {
        libfs_functions::compat_ptr_ioctl(file, cmd)
    }

    fn fsync(&self, _file: &File, _start: u64, _end: u64, _datasync: bool) -> Result<u32> {
        unimplemented!()
    }

    fn mmap(&self, file: &File, vma: &mut bindings::vm_area_struct) -> Result {
        libfs_functions::generic_file_mmap(file, vma)
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

    fn allocate_file(
        &self,
        file: &File,
        mode: FileAllocMode,
        offset: bindings::loff_t,
        length: bindings::loff_t,
    ) -> Result {
        if !mode.without(FileAllocMode::KEEP_SIZE).is_empty() {
            // No support for hole punch or other fallocate flags.
            return Err(Error::EOPNOTSUPP);
        }

        let inode = file.host_inode();

        if !inode.mode().is_regular_file() {
            return Err(Error::EOPNOTSUPP);
        }

        let end_offset = offset + length;
        let sb_info: &BS2FatSuperOps = todo!(); // inode.super_block().super_ops().as_mut();
        let inode = inode.lock();
        if mode.has(FileAllocMode::KEEP_SIZE) {
            let size_on_disk = inode.i_blocks << 9;
            if end_offset <= size_on_disk as _ {
                return Ok(());
            }

            let bytes_for_file = end_offset as u64 - size_on_disk;
            let num_clusters =
                (bytes_for_file + sb_info.cluster_size as u64 - 1) >> sb_info.cluster_bits;

            for _ in 0..num_clusters {
                // Notably, these are not zeroed
                fat_add_cluster(inode.deref_mut())?;
            }

            Ok(())
        } else {
            if end_offset <= inode.size_read() {
                return Ok(());
            }

            // This is just an expanding truncate
            fat_cont_expand(inode.deref_mut(), end_offset)
        }
    }
}

fn fat_add_cluster(_inode: &mut Inode) -> Result {
    unimplemented!()
}
fn fat_cont_expand(_inode: &mut Inode, length: bindings::loff_t) -> Result {
    unimplemented!()
}
