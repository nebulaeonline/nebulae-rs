use uefi::table::boot::*;
pub static mut UEFI_MEMORY_MAP: Option<&MemoryMap> = None;