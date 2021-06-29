#![no_std]
#![feature(allocator_api)]

use alloc::boxed::Box;
use core::{cmp::Ord, mem, ops::DerefMut, ptr};

use kernel::{
    bindings,
    buffer_head::BufferHead,
    c_types::*,
    declare_file_operations,
    file::File,
    file_operations::{
        FMode, FileAllocMode, FileOperations, FileTimeFlags, IoctlCommand, SeekFrom,
    },
    fs::{
        dentry::Dentry,
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
    // TODO: actually, these are all implemented as e.g. u16::from_le
    fn rust_helper_le16_to_cpu(x: u16) -> u16;
    fn rust_helper_le32_to_cpu(x: u32) -> u32;
    fn rust_helper_cpu_to_le16(x: u16) -> u16;
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
const SECS_PER_HOUR: i64 = 60 * 60;
const SECS_PER_DAY: i64 = 60 * 60 * 24;

// DOS dates from 1980/1/1 through 2107/12/31
const FAT_DATE_MIN: u16 = 0 << 9 | 1 << 5 | 1;
const FAT_DATE_MAX: u16 = 127 << 9 | 12 << 5 | 31;
const FAT_TIME_MAX: u16 = 23 << 11 | 59 << 5 | 29;

/// days between 1.1.70 and 1.1.80 (2 leap days)
const DAYS_DELTA: i64 = 365 * 10 + 2;
#[rustfmt::skip]
const DAYS_IN_YEAR: &[i64] = &[
    // Jan  Feb  Mar  Apr  May  Jun  Jul  Aug  Sep  Oct  Nov  Dec
    0,   0,  31,  59,  90, 120, 151, 181, 212, 243, 273, 304, 334, 0, 0, 0,
];
const YEAR_2100: i64 = 120;
const fn is_leap_year(year: i64) -> bool {
    (year & 0b11) == 0 && year != YEAR_2100
}

const FAT_ROOT_INO: u64 = 1;
const FAT_FSINFO_INO: u64 = 2;
/// start of data cluster's entry (number of reserved clusters)
const FAT_START_ENT: u32 = 2;
const MSDOS_SUPER_MAGIC: u64 = 0x4d44;
const MSDOS_NAME: usize = 11; // maximum name length

const FAT_STATE_DIRTY: u8 = 1;

const FAT12_MAX_CLUSTERS: usize = 0xff4;
const FAT16_MAX_CLUSTERS: usize = 0xfff4;

extern "C" {
    static RUST_HELPER_HZ: c_long;
    fn rust_helper_congestion_wait(sync: c_int, timeout: c_long) -> c_long;
}

struct BS2Fat;

type Cluster = u32;

// TODO: include/linux/backing-dev-defs.h, doesnt exist in bindgen
enum BLK_RW {
    ASYNC = 0,
    SYNC = 1,
}

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
        mut sb: &mut SuperBlock,
        data: Option<&mut Self::MountOptions>,
        silent: c_int,
    ) -> Result {
        enum FillSuperErrorKind {
            Invalid,
            Fail(Error),
        }
        use FillSuperErrorKind::*;

        let silent = silent == 1; // FIXME: why do we not do this in the lib callback?

        // niklas: We really want to still write to that, but also we want to allocate and error
        // early here
        // I think we should create the boxed value here, but set it later. This would require a
        // change in the SuperBlock signature, but I think it's good anyways
        // MAYBE, we could even consider a FatSuperOpsBuilder that lets you set the fields over
        // time and emits the final struct when its done
        let mut ops = Box::try_new(BS2FatSuperOps::default())?;
        // sb.set_super_operations(ops)?;

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
            // niklas, later: let's first see how this is used, maybe we can make it rust-y and
            // have a field of ops be a &(dyn InodeOperations) or so
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

            let logical_sector_size = bpb.sector_size as u64;
            // FIXME see comment above
            ops.sectors_per_cluster = bpb.sectors_per_cluster as _;

            if logical_sector_size < sb.s_blocksize {
                pr_err!(
                    "logical sector size too small for device ({})",
                    logical_sector_size
                );
                return Err(Fail(Error::EIO));
            }

            if logical_sector_size > sb.s_blocksize {
                if sb.set_blocksize(logical_sector_size as _) != 0 {
                    pr_err!("unable to set blocksize {}", logical_sector_size);
                    return Err(Fail(Error::EIO));
                }

                if let Some(bh_resize) = sb.read_block(0) {
                    libfs_functions::release_buffer(bh_resize.as_mut());
                } else {
                    pr_err!(
                        "unable to read boot sector (logical sector size {})",
                        sb.s_blocksize
                    );
                    return Err(Fail(Error::EIO));
                }
            }

            // mutex_init => TODO should be done in constructor / default
            ops.cluster_size = sb.s_blocksize as u32 * ops.sectors_per_cluster as u32;
            ops.cluster_bits = ops.cluster_size.trailing_zeros() as _; // TODO someone sanity-check please
            ops.fats = bpb.fats;
            ops.fat_bits = 0; // don't know yet
            ops.fat_start = bpb.reserved;
            ops.fat_length = bpb.fat_length;
            ops.root_cluster = 0;
            ops.free_clusters = u32::MAX; // don't know yet
            ops.free_clusters_valid = 0;
            ops.previous_free = FAT_START_ENT;
            sb.s_maxbytes = 0xffffffff;
            unsafe {
                sb.s_time_min =
                    fat_time_to_unix_time(&ops, 0, rust_helper_cpu_to_le16(FAT_DATE_MIN), 0).tv_sec;
                sb.s_time_max = fat_time_to_unix_time(
                    &ops,
                    rust_helper_cpu_to_le16(FAT_TIME_MAX),
                    rust_helper_cpu_to_le16(FAT_DATE_MAX),
                    0,
                )
                .tv_sec;
            }

            // skipping over the
            //     if (!sbi->fat_length && bpb.fat32_length) { ... }

            ops.volume_id = bpb.fat16_vol_id;
            ops.dir_per_block = (sb.s_blocksize / mem::size_of::<Bs2FatDirEntry>() as u64) as _;
            ops.dir_per_block_bits = ops.dir_per_block.trailing_zeros() as _; // TODO someone sanity check please
            ops.dir_start = ops.fat_start as usize + ops.fats as usize * ops.fat_length as usize;
            ops.dir_entries = bpb.dir_entries;

            if ops.dir_entries as i32 & (ops.dir_per_block - 1) != 0 {
                if !silent {
                    pr_err!("bogus number of directory entries ({})", ops.dir_entries);
                }
                return Err(Invalid);
            }

            let rootdir_sectors = ops.dir_entries as usize * mem::size_of::<Bs2FatDirEntry>()
                / sb.s_blocksize as usize;
            ops.data_start = ops.dir_start + rootdir_sectors;
            let total_sectors = Some(bpb.sectors)
                .filter(|&x| x != 0)
                .unwrap_or(bpb.total_sectors as _);
            let total_clusters =
                (total_sectors as usize - ops.data_start) / ops.sectors_per_cluster as usize;

            ops.fat_bits = match total_clusters {
                x if x <= FAT12_MAX_CLUSTERS => 12,
                _ => 16,
            };

            ops.dirty = (bpb.fat16_state & FAT_STATE_DIRTY) as _; // FIXME wrapper

            // check that the table doesn't overflow
            let fat_clusters = calc_fat_clusters(&sb);
            let total_clusters = total_clusters.min(fat_clusters - FAT_START_ENT as usize);
            if total_clusters > ops.max_fats() {
                if !silent {
                    pr_err!("count of clusters too big ({})", total_clusters);
                }
                return Err(Invalid);
            }
            ops.max_cluster = total_clusters + FAT_START_ENT as usize;

            if ops.free_clusters > total_clusters as u32 {
                ops.free_clusters = u32::MAX;
            }
            ops.previous_free = (ops.previous_free % ops.max_cluster as u32).max(FAT_START_ENT);

            // set up enough so that it can read an inode
            // FIXME currently, we haven't set the super ops yet, becaues we are still editing the
            // struct
            fat_hash_init(&mut sb);
            dir_hash_init(&mut sb);
            fat_ent_access_init(&mut sb);

            // TODO something about nls and codepages, let's first check whether that is important

            ops.fat_inode = Some(Inode::new(&mut sb).ok_or(Fail(Error::ENOMEM))?);
            ops.fsinfo_inode = {
                let inode = Inode::new(&mut sb).ok_or(Fail(Error::ENOMEM))?;
                inode.i_ino = FAT_FSINFO_INO;
                inode.insert_hash();
                Some(inode)
            };
            sb.s_root = {
                let mut inode = Inode::new(&mut sb).ok_or(Fail(Error::ENOMEM))?;
                inode.i_ino = FAT_ROOT_INO;
                inode.set_iversion(1);
                if let Err(e) = fat_read_root(&mut inode) {
                    inode.put();
                    return Err(Fail(e));
                }
                inode.insert_hash();
                fat_attach(&mut inode, 0);
                Dentry::make_root(&mut inode)
                    .ok_or_else(|| {
                        pr_err!("get root inode failed");
                        Fail(Error::ENOMEM)
                    })?
                    .as_ptr_mut()
            };

            // TODO something about the "discard" option

            fat_set_state(&mut sb, 1, 0);

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

            // TODO some nls things

            if let Some(mut ops) = sb.take_super_operations::<BS2FatSuperOps>() {
                unsafe {
                    if let Some(inode_ptr) = ops.fsinfo_inode.take() {
                        (*inode_ptr).put();
                    }
                    if let Some(inode_ptr) = ops.fat_inode.take() {
                        (*inode_ptr).put();
                    }
                }
                drop(ops);
            }

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

fn fat_hash_init(sb: &mut SuperBlock) {
    unimplemented!()
}
fn dir_hash_init(sb: &mut SuperBlock) {
    unimplemented!()
}
fn fat_ent_access_init(sb: &mut SuperBlock) {
    unimplemented!()
}

#[repr(C)]
struct BootSector {
    _ignored: [u8; 3],
    _system_id: [u8; 8],
    sector_size: [u8; 2],
    sec_per_clus: u8,
    reserved: u16, /* niklas: in C, this is explicitly little endian, but the type aliases for both endianneses (?) are identical */
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

fn calc_fat_clusters(sb: &SuperBlock) -> usize {
    unimplemented!()
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

fn fat_time_to_unix_time(
    sbi: &BS2FatSuperOps,
    time: u16,
    date: u16,
    time_cs: u8,
) -> bindings::timespec64 {
    let (time, date) = unsafe { (rust_helper_le16_to_cpu(time), rust_helper_le16_to_cpu(date)) };
    let year = (date >> 9) as i64;
    let month = ((date >> 5) & 0xf).max(1) as usize;
    let day = ((date & 0x1f).max(1) - 1) as i64;
    let mut leap_day = (year + 3) / 4;
    if year > YEAR_2100 {
        leap_day -= 1;
    }
    if is_leap_year(year) && month > 2 {
        leap_day += 1;
    }

    let time = time as i64;
    let mut second = (time & 0x1f) << 1;
    second += ((time >> 5) & 0x3f) * SECS_PER_MIN;
    second += (time >> 11) * SECS_PER_HOUR;
    second += (year * 365 + leap_day + DAYS_IN_YEAR[month] + day + DAYS_DELTA) * SECS_PER_DAY;
    second += sbi.timezone_offset();

    if time_cs != 0 {
        let time_cs = time_cs as i64;
        bindings::timespec64 {
            tv_sec: second + (time_cs / 100),
            tv_nsec: (time_cs % 100) * 10_000_000,
        }
    } else {
        bindings::timespec64 {
            tv_sec: second,
            tv_nsec: 0,
        }
    }
}

fn fat_read_root(root_inode: &mut Inode) -> Result {
    unimplemented!()
}
fn fat_attach(root_inode: &mut Inode, some_number: usize) {
    unimplemented!()
}
fn fat_set_state(sb: &mut SuperBlock, anumber: usize, anothernumber: usize) {
    unimplemented!()
}

struct Bs2FatDirEntry; // is this supposed to be a dentry, or an entry of a directory some other way?

#[derive(Default)]
struct BS2FatSuperOps {
    sectors_per_cluster: u16,
    cluster_bits: u16,
    cluster_size: u32,

    /// number of tables
    fats: u8,
    /// 12, 16 (, 32)
    fat_bits: u8,
    fat_start: u16,
    fat_length: u16,

    dir_start: usize,
    dir_entries: u16,

    data_start: usize,

    /// maximum cluster number
    max_cluster: usize,
    root_cluster: isize,
    previous_free: u32,
    free_clusters: u32, // C sets this to -1 sometimes, we probably want to use u32::MAX for that
    free_clusters_valid: u32,

    // niklas: Mutex around () is closest to the C way
    // if users of the guarded values _always_ lock the mutex, we can move the protected value into
    // the Mutex as one would do in Rust
    // fat_lock: Mutex<()>,
    // nfs_build_inode_lock: Mutex<()>,
    // s_lock: Mutex<()>,
    options: BS2FatMountOptions,

    /// directory entries per block
    dir_per_block: i32,
    dir_per_block_bits: i32,

    volume_id: u32,

    fat_inode: Option<*mut Inode>,
    fsinfo_inode: Option<*mut Inode>,

    /// fs state before mount
    dirty: u32,
}

// FIXME there isn't much to say, is there?
unsafe impl Send for BS2FatSuperOps {}
unsafe impl Sync for BS2FatSuperOps {}

impl BS2FatSuperOps {
    pub fn is_fat16(&self) -> bool {
        self.fat_bits == 16
    }

    pub fn max_fats(&self) -> usize {
        if self.is_fat16() {
            FAT16_MAX_CLUSTERS
        } else {
            FAT12_MAX_CLUSTERS
        }
    }
}

impl SuperOperations for BS2FatSuperOps {
    kernel::declare_super_operations!();
}

#[derive(Default)]
struct BS2FatMountOptions {
    timezone_set: bool,
    time_offset: i64,
    flush: bool,
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

    fn release(_obj: Self::Wrapper, file: &File) {
        // Assumption: Inode stems from file (! please verify); TODO:
        let inode: &mut Inode = file.inode();
        if file.fmode().has(FMode::FMODE_WRITE) && msdos_sb(inode.super_block_mut()).options.flush {
            fat_flush_inodes(inode.super_block_mut(), Some(inode), None);
            unsafe { rust_helper_congestion_wait(BLK_RW::ASYNC as _, RUST_HELPER_HZ / 10) };
        }
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

    fn fsync(&self, file: &File, start: u64, end: u64, datasync: bool) -> Result<u32> {
        // let inode: inode = file.f_mapping.host;
        // int err;

        // libfs_functions::generic_file_fsync(filp, start, end, datasync);

        // err = sync_mapping_buffers(MSDOS_SB(inode->i_sb)->fat_inode->i_mapping);
        // if (err)
        //     return err;

        // return blkdev_issue_flush(inode->i_sb->s_bdev);
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

fn msdos_sb(sb: &mut SuperBlock) -> &mut BS2FatSuperOps {
    // TODO: use own type for this void* field?
    //&*((*sb).s_fs_info as *const T)
    unsafe {
        (sb.s_fs_info as *mut BS2FatSuperOps)
            .as_mut()
            .expectk("msdos_sb in s_fs_info is null!")
    }
}

fn fat_flush_inodes(sb: &mut SuperBlock, i1: Option<&mut Inode>, i2: Option<&mut Inode>) -> Result {
    if !msdos_sb(sb).options.flush {
        return Ok(());
    }
    // TODO: return better fitting error?
    writeback_inode(i1.ok_or(Error::EINVAL)?)?;
    writeback_inode(i2.ok_or(Error::EINVAL)?)?;
    unimplemented!()
    // libfs_functions::filemap_flush(sb.s_bdev.bd_inode.i_mapping) // TODO: write block_device struct, not in bindings
}

fn writeback_inode(inode: &mut Inode) -> Result {
    /* if we used wait=1, sync_inode_metadata waits for the io for the
     * inode to finish.  So wait=0 is sent down to sync_inode_metadata
     * and filemap_fdatawrite is used for the data blocks
     */
    libfs_functions::sync_inode_metadata(inode, 0)?;
    libfs_functions::filemap_fdatawrite(inode.mapping())
}

// fn fat_alloc_clusters(inode: &mut Inode, cluster: &mut Cluster, nr_cluster: u32)
// {
//     struct super_block *sb = inode->i_sb;
//     struct msdos_sb_info *sbi = MSDOS_SB(sb);
//     const struct fatent_operations *ops = sbi->fatent_ops;
//     struct fat_entry fatent, prev_ent;
//     struct buffer_head *bhs[MAX_BUF_PER_PAGE];
//     int i, count, err, nr_bhs, idx_clus;

//     BUG_ON(nr_cluster > (MAX_BUF_PER_PAGE / 2));    /* fixed limit */
//     lock_fat(sbi);
//     if (sbi->free_clusters != -1 && sbi->free_clus_valid &&
//         sbi->free_clusters < nr_cluster) {
//         unlock_fat(sbi);
//         return -ENOSPC;
//     }

//     err = nr_bhs = idx_clus = 0;
//     count = FAT_START_ENT;
//     fatent_init(&prev_ent);
//     fatent_init(&fatent);
//     fatent_set_entry(&fatent, sbi->prev_free + 1);
//     while (count < sbi->max_cluster) {
//         if (fatent.entry >= sbi->max_cluster)
//             fatent.entry = FAT_START_ENT;
//         fatent_set_entry(&fatent, fatent.entry);
//         err = fat_ent_read_block(sb, &fatent);
//         if (err)
//             goto out;

//         /* Find the free entries in a block */
//         do {
//             if (ops->ent_get(&fatent) == FAT_ENT_FREE) {
//                 int entry = fatent.entry;

//                 /* make the cluster chain */
//                 ops->ent_put(&fatent, FAT_ENT_EOF);
//                 if (prev_ent.nr_bhs)
//                     ops->ent_put(&prev_ent, entry);

//                 fat_collect_bhs(bhs, &nr_bhs, &fatent);

//                 sbi->prev_free = entry;
//                 if (sbi->free_clusters != -1)
//                     sbi->free_clusters--;

//                 cluster[idx_clus] = entry;
//                 idx_clus++;
//                 if (idx_clus == nr_cluster)
//                     goto out;

//                 /*
//                  * fat_collect_bhs() gets ref-count of bhs,
//                  * so we can still use the prev_ent.
//                  */
//                 prev_ent = fatent;
//             }
//             count++;
//             if (count == sbi->max_cluster)
//                 break;
//         } while (fat_ent_next(sbi, &fatent));
//     }

//     /* Couldn't allocate the free entries */
//     sbi->free_clusters = 0;
//     sbi->free_clus_valid = 1;
//     err = -ENOSPC;

// out:
//     unlock_fat(sbi);
//     mark_fsinfo_dirty(sb);
//     fatent_brelse(&fatent);
//     if (!err) {
//         if (inode_needs_sync(inode))
//             err = fat_sync_bhs(bhs, nr_bhs);
//         if (!err)
//             err = fat_mirror_bhs(sb, bhs, nr_bhs);
//     }
//     for (i = 0; i < nr_bhs; i++)
//         brelse(bhs[i]);

//     if (err && idx_clus)
//         fat_free_clusters(inode, cluster[0]);

//     return err;
// }

// int fat_free_clusters(struct inode *inode, int cluster)
// {
// 	struct super_block *sb = inode->i_sb;
// 	struct msdos_sb_info *sbi = MSDOS_SB(sb);
// 	const struct fatent_operations *ops = sbi->fatent_ops;
// 	struct fat_entry fatent;
// 	struct buffer_head *bhs[MAX_BUF_PER_PAGE];
// 	int i, err, nr_bhs;
// 	int first_cl = cluster, dirty_fsinfo = 0;

// 	nr_bhs = 0;
// 	fatent_init(&fatent);
// 	lock_fat(sbi);
// 	do {
// 		cluster = fat_ent_read(inode, &fatent, cluster);
// 		if (cluster < 0) {
// 			err = cluster;
// 			goto error;
// 		} else if (cluster == FAT_ENT_FREE) {
// 			fat_fs_error(sb, "%s: deleting FAT entry beyond EOF",
// 				     __func__);
// 			err = -EIO;
// 			goto error;
// 		}

// 		if (sbi->options.discard) {
// 			/*
// 			 * Issue discard for the sectors we no longer
// 			 * care about, batching contiguous clusters
// 			 * into one request
// 			 */
// 			if (cluster != fatent.entry + 1) {
// 				int nr_clus = fatent.entry - first_cl + 1;

// 				sb_issue_discard(sb,
// 					fat_clus_to_blknr(sbi, first_cl),
// 					nr_clus * sbi->sec_per_clus,
// 					GFP_NOFS, 0);

// 				first_cl = cluster;
// 			}
// 		}

// 		ops->ent_put(&fatent, FAT_ENT_FREE);
// 		if (sbi->free_clusters != -1) {
// 			sbi->free_clusters++;
// 			dirty_fsinfo = 1;
// 		}

// 		if (nr_bhs + fatent.nr_bhs > MAX_BUF_PER_PAGE) {
// 			if (sb->s_flags & SB_SYNCHRONOUS) {
// 				err = fat_sync_bhs(bhs, nr_bhs);
// 				if (err)
// 					goto error;
// 			}
// 			err = fat_mirror_bhs(sb, bhs, nr_bhs);
// 			if (err)
// 				goto error;
// 			for (i = 0; i < nr_bhs; i++)
// 				brelse(bhs[i]);
// 			nr_bhs = 0;
// 		}
// 		fat_collect_bhs(bhs, &nr_bhs, &fatent);
// 	} while (cluster != FAT_ENT_EOF);

// 	if (sb->s_flags & SB_SYNCHRONOUS) {
// 		err = fat_sync_bhs(bhs, nr_bhs);
// 		if (err)
// 			goto error;
// 	}
// 	err = fat_mirror_bhs(sb, bhs, nr_bhs);
// error:
// 	fatent_brelse(&fatent);
// 	for (i = 0; i < nr_bhs; i++)
// 		brelse(bhs[i]);
// 	unlock_fat(sbi);
// 	if (dirty_fsinfo)
// 		mark_fsinfo_dirty(sb);

// 	return err;
// }

// int fat_chain_add(struct inode *inode, int new_dclus, int nr_cluster)
// {
// 	struct super_block *sb = inode->i_sb;
// 	struct msdos_sb_info *sbi = MSDOS_SB(sb);
// 	int ret, new_fclus, last;

// 	/*
// 	 * We must locate the last cluster of the file to add this new
// 	 * one (new_dclus) to the end of the link list (the FAT).
// 	 */
// 	last = new_fclus = 0;
// 	if (MSDOS_I(inode)->i_start) {
// 		int fclus, dclus;

// 		ret = fat_get_cluster(inode, FAT_ENT_EOF, &fclus, &dclus);
// 		if (ret < 0)
// 			return ret;
// 		new_fclus = fclus + 1;
// 		last = dclus;
// 	}

// 	/* add new one to the last of the cluster chain */
// 	if (last) {
// 		struct fat_entry fatent;

// 		fatent_init(&fatent);
// 		ret = fat_ent_read(inode, &fatent, last);
// 		if (ret >= 0) {
// 			int wait = inode_needs_sync(inode);
// 			ret = fat_ent_write(inode, &fatent, new_dclus, wait);
// 			fatent_brelse(&fatent);
// 		}
// 		if (ret < 0)
// 			return ret;
// 		/*
// 		 * FIXME:Although we can add this cache, fat_cache_add() is
// 		 * assuming to be called after linear search with fat_cache_id.
// 		 */
// //		fat_cache_add(inode, new_fclus, new_dclus);
// 	} else {
// 		MSDOS_I(inode)->i_start = new_dclus;
// 		MSDOS_I(inode)->i_logstart = new_dclus;
// 		/*
// 		 * Since generic_write_sync() synchronizes regular files later,
// 		 * we sync here only directories.
// 		 */
// 		if (S_ISDIR(inode->i_mode) && IS_DIRSYNC(inode)) {
// 			ret = fat_sync_inode(inode);
// 			if (ret)
// 				return ret;
// 		} else
// 			mark_inode_dirty(inode);
// 	}
// 	if (new_fclus != (inode->i_blocks >> (sbi->cluster_bits - 9))) {
// 		fat_fs_error(sb, "clusters badly computed (%d != %llu)",
// 			     new_fclus,
// 			     (llu)(inode->i_blocks >> (sbi->cluster_bits - 9)));
// 		fat_cache_inval_inode(inode);
// 	}
// 	inode->i_blocks += nr_cluster << (sbi->cluster_bits - 9);

// 	return 0;
// }

fn fat_add_cluster(_inode: &mut Inode) -> Result {
    // int err, cluster;

    // err = fat_alloc_clusters(inode, &cluster, 1);
    // if (err)
    //     return err;
    // /* FIXME: this cluster should be added after data of this
    //  * cluster is writed */
    // err = fat_chain_add(inode, cluster, 1);
    // if (err)
    //     fat_free_clusters(inode, cluster);
    // return err;
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
