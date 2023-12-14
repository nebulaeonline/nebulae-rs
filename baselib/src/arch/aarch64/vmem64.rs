use uefi::table::boot::MemoryType;

use crate::common::*;

use core::ptr;

// CONSTANTS

// STATICS

// THE REST

pub struct Vas {
    pub base_page_table: *mut BasePageTable,
}
impl Vas {
    pub fn new() -> Self {
        Vas {
            base_page_table: ptr::null_mut(),
        }
    }

    pub fn switch_to(&mut self) {
    }

    pub fn identity_map_based_on_memory_map(&mut self) {
        for e in unsafe { UEFI_MEMORY_MAP_1.lock().as_ref().unwrap().entries() } {
            if e.ty != MemoryType::CONVENTIONAL {
                for i in 0..e.page_count as usize {
                    let page_start = e.phys_start as usize + (i * MEMORY_DEFAULT_PAGE_USIZE);

                    // unsafe {
                    //     self.base_page_table.as_mut().unwrap().identity_map_page(
                    //         PhysAddr(page_start),
                    //         PageSize::Small,
                    //         PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);
                    //     }
                }
            }
        }
    }
}

// Page dir / table entries

#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PageSize {
    Small = USIZE_4K,
    Medium = USIZE_16K,
    Large = USIZE_64K,
}
impl PageSize {
    pub const fn into_bits(self) -> usize {
        self as _
    }

    pub const fn from_bits(value: usize) -> Self {
        match value {
            USIZE_4K => PageSize::Small,
            USIZE_16K => PageSize::Medium,
            USIZE_64K => PageSize::Large,
            _ => PageSize::Small,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct PageTable {    
    pub entries: [usize; PAGE_TABLE_MAX_ENTRIES],
}

pub type BasePageTable = PageTable;