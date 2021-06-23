#![no_std]

use core::{ops::DerefMut, ptr};

use kernel::{
    bindings,
    buffer_head::BufferHead,
    c_types::*,
    declare_file_operations,
    file::File,
    file_operations::{FileAllocMode, FileOperations, FileTimeFlags, IoctlCommand, SeekFrom},
    fs::{
        inode::{Inode, WriteSync},
        kiocb::Kiocb,
        libfs_functions,
        super_block::SuperBlock,
        super_operations::SuperOperations,
        FileSystemBase, FileSystemType,
    },
    iov_iter::IovIter,
    prelude::*,
    print::ExpectK,
    str::CStr,
    types::Mode,
    Error,
};

extern "C" {
    fn rust_helper_le16_to_cpu(x: u16) -> u16;
    fn rust_helper_le32_to_cpu(x: u32) -> u32;
    fn rust_helper_get_unaligned_le16(p: *const c_void) -> u16;
    fn rust_helper_get_unaligned_le32(p: *const c_void) -> u32;
}

module! {
    type: BS2Fat,
    name: b"bs2fat",
    author: b"Rust for Linux Contributors",
    description: b"MS-DOS filesystem support",
    license: b"GPL v2",
}

// Characters that are undesirable in an MS-DOS file name
const BAD_CHARS: &[u8] = b"*?<>|\"";
const BAD_IF_STRICT: &[u8] = b"+=,; ";

const SECS_PER_MIN: i64 = 60;
const SECS_PER_DAY: i64 = 60 * 60 * 24;
const FAT_ROOT_INO: u64 = 1;
const MSDOS_SUPER_MAGIC: u64 = 0x4d44;

struct BS2Fat;

impl FileSystemBase for BS2Fat {
    const NAME: &'static CStr = kernel::c_str!("bs2fat");
    const FS_FLAGS: c_int = (bindings::FS_REQUIRES_DEV | bindings::FS_ALLOW_IDMAP) as _;
    const OWNER: *mut bindings::module = ptr::null_mut();

    fn mount(
        _fs_type: &'_ mut FileSystemType,
        flags: c_int,
        device_name: &CStr,
        data: Option<&mut Self::MountOptions>,
    ) -> Result<*mut bindings::dentry> {
        libfs_functions::mount_bdev::<Self>(flags, device_name, data)
    }

    fn kill_super(sb: &mut SuperBlock) {
        libfs_functions::kill_block_super(sb);
    }

