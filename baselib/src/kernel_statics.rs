use uefi::prelude::*;
use uefi::table::boot::*;

use crate::frame_alloc::*;
use crate::memory::*;

// ALWAYS LOCK HIGH TO LOW WHEN TAKING MULTIPLE LOCKS;
// prefer to take locks as part of a single statement so they don't stick around
pub static mut UEFI_SYSTEM_TABLE_0: spin::Mutex<Option<SystemTable<Boot>>> =
    spin::Mutex::new(None);
pub static mut UEFI_MEMORY_MAP_1: spin::Mutex<Option<MemoryMap>> = spin::Mutex::new(None);
pub static mut PHYS_MEM_MAX_2: spin::Mutex<Option<PhysAddr>> = spin::Mutex::new(None);
pub static mut PHYS_MEM_MAX_USIZE_IDX_2: spin::Mutex<Option<usize>> = spin::Mutex::new(None);
pub static mut FRAME_ALLOCATOR_3: spin::Mutex<Option<TreeAllocator>> = spin::Mutex::new(None);
pub static mut KERNEL_BASE_VAS_4: spin::Mutex<Option<Vas>> = spin::Mutex::new(None);
//pub static mut KERNEL_BUMP_ALLOC_5: spin::Mutex<Option<BumpAllocator>> = spin::Mutex::new(None);
pub static mut USING_FRAME_ALLOCATOR_6: spin::Mutex<bool> = spin::Mutex::new(false);