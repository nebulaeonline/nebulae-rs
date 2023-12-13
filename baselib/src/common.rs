pub use uefi::prelude::*;
pub use uefi::table::boot::*;

pub use crate::serial_println;

pub use crate::constants::*;

pub use crate::bit::*;
pub use crate::cpu::*;
pub use crate::util::*;
pub use crate::memory::*;
//pub use crate::kalloc::*;
#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
pub use crate::interrupts::*;

pub const KERNEL_STACK_SIZE_SMALL: usize = USIZE_256K;
pub const KERNEL_STACK_SIZE_MED: usize = USIZE_1M;
pub const KERNEL_STACK_SIZE_LARGE: usize = USIZE_8M;

// STATICS

// ALWAYS LOCK HIGH TO LOW WHEN TAKING MULTIPLE LOCKS;
// prefer to take locks as part of a single statement so they don't stick around
pub static mut UEFI_SYSTEM_TABLE_0: spin::Mutex<Option<SystemTable<Boot>>> = spin::Mutex::new(None);
pub static mut UEFI_MEMORY_MAP_1: spin::Mutex<Option<MemoryMap>> = spin::Mutex::new(None);
pub static mut PHYS_MEM_MAX_2: spin::Mutex<Option<PhysAddr>> = spin::Mutex::new(None);
pub static mut PHYS_MEM_MAX_UINTN_IDX_2: spin::Mutex<Option<usize>> = spin::Mutex::new(None);
pub static mut FRAME_ALLOCATOR_3: spin::Mutex<Option<FrameAllocator>> = spin::Mutex::new(None);
pub static mut KERNEL_BASE_VAS_4: spin::Mutex<Option<Vas>> = spin::Mutex::new(None);
//pub static mut KERNEL_BUMP_ALLOC_5: spin::Mutex<Option<BumpAllocator>> = spin::Mutex::new(None);
pub static mut USING_FRAME_ALLOCATOR_6: spin::Mutex<bool> = spin::Mutex::new(false);

// FUNCTIONS

// numeric alignment functions
// multiples must be powers of 2
#[inline(always)]
pub const fn align_up(value: usize, multiple: usize) -> usize {
    (value + multiple - 1) & !(multiple - 1)
}

#[inline(always)]
pub const fn align_down(value: usize, multiple: usize) -> usize {
    value - (value % multiple)
}