    fn fill_super(
        sb: &mut SuperBlock,
        data: Option<&mut Self::MountOptions>,
        silent: c_int,
    ) -> Result {
        enum FillSuperErrorKind {
            Invalid,
            Fail(Error),
        }
        use FillSuperErrorKind::*;

        let silent = silent == 1; // FIXME: why do we not do this in the lib callback?

        let ops = BS2FatSuperOps::default();
        sb.set_super_operations(ops)?;

        let res = (|| -> core::result::Result<(), FillSuperErrorKind> {
            sb.s_flags |= bindings::SB_NODIRATIME as u64;
            sb.s_magic = MSDOS_SUPER_MAGIC;

            // sb.s_export_op = &fat_export_ops; // FIXME

            sb.s_time_gran = 1;
            // sbi.nfs_build_inode_lock = Mutex::ratelimit_state_init(ops.ratelimit, DEFAULT_RATELIMIT_INTERVAL, DEFAULT_RATELIMIT_BURST); // FIXME
            // parse_options(
            //     sb,
            //     data,
            //     false, /* is_vfat */
            //     silent,
            //     &debug,
            //     ops.options,
            // )
            // .map_err(Fail)?;

            // niklas: C calls the given "setup" here, I inlined that
            // MSDOS_SB(sb)->dir_ops = &msdos_dir_inode_operations; // TODO This should be done in BS2FatSuperOps::default()
            // sb.set_dentry_operations::<BS2FatDentryOps>();
            sb.s_flags |= bindings::SB_NOATIME as u64;

            sb.set_min_blocksize(512);
            let buffer_head = sb
                .read_block(0)
                .ok_or_else(|| {
                    pr_err!("unable to read boot sector");
                    Fail(Error::EIO)
                })?
                .as_mut();
            let boot_sector = unsafe {
                buffer_head
                    .b_data
                    .cast::<BootSector>()
                    .as_mut()
                    .expectk("buffer data was NULL")
            };
            let bpb = fat_read_bpb(sb, boot_sector, silent);
            // niklas: I (for now) chose not to add the floppy disk thingy here :)
            libfs_functions::release_buffer(buffer_head);
            let bpb = bpb.map_err(|e| if e == Error::EINVAL { Invalid } else { Fail(e) })?;

            // <snip>

            Ok(())
        })();

        res.map_err(|x| {
            let error_val = match x {
                Invalid => {
                    if !silent {
                        // TODO: what is fat_msg? sb is given to it too ...
                        pr_info!("Can't find a valid FAT filesystem");
                    }
                    Error::EINVAL
                }
                Fail(e) => e,
            };

            // TODO do things after out_fail

            error_val
        })
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

const MSDOS_NAME: usize = 11; // maximum name length

#[repr(C)]
struct BootSector {
    _ignored: [u8; 3],
    _system_id: [u8; 8],
    sector_size: [u8; 2],
    sec_per_clus: u8,
    reserved: u16, // niklas: in C, this is explicitly little endian, but the type aliases for both endianneses (?) are identical
    fats: u8,
    dir_entries: [u8; 2],
    sectors: [u8; 2],
    media: u8,
    fat_length: u16,
    secs_track: u16,
    heads: u16,
    hidden: u32,
    total_sect: u32,

    // fat16
    drive_number: u8,
    state: u8,
    signature: u8,
    vol_id: [u8; 4],
    vol_label: [u8; MSDOS_NAME],
    fs_type: [u8; 8],
    // normally, this is a union with fat32 stuff, but ...
}

#[repr(C)]
#[derive(Default)]
struct BiosParamBlock {
    sector_size: u16,
    sectors_per_cluster: u8,
    reserved: u16,
    fats: u8,
    dir_entries: u16,
    sectors: u16,
    fat_length: u16,
    total_sectors: u32,

    fat16_state: u8,
    fat16_vol_id: u32,

    _fat32_length: u32,
    _fat32_root_cluster: u32,
    _fat32_info_sector: u16,
    _fat32_state: u8,
    _fat32_vol_id: u32,
}

fn fat_read_bpb(sb: &mut SuperBlock, b: &BootSector, silent: bool) -> Result<BiosParamBlock> {
    let bpb = unsafe {
        BiosParamBlock {
            sector_size: rust_helper_get_unaligned_le16(ptr::addr_of!(b.sector_size).cast()),
            sectors_per_cluster: b.sec_per_clus,
            reserved: rust_helper_le16_to_cpu(b.reserved),
            fats: b.fats,
            dir_entries: rust_helper_get_unaligned_le16(ptr::addr_of!(b.dir_entries).cast()),
            sectors: rust_helper_get_unaligned_le16(ptr::addr_of!(b.sectors).cast()),
            fat_length: rust_helper_le16_to_cpu(b.fat_length),
            total_sectors: rust_helper_le32_to_cpu(b.total_sect),

            fat16_state: b.state,
            fat16_vol_id: rust_helper_get_unaligned_le32(ptr::addr_of!(b.vol_id).cast()),
            ..Default::default()
        }
    };

    if bpb.reserved == 0 {
        if !silent {
            pr_err!("bogus number of reserved sectors");
        }

        return Err(Error::EINVAL);
    }

    if bpb.fats == 0 {
        if !silent {
            pr_err!("bogus number of FAT structure");
        }

        return Err(Error::EINVAL);
    }

    if !(0xf8 <= b.media || b.media == 0xf0) {
        if !silent {
            pr_err!("invalid media value ({:#x})", b.media);
        }
        return Err(Error::EINVAL);
    }

    if !bpb.sector_size.is_power_of_two() || bpb.sector_size < 512 || bpb.sector_size > 4096 {
        if !silent {
            pr_err!("bogus logical sector size {}", bpb.sector_size);
        }
        return Err(Error::EINVAL);
    }

    if !bpb.sectors_per_cluster.is_power_of_two() {
        if !silent {
            pr_err!("bogus sectors per cluster {}", bpb.sectors_per_cluster);
        }
        return Err(Error::EINVAL);
    }

    if bpb.fat_length == 0 {
        // FIXME: C also checks a fat32 thing here
        if !silent {
            pr_err!("bogus number of FAT sectors");
        }
        return Err(Error::EINVAL);
    }

    Ok(bpb)
}

#[derive(Default)]
struct BS2FatSuperOps {
    cluster_bits: u16,
    cluster_size: usize,
    options: BS2FatMountOptions,
    // nfs_build_inode_lock: Mutex,
}

impl SuperOperations for BS2FatSuperOps {
    kernel::declare_super_operations!();
}

#[derive(Default)]
struct BS2FatMountOptions {
    timezone_set: bool,
    time_offset: i64,
}

impl BS2FatSuperOps {
    pub fn timezone_offset(&self) -> i64 {
        let minutes = if self.options.timezone_set {
            -self.options.time_offset
        } else {
            unsafe { bindings::sys_tz }.tz_minuteswest as _
        };
        minutes * SECS_PER_MIN
    }
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
            pr_emerg!("since fat_add_cluster is not implemented, this isn't gonna end well");
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

fn fat_cont_expand(inode: &mut Inode, size: bindings::loff_t) -> Result {
    libfs_functions::generic_cont_expand_simple(inode, size)?;
    fat_truncate_time(
        inode,
        None,
        FileTimeFlags::empty()
            .with(FileTimeFlags::C)
            .with(FileTimeFlags::M),
    );
    inode.mark_dirty();

    if !inode.is_sync() {
        return Ok(());
    }

    // niklas: This is odd, they only use count as start + count - 1 which is just size - 1
    let start = inode.i_size;
    let count = size - inode.i_size;
    let mapping = inode.i_mapping;

    // Opencode syncing since we don't have a file open to use standard fsync path.
    libfs_functions::filemap_fdatawrite_range(mapping, start, start + count - 1)
        .and(libfs_functions::sync_mapping_buffers(mapping))
        .and(inode.write_now(WriteSync::Yes))
        .and_then(|()| libfs_functions::filemap_fdatawait_range(mapping, start, start + count - 1))
}

/// truncate the various times with appropriate granularity:
///   root inode:
///     all times always 0
///   all other inodes:
///     mtime - 2 seconds
///     ctime
///       msdos - 2 seconds
///       vfat  - 10 milliseconds // niklas: we don't care
///     atime - 24 hours (00:00:00 in local timezone)
fn fat_truncate_time(inode: &mut Inode, now: Option<bindings::timespec64>, flags: FileTimeFlags) {
    if inode.i_ino == FAT_ROOT_INO {
        return;
    }

    // niklas: I changed the signature to take `now` by value, because we only read from it anyways
    let now = now.unwrap_or_else(|| inode.current_time());

    if flags.has(FileTimeFlags::A) {
        let sb_info: &BS2FatSuperOps = todo!(); // see allocate file
        let tz_offset = sb_info.timezone_offset();
        let seconds = now.tv_sec - tz_offset;
        let seconds = seconds + tz_offset - (seconds % SECS_PER_DAY);
        inode.i_atime = bindings::timespec64 {
            tv_sec: seconds,
            tv_nsec: 0,
        };
    }
    if flags.has(FileTimeFlags::C) {
        // niklas: I didn't bother to add the check for vfat
        inode.i_ctime = fat_timespec64_trunc_2secs(now);
    }
    if flags.has(FileTimeFlags::M) {
        inode.i_mtime = fat_timespec64_trunc_2secs(now);
    }
}

fn fat_timespec64_trunc_2secs(ts: bindings::timespec64) -> bindings::timespec64 {
    bindings::timespec64 {
        tv_sec: ts.tv_sec & !0b1,
        tv_nsec: 0,
    }
}
