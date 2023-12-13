#![cfg(target_arch="aarch64")]

use crate::common::*;

// CONSTANTS

pub const ALIGN_MASK_4K: usize = 0xFFFF_FFFF_FFFF_F000;
pub const ALIGN_MASK_16K: usize = 0xFFFF_FFFF_FFFF_C000;
pub const ALIGN_MASK_64K: usize = 0xFFFF_FFFF_FFFF_0000;

pub const MEMORY_DEFAULT_PAGE_USIZE: usize = USIZE_4K;
pub const PAGING_DEFAULT_PAGE_SIZE: Uintn = SIZE_4K as Uintn;

pub const PAGING_MEM_MAX: Uintn = MAX_MEMORY - 1;
pub const PAGING_MEM_UMAX: usize = MAX_MEMORY as usize - 1;

pub const KERNEL_HEAP_START: VirtAddr = VirtAddr(USIZE_1G);
pub const KERNEL_HEAP_SIZE: usize = USIZE_32M;

pub const AARCH64_PAGE_TABLE_MAX_ENTRIES: usize = 512;
