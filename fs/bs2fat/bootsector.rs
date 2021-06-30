use core::ptr;

use kernel::{fs::super_block::SuperBlock, prelude::*, Error, Result};

use crate::{
    rust_helper_get_unaligned_le16, rust_helper_get_unaligned_le32, rust_helper_le16_to_cpu,
    rust_helper_le32_to_cpu,
};

pub const MSDOS_NAME: usize = 11; // maximum name length

#[repr(C)]
pub struct BootSector {
    pub _ignored: [u8; 3],
    pub _system_id: [u8; 8],
    pub sector_size: [u8; 2],
    pub sec_per_clus: u8,
    pub reserved: u16, /* niklas: in C, this is explicitly little endian, but the type aliases for both endianneses (?) are identical */
    pub fats: u8,
    pub dir_entries: [u8; 2],
    pub sectors: [u8; 2],
    pub media: u8,
    pub fat_length: u16,
    pub secs_track: u16,
    pub heads: u16,
    pub hidden: u32,
    pub total_sect: u32,

    // fat16
    pub drive_number: u8,
    pub state: u8,
    pub signature: u8,
    pub vol_id: [u8; 4],
    pub vol_label: [u8; MSDOS_NAME],
    pub fs_type: [u8; 8],
    // normally, this is a union with fat32 stuff, but ...
}

#[repr(C)]
#[derive(Default)]
pub struct BiosParamBlock {
    pub sector_size: u16,
    pub sectors_per_cluster: u8,
    pub reserved: u16,
    pub fats: u8,
    pub dir_entries: u16,
    pub sectors: u16,
    pub fat_length: u16,
    pub total_sectors: u32,

    pub fat16_state: u8,
    pub fat16_vol_id: u32,

    pub _fat32_length: u32,
    pub _fat32_root_cluster: u32,
    pub _fat32_info_sector: u16,
    pub _fat32_state: u8,
    pub _fat32_vol_id: u32,
}

pub fn fat_read_bpb(sb: &mut SuperBlock, b: &BootSector, silent: bool) -> Result<BiosParamBlock> {
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
