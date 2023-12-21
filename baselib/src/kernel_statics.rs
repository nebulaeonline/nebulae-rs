use uefi::table::boot::*;

// ALWAYS LOCK HIGH TO LOW WHEN TAKING MULTIPLE LOCKS;
// prefer to take locks as part of a single statement so they don't stick around
pub static mut UEFI_MEMORY_MAP_0: Option<&MemoryMap> = None;

// The rest to be eliminated

//pub static mut KERNEL_BUMP_ALLOC_5: spin::Mutex<Option<BumpAllocator>> = spin::Mutex::new(None);
pub static mut USING_FRAME_ALLOCATOR_6: spin::Mutex<bool> = spin::Mutex::new(false);
