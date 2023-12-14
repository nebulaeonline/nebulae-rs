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

pub const PAGE_TABLE_MAX_ENTRIES: usize = 512;

// placeholders from x64
pub const PAGING_PRESENT: usize = usizebit(0);
pub const PAGING_WRITABLE: usize = usizebit(1);
pub const PAGING_USERMODE: usize = usizebit(2);
pub const PAGING_WRITETHROUGH: usize = usizebit(3);
pub const PAGING_CACHE_DISABLE: usize = usizebit(4);
pub const PAGING_ACCESSED: usize = usizebit(5);
pub const PAGING_DIRTY: usize = usizebit(6);
pub const PAGING_IS_PAGE_FRAME: usize = usizebit(7);
pub const PAGING_IS_PAGE_FRAME_BIT: usize = 7;
pub const PAGING_GLOBAL: usize = usizebit(8);

pub const PAGING_NX: usize = usizebit(63);
