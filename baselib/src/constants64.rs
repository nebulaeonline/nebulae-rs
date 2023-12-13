use crate::common::*;

pub const BYTE4_USIZE: usize = 0x0000_00FF_0000_0000;
pub const BYTE5_USIZE: usize = 0x0000_FF00_0000_0000;
pub const BYTE6_USIZE: usize = 0x00FF_0000_0000_0000;
pub const BYTE7_USIZE: usize = 0xFF00_0000_0000_0000;

pub const BYTE4_UINTN: Uintn = BYTE4_USIZE as Uintn;
pub const BYTE5_UINTN: Uintn = BYTE5_USIZE as Uintn;
pub const BYTE6_UINTN: Uintn = BYTE6_USIZE as Uintn;
pub const BYTE7_UINTN: Uintn = BYTE7_USIZE as Uintn;

pub const WORD3_USIZE: usize = 0x0000_FFFF_0000_0000;
pub const WORD4_USIZE: usize = 0xFFFF_0000_0000_0000;

pub const WORD3_UINTN: Uintn = WORD3_USIZE as Uintn;
pub const WORD4_UINTN: Uintn = WORD4_USIZE as Uintn;

pub const DWORD0_USIZE: usize = 0x0000_0000_FFFF_FFFF;
pub const DWORD1_USIZE: usize = 0xFFFF_FFFF_0000_0000;

pub const DWORD0_UINTN: Uintn = DWORD0_USIZE as Uintn;
pub const DWORD1_UINTN: Uintn = DWORD1_USIZE as Uintn;

pub const USIZE_4G: usize = 0x0001_0000_0000;
pub const USIZE_8G: usize = 0x0002_0000_0000;
pub const USIZE_16G: usize = 0x0004_0000_0000;
pub const USIZE_32G: usize = 0x0008_0000_0000;
pub const USIZE_64G: usize = 0x0010_0000_0000;
pub const USIZE_128G: usize = 0x0020_0000_0000;
pub const USIZE_256G: usize = 0x0040_0000_0000;
pub const USIZE_512G: usize = 0x0080_0000_0000;
pub const USIZE_1T: usize = 0x0100_0000_0000;
pub const USIZE_2T: usize = 0x0200_0000_0000;
pub const USIZE_4T: usize = 0x0400_0000_0000;
pub const USIZE_8T: usize = 0x0800_0000_0000;
pub const USIZE_16T: usize = 0x1000_0000_0000;
pub const USIZE_32T: usize = 0x2000_0000_0000;
pub const USIZE_64T: usize = 0x4000_0000_0000;
pub const USIZE_128T: usize = 0x8000_0000_0000;
pub const USIZE_256T: usize = 0x0001_0000_0000_0000;
pub const USIZE_512T: usize = 0x0002_0000_0000_0000;
pub const USIZE_1P: usize = 0x0004_0000_0000_0000;
pub const USIZE_2P: usize = 0x0008_0000_0000_0000;
pub const USIZE_4P: usize = 0x0010_0000_0000_0000;
pub const USIZE_8P: usize = 0x0020_0000_0000_0000;
pub const USIZE_16P: usize = 0x0040_0000_0000_0000;
pub const USIZE_32P: usize = 0x0080_0000_0000_0000;
pub const USIZE_64P: usize = 0x0100_0000_0000_0000;
pub const USIZE_128P: usize = 0x0200_0000_0000_0000;
pub const USIZE_256P: usize = 0x0400_0000_0000_0000;
pub const USIZE_512P: usize = 0x0800_0000_0000_0000;
pub const USIZE_1E: usize = 0x1000_0000_0000_0000;
pub const USIZE_2E: usize = 0x2000_0000_0000_0000;
pub const USIZE_4E: usize = 0x4000_0000_0000_0000;
pub const USIZE_8E: usize = 0x8000_0000_0000_0000;