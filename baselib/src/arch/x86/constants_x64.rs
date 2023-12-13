use crate::memory::VirtAddr;
use crate::bit::*;
use crate::constants::*;
use crate::common::Uintn;

pub const ALIGN_MASK_4K: usize = 0x000F_FFFF_FFFF_F000;
pub const ALIGN_MASK_2M: usize = 0x000F_FFFF_FFE0_0000;
pub const ALIGN_MASK_1G: usize = 0x000F_FFFF_C000_0000;

pub const KERNEL_HEAP_START: VirtAddr = VirtAddr(USIZE_4T);
pub const KERNEL_HEAP_SIZE: usize = USIZE_32M;

pub const PAGE_TABLE_MAX_ENTRIES: usize = 512;
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

pub const PAGING_PCID_CR3_MASK: usize = 0x0FFF;

pub const MEMORY_DEFAULT_PAGE_USIZE: usize = USIZE_4K;
pub const PAGING_DEFAULT_PAGE_SIZE: Uintn = SIZE_4K as Uintn;

pub const PAGING_MEM_MAX: Uintn = MAX_MEMORY - 1;
pub const PAGING_MEM_UMAX: usize = MAX_MEMORY as usize - 1;
