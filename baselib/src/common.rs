#![allow(unused_macros)]
pub use ::uefi::prelude::*;
pub use ::uefi::table::boot::*;

pub use crate::serial_println;

pub use base::*;

pub use crate::cpu::*;
pub use crate::kalloc::*;
pub use crate::memory::*;

pub mod debug_assert {

    #[allow(unused_macros)]
    macro_rules! debug_assert {
        ( $ ( $ arg : tt ) * ) => { ... };
    }
}

pub mod priority {
    pub enum Priority {
        Lowest,
        Anonymous,
        Low,
        Normal,
        High,
        Highest,
        System,
    }

    pub enum Importance {
        DesiredButNotCritical,
        Critical,
    }
}

pub mod bit {
    // Trait(s)
    pub trait Bitmask {
        fn bitmask(&self, mask: usize) -> Self;
    }

    impl Bitmask for usize {
        #[inline(always)]
        fn bitmask(&self, mask: usize) -> Self {
            self & mask
        }
    }

    // Masking and shifting operations
    macro_rules! bit_ops {
        ($t:ty, $mod_name:ident) => {
            pub mod $mod_name {
                pub const fn bit(bit: usize) -> $t {
                    debug_assert!(
                        bit < core::mem::size_of::<$t>() * 8,
                        "Bit index out of range in bit()"
                    );
                    1 << bit
                }

                pub const fn set_bit(n: $t, bit: usize) -> $t {
                    debug_assert!(
                        bit < core::mem::size_of::<$t>() * 8,
                        "Bit index out of range in set_bit()"
                    );
                    n | (1 << bit)
                }

                pub const fn clear_bit(n: $t, bit: usize) -> $t {
                    debug_assert!(
                        bit < core::mem::size_of::<$t>() * 8,
                        "Bit index out of range in clear_bit()"
                    );
                    n & !(1 << bit)
                }

                pub const fn is_bit_set(n: $t, bit: usize) -> bool {
                    debug_assert!(
                        bit < core::mem::size_of::<$t>() * 8,
                        "Bit index out of range in is_bit_set()"
                    );
                    (n & (1 << bit)) != 0
                }

                pub const fn is_bit_clear(n: $t, bit: usize) -> bool {
                    debug_assert!(
                        bit < core::mem::size_of::<$t>() * 8,
                        "Bit index out of range in is_bit_clear()"
                    );
                    (n & (1 << bit)) == 0
                }

                pub const fn apply_mask(n: $t, mask: $t) -> $t {
                    n & mask
                }

                pub const fn create_bitmask(start: usize, end: usize) -> $t {
                    debug_assert!(
                        start <= end,
                        "Start bit should not be greater than end bit in create_bitmask()"
                    );
                    debug_assert!(
                        end < core::mem::size_of::<$t>() * 8,
                        "Bit index out of range in create_bitmask()"
                    );
                    ((1 << (end - start + 1)) - 1) << start
                }
            }
        };
    }

    bit_ops!(u32, u32bit);
    bit_ops!(u64, u64bit);
    bit_ops!(usize, ubit);

    // functions for constructing bitmap-like structures
    pub mod bitindex {
        use crate::common::base::*;

        #[inline(always)]
        pub const fn calc_bitindex_size_in_usize(capacity: usize) -> usize {
            (capacity + (usize::BITS as usize - 1)) / usize::BITS as usize
        }

        #[inline(always)]
        pub const fn calc_bitindex_size_in_bytes(capacity: usize) -> usize {
            calc_bitindex_size_in_usize(capacity) * MACHINE_UBYTES
        }

        #[inline(always)]
        pub const fn calc_bitindex_size_in_pages(capacity: usize, page_size: PageSize) -> usize {
            (calc_bitindex_size_in_bytes(capacity) + page_size.as_const_usize() - 1)
                / page_size.as_const_usize()
        }

        #[inline(always)]
        pub const fn calc_wasted_bytes(capacity: usize, page_size: PageSize) -> usize {
            (calc_bitindex_size_in_pages(capacity, page_size) * page_size.as_const_usize())
                - calc_bitindex_size_in_bytes(capacity)
        }

        #[inline(always)]
        pub const fn calc_bitindex_and_bitpos(index: usize) -> (usize, usize) {
            let index_usize = index / MACHINE_UBITS;
            let index_bit = index % MACHINE_UBITS;
            (index_usize, index_bit)
        }
    }
}

pub mod factor {
    pub const FACTOR_OF_2: u64 = 1;
    pub const FACTOR_OF_4: u64 = 2;
    pub const FACTOR_OF_8: u64 = 3;
    pub const FACTOR_OF_16: u64 = 4;
    pub const FACTOR_OF_32: u64 = 5;
    pub const FACTOR_OF_64: u64 = 6;
    pub const FACTOR_OF_128: u64 = 7;
    pub const FACTOR_OF_256: u64 = 8;
    pub const FACTOR_OF_512: u64 = 9;
    pub const FACTOR_OF_1K: u64 = 10;
    pub const FACTOR_OF_2K: u64 = 11;
    pub const FACTOR_OF_4K: u64 = 12;
    pub const FACTOR_OF_8K: u64 = 13;
    pub const FACTOR_OF_16K: u64 = 14;
    pub const FACTOR_OF_32K: u64 = 15;
    pub const FACTOR_OF_64K: u64 = 16;
    pub const FACTOR_OF_128K: u64 = 17;
    pub const FACTOR_OF_256K: u64 = 18;
    pub const FACTOR_OF_512K: u64 = 19;
    pub const FACTOR_OF_1M: u64 = 20;
    pub const FACTOR_OF_2M: u64 = 21;
    pub const FACTOR_OF_4M: u64 = 22;
    pub const FACTOR_OF_8M: u64 = 23;
    pub const FACTOR_OF_16M: u64 = 24;
    pub const FACTOR_OF_32M: u64 = 25;
    pub const FACTOR_OF_64M: u64 = 26;
    pub const FACTOR_OF_128M: u64 = 27;
    pub const FACTOR_OF_256M: u64 = 28;
    pub const FACTOR_OF_512M: u64 = 29;
    pub const FACTOR_OF_1G: u64 = 30;
    pub const FACTOR_OF_2G: u64 = 31;

    pub const UFACTOR_OF_2: usize = 1;
    pub const UFACTOR_OF_4: usize = 2;
    pub const UFACTOR_OF_8: usize = 3;
    pub const UFACTOR_OF_16: usize = 4;
    pub const UFACTOR_OF_32: usize = 5;
    pub const UFACTOR_OF_64: usize = 6;
    pub const UFACTOR_OF_128: usize = 7;
    pub const UFACTOR_OF_256: usize = 8;
    pub const UFACTOR_OF_512: usize = 9;
    pub const UFACTOR_OF_1K: usize = 10;
    pub const UFACTOR_OF_2K: usize = 11;
    pub const UFACTOR_OF_4K: usize = 12;
    pub const UFACTOR_OF_8K: usize = 13;
    pub const UFACTOR_OF_16K: usize = 14;
    pub const UFACTOR_OF_32K: usize = 15;
    pub const UFACTOR_OF_64K: usize = 16;
    pub const UFACTOR_OF_128K: usize = 17;
    pub const UFACTOR_OF_256K: usize = 18;
    pub const UFACTOR_OF_512K: usize = 19;
    pub const UFACTOR_OF_1M: usize = 20;
    pub const UFACTOR_OF_2M: usize = 21;
    pub const UFACTOR_OF_4M: usize = 22;
    pub const UFACTOR_OF_8M: usize = 23;
    pub const UFACTOR_OF_16M: usize = 24;
    pub const UFACTOR_OF_32M: usize = 25;
    pub const UFACTOR_OF_64M: usize = 26;
    pub const UFACTOR_OF_128M: usize = 27;
    pub const UFACTOR_OF_256M: usize = 28;
    pub const UFACTOR_OF_512M: usize = 29;
    pub const UFACTOR_OF_1G: usize = 30;
    pub const UFACTOR_OF_2G: usize = 31;
}

pub mod base {
    use core::mem;
    
    pub use crate::nebulae::*;
    pub use crate::sync::*;
    pub use crate::rng::isaac64::*;
    pub use crate::permissions::*;
    pub use crate::bringup::*;
    pub use super::as_usize::*;
    pub use super::bit::*;
    #[allow(unused_imports)] // not sure why this is necessary
    pub use super::debug_assert::*;
    pub use super::factor::*;
    pub use super::platform_constants::*;
    pub use super::platform_memory::*;
    pub use super::priority::*;
    pub use crate::status::*;
    pub use crate::memory::*;

    // Bring in the serial port
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub use crate::arch::x86::serial;
    #[cfg(target_arch = "aarch64")]
    pub use crate::arch::aa64::serial;
    
    // Re-export the serial port output macros
    pub use crate::serial_print;
    pub use crate::serial_println;

    // Re-export our fuse macros
    pub use crate::atomic_fuse;
    pub use crate::atomic_panic_fuse;
    pub use crate::trip_atomic_fuse;
    
    // Zero Constants
    pub const ZERO_U8: u8 = 0u8;
    pub const ZERO_U16: u16 = 0u16;
    pub const ZERO_U32: u32 = 0u32;
    pub const ZERO_U64: u64 = 0u64;
    pub const ZERO_USIZE: usize = 0usize;
    pub const ZERO_U128: u128 = 0u128;

    pub const SIZE_1B: u64 = 0x0000_0001;
    pub const SIZE_2B: u64 = 0x0000_0002;
    pub const SIZE_4B: u64 = 0x0000_0004;
    pub const SIZE_8B: u64 = 0x0000_0008;
    pub const SIZE_16B: u64 = 0x0000_0010;
    pub const SIZE_32B: u64 = 0x0000_0020;
    pub const SIZE_64B: u64 = 0x0000_0040;
    pub const SIZE_128B: u64 = 0x0000_0080;
    pub const SIZE_256B: u64 = 0x0000_0100;
    pub const SIZE_512B: u64 = 0x0000_0200;
    pub const SIZE_1K: u64 = 0x0000_0400;
    pub const SIZE_2K: u64 = 0x0000_0800;
    pub const SIZE_4K: u64 = 0x0000_1000;
    pub const SIZE_8K: u64 = 0x0000_2000;
    pub const SIZE_16K: u64 = 0x0000_4000;
    pub const SIZE_32K: u64 = 0x0000_8000;
    pub const SIZE_64K: u64 = 0x0001_0000;
    pub const SIZE_128K: u64 = 0x0002_0000;
    pub const SIZE_256K: u64 = 0x0004_0000;
    pub const SIZE_512K: u64 = 0x0008_0000;
    pub const SIZE_1M: u64 = 0x0010_0000;
    pub const SIZE_2M: u64 = 0x0020_0000;
    pub const SIZE_4M: u64 = 0x0040_0000;
    pub const SIZE_8M: u64 = 0x0080_0000;
    pub const SIZE_16M: u64 = 0x0100_0000;
    pub const SIZE_32M: u64 = 0x0200_0000;
    pub const SIZE_64M: u64 = 0x0400_0000;
    pub const SIZE_128M: u64 = 0x0800_0000;
    pub const SIZE_256M: u64 = 0x1000_0000;
    pub const SIZE_512M: u64 = 0x2000_0000;
    pub const SIZE_1G: u64 = 0x4000_0000;
    pub const SIZE_2G: u64 = 0x8000_0000;
    pub const SIZE_3G: usize = 0xC000_0000;
    pub const SIZE_4G: u64 = 0x0001_0000_0000;
    pub const SIZE_8G: u64 = 0x0002_0000_0000;
    pub const SIZE_16G: u64 = 0x0004_0000_0000;
    pub const SIZE_32G: u64 = 0x0008_0000_0000;
    pub const SIZE_64G: u64 = 0x0010_0000_0000;
    pub const SIZE_128G: u64 = 0x0020_0000_0000;
    pub const SIZE_256G: u64 = 0x0040_0000_0000;
    pub const SIZE_512G: u64 = 0x0080_0000_0000;
    pub const SIZE_1T: u64 = 0x0100_0000_0000;
    pub const SIZE_2T: u64 = 0x0200_0000_0000;
    pub const SIZE_4T: u64 = 0x0400_0000_0000;
    pub const SIZE_8T: u64 = 0x0800_0000_0000;
    pub const SIZE_16T: u64 = 0x1000_0000_0000;
    pub const SIZE_32T: u64 = 0x2000_0000_0000;
    pub const SIZE_64T: u64 = 0x4000_0000_0000;
    pub const SIZE_128T: u64 = 0x8000_0000_0000;
    pub const SIZE_256T: u64 = 0x0001_0000_0000_0000;
    pub const SIZE_512T: u64 = 0x0002_0000_0000_0000;
    pub const SIZE_1P: u64 = 0x0004_0000_0000_0000;
    pub const SIZE_2P: u64 = 0x0008_0000_0000_0000;
    pub const SIZE_4P: u64 = 0x0010_0000_0000_0000;
    pub const SIZE_8P: u64 = 0x0020_0000_0000_0000;
    pub const SIZE_16P: u64 = 0x0040_0000_0000_0000;
    pub const SIZE_32P: u64 = 0x0080_0000_0000_0000;
    pub const SIZE_64P: u64 = 0x0100_0000_0000_0000;
    pub const SIZE_128P: u64 = 0x0200_0000_0000_0000;
    pub const SIZE_256P: u64 = 0x0400_0000_0000_0000;
    pub const SIZE_512P: u64 = 0x0800_0000_0000_0000;
    pub const SIZE_1E: u64 = 0x1000_0000_0000_0000;
    pub const SIZE_2E: u64 = 0x2000_0000_0000_0000;
    pub const SIZE_4E: u64 = 0x4000_0000_0000_0000;
    pub const SIZE_8E: u64 = 0x8000_0000_0000_0000;

    pub const USIZE_1B: usize = SIZE_1B as usize;
    pub const USIZE_2B: usize = SIZE_2B as usize;
    pub const USIZE_4B: usize = SIZE_4B as usize;
    pub const USIZE_8B: usize = SIZE_8B as usize;
    pub const USIZE_16B: usize = SIZE_16B as usize;
    pub const USIZE_32B: usize = SIZE_32B as usize;
    pub const USIZE_64B: usize = SIZE_64B as usize;
    pub const USIZE_128B: usize = SIZE_128B as usize;
    pub const USIZE_256B: usize = SIZE_256B as usize;
    pub const USIZE_512B: usize = SIZE_512B as usize;
    pub const USIZE_1K: usize = SIZE_1K as usize;
    pub const USIZE_2K: usize = SIZE_2K as usize;
    pub const USIZE_4K: usize = SIZE_4K as usize;
    pub const USIZE_8K: usize = SIZE_8K as usize;
    pub const USIZE_16K: usize = SIZE_16K as usize;
    pub const USIZE_32K: usize = SIZE_32K as usize;
    pub const USIZE_64K: usize = SIZE_64K as usize;
    pub const USIZE_128K: usize = SIZE_128K as usize;
    pub const USIZE_256K: usize = SIZE_256K as usize;
    pub const USIZE_512K: usize = SIZE_512K as usize;
    pub const USIZE_1M: usize = SIZE_1M as usize;
    pub const USIZE_2M: usize = SIZE_2M as usize;
    pub const USIZE_4M: usize = SIZE_4M as usize;
    pub const USIZE_8M: usize = SIZE_8M as usize;
    pub const USIZE_16M: usize = SIZE_16M as usize;
    pub const USIZE_32M: usize = SIZE_32M as usize;
    pub const USIZE_64M: usize = SIZE_64M as usize;
    pub const USIZE_128M: usize = SIZE_128M as usize;
    pub const USIZE_256M: usize = SIZE_256M as usize;
    pub const USIZE_512M: usize = SIZE_512M as usize;
    pub const USIZE_1G: usize = SIZE_1G as usize;
    pub const USIZE_2G: usize = SIZE_2G as usize;
    pub const USIZE_3G: usize = SIZE_3G as usize;
    pub const USIZE_U32_MAX: usize = u32::MAX as usize;
    pub const USIZE_MAX: usize = usize::MAX;

    pub const BYTE0_U64: u64 = 0x0000_00FFu64;
    pub const BYTE1_U64: u64 = 0x0000_FF00u64;
    pub const BYTE2_U64: u64 = 0x00FF_0000u64;
    pub const BYTE3_U64: u64 = 0xFF00_0000u64;
    pub const BYTE4_U64: u64 = 0x0000_00FF_0000_0000u64;
    pub const BYTE5_U64: u64 = 0x0000_FF00_0000_0000u64;
    pub const BYTE6_U64: u64 = 0x00FF_0000_0000_0000u64;
    pub const BYTE7_U64: u64 = 0xFF00_0000_0000_0000u64;

    pub const BYTE0_U32: u32 = BYTE0_U64 as u32;
    pub const BYTE1_U32: u32 = BYTE1_U64 as u32;
    pub const BYTE2_U32: u32 = BYTE2_U64 as u32;
    pub const BYTE3_U32: u32 = BYTE3_U64 as u32;

    pub const BYTE0_USIZE: usize = BYTE0_U64 as usize;
    pub const BYTE1_USIZE: usize = BYTE1_U64 as usize;
    pub const BYTE2_USIZE: usize = BYTE2_U64 as usize;
    pub const BYTE3_USIZE: usize = BYTE3_U64 as usize;

    pub const WORD0_U64: u64 = 0x0000_0000_0000_FFFF;
    pub const WORD1_U64: u64 = 0x0000_0000_FFFF_0000;
    pub const WORD2_U64: u64 = 0x0000_FFFF_0000_0000;
    pub const WORD3_U64: u64 = 0xFFFF_0000_0000_0000;

    pub const WORD0_USIZE: usize = WORD0_U64 as usize;
    pub const WORD1_USIZE: usize = WORD1_U64 as usize;

    pub const DWORD0_U64: u64 = 0x0000_0000_FFFF_FFFF;
    pub const DWORD1_U64: u64 = 0xFFFF_FFFF_0000_0000;

    pub const QWORD0_U128: u128 = 0x0000_0000_0000_0000_FFFF_FFFF_FFFF_FFFF;
    pub const QWORD1_U128: u128 = 0xFFFF_FFFF_FFFF_FFFF_0000_0000_0000_0000;

    pub const fn make128(hi: usize, lo: usize) -> u128 {
        ((hi as u128) << 64) | (lo as u128)
    }

    pub const fn hi64(value: u128) -> u64 {
        ((value & QWORD1_U128) >> 64) as u64
    }

    pub const fn lo64(value: u128) -> u64 {
        (value & QWORD0_U128) as u64
    }

    pub const fn make64(hi: u32, lo: u32) -> u64 {
        ((hi as u64) << 32) | (lo as u64)
    }

    pub const fn hi32(value: u64) -> u32 {
        ((value & DWORD1_U64) >> 32) as u32
    }

    pub const fn lo32(value: u64) -> u32 {
        (value & DWORD0_U64) as u32
    }

    pub const MACHINE_UBYTES: usize = mem::size_of::<usize>();
    pub const MACHINE_BYTES: u64 = MACHINE_UBYTES as u64;

    pub const MACHINE_BITS: u64 = MACHINE_BYTES << FACTOR_OF_8;
    pub const MACHINE_UBITS: usize = MACHINE_UBYTES << UFACTOR_OF_8;

    pub const NEBULAE: u64 = 0x6464_8888_3232_5225;
    pub const NEBULAE32: u32 = 0x8888_5225;

    pub type NebulaeId = u128;
    pub const NEBULAE_ID_NOBODY: NebulaeId = ZERO_U128;
    pub const NEBULAE_ID_BASE_SYSTEM: NebulaeId = 2u128;

    #[cfg(target_pointer_width = "32")]
    pub const NEBULAE_TEST_PATTERN: usize = NEBULAE32 as usize;
    #[cfg(target_pointer_width = "64")]
    pub const NEBULAE_TEST_PATTERN: usize = NEBULAE as usize;
    
    // to create fuses, use these macros:
    #[macro_export]
    macro_rules! atomic_fuse {
        ($fuse_name:ident) => {
            pub fn $fuse_name(checking_status: bool) -> bool {
                use core::sync::atomic::{AtomicBool, Ordering};

                static mut FUSE: AtomicBool = AtomicBool::new(false);

                if checking_status {
                    unsafe { FUSE.load(Ordering::Acquire) }
                } else {
                    unsafe { FUSE.store(true, Ordering::SeqCst) };                
                    true
                }
            }
        };
    }

    // panics if the fuse has already been blown
    #[macro_export]
    macro_rules! atomic_panic_fuse {
        ($fuse_name:ident) => {
            pub fn $fuse_name(checking_status: bool) -> bool {
                use core::sync::atomic::{AtomicBool, Ordering};

                static mut FUSE: AtomicBool = AtomicBool::new(false);

                let res = unsafe { FUSE.load(Ordering::Acquire) };

                if checking_status {
                    res
                } else {
                    if res {
                        panic!("Iron fuse $fuse_name has already been blown!");
                    } else {
                        unsafe { FUSE.store(true, Ordering::SeqCst) };                
                    }
                    true
                }
            }
        };
    }

    #[macro_export]
    macro_rules! trip_atomic_fuse {
        ($fuse_name:ident) => {
            _ = $fuse_name(false);
        };
    }

    // fuses to prevent the execution of certain functions:
    
    // until we have the memory map
    atomic_panic_fuse!(memory_map_fuse);

    // until the mem frame structs are ready
    atomic_panic_fuse!(mem_frame_structs_fuse);

    // until the page info structs are ready
    atomic_panic_fuse!(page_info_structs_fuse);

    // until the frame allocator is ready
    atomic_panic_fuse!(frame_alloc_fuse);

    // until the genesis block is ready
    atomic_panic_fuse!(base_nebulae_genesis_frame_fuse);

    // until the base VAS is ready
    atomic_panic_fuse!(base_vas_fuse);
    
    // until the kernel stack is ready
    atomic_panic_fuse!(kernel_stack_fuse);

    // until the fiber subsystem is ready
    atomic_panic_fuse!(fiber_subsystem_fuse);

    // locate & return the main nebulae struct for this instance
    // currently wraps the base instance
    pub fn iron() -> Option<&'static mut Nebulae<'static>> {
        if !base_nebulae_genesis_frame_fuse(true) {
            return None;
        }

        base_nebulae_genesis_frame(None)
    }

    // numeric alignment functions
    // multiples must be powers of 2
    #[inline(always)]
    pub const fn align_up(value: usize, multiple: usize) -> usize {
        debug_assert!(multiple.is_power_of_two());
        (value + multiple - 1) & !(multiple - 1)
    }

    #[inline(always)]
    pub const fn align_down(value: usize, multiple: usize) -> usize {
        debug_assert!(multiple.is_power_of_two());
        value - (value % multiple)
    }

    #[inline(always)]
    pub const fn range_contains(range_start: usize, range_size: usize, value: usize) -> bool {
        value >= range_start && value < range_start + range_size
    } 
}

#[cfg(target_pointer_width = "32")]
pub mod platform_constants {
    use super::base::*;

    pub const ALIGN_MASK_4K: usize = 0xFFFF_F000;
    pub const ALIGN_MASK_16K: usize = 0xFFFF_C000;
    pub const ALIGN_MASK_32K: usize = 0xFFFF_8000;
    pub const ALIGN_MASK_64K: usize = 0xFFFF_0000;
    pub const ALIGN_MASK_4M: usize = 0xFFC0_0000;

    pub const KERNEL_STACK_SIZE_SMALL: usize = USIZE_256K;
    pub const KERNEL_STACK_SIZE_MED: usize = USIZE_2M;
    pub const KERNEL_STACK_SIZE_LARGE: usize = USIZE_4M;

    pub const FACTOR_OF_USIZE_BYTES: usize = UFACTOR_OF_4;
    pub const FACTOR_OF_USIZE_BITS: usize = UFACTOR_OF_32;

    pub const MAX_PHYSICAL_MEMORY: usize = USIZE_U32_MAX as usize;
}

#[cfg(target_pointer_width = "64")]
pub mod platform_constants {
    use super::base::*;

    // additional constants for 64-bit usize
    pub const BYTE4_USIZE: usize = BYTE4_U64 as usize;
    pub const BYTE5_USIZE: usize = BYTE5_U64 as usize;
    pub const BYTE6_USIZE: usize = BYTE6_U64 as usize;
    pub const BYTE7_USIZE: usize = BYTE7_U64 as usize;

    pub const WORD2_USIZE: usize = WORD2_U64 as usize;
    pub const WORD3_USIZE: usize = WORD3_U64 as usize;

    pub const DWORD0_USIZE: usize = DWORD0_U64 as usize;
    pub const DWORD1_USIZE: usize = DWORD1_U64 as usize;

    pub const USIZE_4G: usize = SIZE_4G as usize;
    pub const USIZE_8G: usize = SIZE_8G as usize;
    pub const USIZE_16G: usize = SIZE_16G as usize;
    pub const USIZE_32G: usize = SIZE_32G as usize;
    pub const USIZE_64G: usize = SIZE_64G as usize;
    pub const USIZE_128G: usize = SIZE_128G as usize;
    pub const USIZE_256G: usize = SIZE_256G as usize;
    pub const USIZE_512G: usize = SIZE_512G as usize;
    pub const USIZE_1T: usize = SIZE_1T as usize;
    pub const USIZE_2T: usize = SIZE_2T as usize;
    pub const USIZE_4T: usize = SIZE_4T as usize;
    pub const USIZE_8T: usize = SIZE_8T as usize;
    pub const USIZE_16T: usize = SIZE_16T as usize;
    pub const USIZE_32T: usize = SIZE_32T as usize;
    pub const USIZE_64T: usize = SIZE_64T as usize;
    pub const USIZE_128T: usize = SIZE_128T as usize;
    pub const USIZE_256T: usize = SIZE_256T as usize;
    pub const USIZE_512T: usize = SIZE_512T as usize;
    pub const USIZE_1P: usize = SIZE_1P as usize;
    pub const USIZE_2P: usize = SIZE_2P as usize;
    pub const USIZE_4P: usize = SIZE_4P as usize;
    pub const USIZE_8P: usize = SIZE_8P as usize;
    pub const USIZE_16P: usize = SIZE_16P as usize;
    pub const USIZE_32P: usize = SIZE_32P as usize;
    pub const USIZE_64P: usize = SIZE_64P as usize;
    pub const USIZE_128P: usize = SIZE_128P as usize;
    pub const USIZE_256P: usize = SIZE_256P as usize;
    pub const USIZE_512P: usize = SIZE_512P as usize;
    pub const USIZE_1E: usize = SIZE_1E as usize;
    pub const USIZE_2E: usize = SIZE_2E as usize;
    pub const USIZE_4E: usize = SIZE_4E as usize;
    pub const USIZE_8E: usize = SIZE_8E as usize;
    // end additional constants for 64-bit usize

    pub const ALIGN_MASK_4K: usize = 0xFFFF_FFFF_FFFF_F000;
    pub const ALIGN_MASK_16K: usize = 0xFFFF_FFFF_FFFF_C000;
    pub const ALIGN_MASK_32K: usize = 0xFFFF_FFFF_FFFF_8000;
    pub const ALIGN_MASK_64K: usize = 0xFFFF_FFFF_FFFF_0000;
    pub const ALIGN_MASK_2M: usize = 0xFFFF_FFFF_FFE0_0000;
    pub const ALIGN_MASK_1G: usize = 0xFFFF_FFFF_C000_0000;
    
    pub const KERNEL_STACK_SIZE_SMALL: usize = USIZE_512K;
    pub const KERNEL_STACK_SIZE_MED: usize = USIZE_4M;
    pub const KERNEL_STACK_SIZE_LARGE: usize = USIZE_8M;

    pub const FACTOR_OF_USIZE_BYTES: usize = UFACTOR_OF_8;
    pub const FACTOR_OF_USIZE_BITS: usize = UFACTOR_OF_64;

    pub const MAX_PHYSICAL_MEMORY: usize = USIZE_256T;
}

#[cfg(target_arch = "x86")]
pub mod platform_memory {
    use super::base::*;

    pub const ALIGN_CANON_4K: usize = 0xFFFF_F000;
    pub const ALIGN_CANON_4M: usize = 0xFFC0_0000;
    pub const MEM_CANONICAL: usize = ALIGN_CANON_4K;

    pub const PAGING_PRESENT: usize = ubit::bit(0);
    pub const PAGING_WRITEABLE: usize = ubit::bit(1);
    pub const PAGING_USERMODE: usize = ubit::bit(2);
    pub const PAGING_WRITETHROUGH: usize = ubit::bit(3);
    pub const PAGING_CACHE_DISABLE: usize = ubit::bit(4);
    pub const PAGING_ACCESSED: usize = ubit::bit(5);
    pub const PAGING_DIRTY: usize = ubit::bit(6);
    pub const PAGING_IS_PAGE_FRAME: usize = ubit::bit(7);
    pub const PAGING_IS_PAGE_FRAME_BIT: usize = 7;
    pub const PAGING_GLOBAL: usize = ubit::bit(8);
    pub const PAGING_NX: usize = ubit::bit(31);
    pub const PAGING_PCID_CR3_MASK: usize = 0x0FFF;
}

#[cfg(target_arch = "x86_64")]
pub mod platform_memory {
    use super::base::*;

    pub const ALIGN_CANON_4K: usize = if cfg!(feature = "bits52") { 0x000F_FFFF_FFFF_F000 } else { 0x0000_FFFF_FFFF_F000 };
    pub const ALIGN_CANON_2M: usize = if cfg!(feature = "bits52") { 0x000F_FFFF_FFE0_0000 } else { 0x0000_FFFF_FFE0_0000 };
    pub const ALIGN_CANON_1G: usize = if cfg!(feature = "bits52") { 0x000F_FFFF_C000_0000 } else { 0x0000_FFFF_C000_0000 };
    pub const MEM_CANONICAL: usize = ALIGN_CANON_4K;

    pub const PAGING_PRESENT: usize = ubit::bit(0);
    pub const PAGING_WRITEABLE: usize = ubit::bit(1);
    pub const PAGING_USERMODE: usize = ubit::bit(2);
    pub const PAGING_WRITETHROUGH: usize = ubit::bit(3);
    pub const PAGING_CACHE_DISABLE: usize = ubit::bit(4);
    pub const PAGING_ACCESSED: usize = ubit::bit(5);
    pub const PAGING_DIRTY: usize = ubit::bit(6);
    pub const PAGING_IS_PAGE_FRAME: usize = ubit::bit(7);
    pub const PAGING_IS_PAGE_FRAME_BIT: usize = 7;
    pub const PAGING_GLOBAL: usize = ubit::bit(8);
    pub const PAGING_NX: usize = ubit::bit(63);
    pub const PAGING_PCID_CR3_MASK: usize = 0x0FFF;
}

#[cfg(target_arch = "aarch64")]
pub mod platform_memory {

    use super::base::*;

    // CONSTANTS

    pub const ALIGN_CANON_4K: usize = if cfg!(feature = "bits52") { 0x000F_FFFF_FFFF_F000 } else { 0x0000_FFFF_FFFF_F000 };
    pub const ALIGN_CANON_2M: usize = if cfg!(feature = "bits52") { 0x000F_FFFF_FFE0_0000 } else { 0x0000_FFFF_FFE0_0000 };
    pub const ALIGN_CANON_1G: usize = if cfg!(feature = "bits52") { 0x000F_FFFF_C000_0000 } else { 0x0000_FFFF_C000_0000 };
    pub const MEM_CANONICAL: usize = ALIGN_CANON_4K;

    pub const MEMORY_DEFAULT_PAGE_USIZE: usize = USIZE_4K;
    pub const PAGING_DEFAULT_PAGE_SIZE: u64 = MEMORY_DEFAULT_PAGE_USIZE as u64;

    pub const PAGE_TABLE_MAX_ENTRIES: usize = 512;

    // placeholders from x64
    pub const PAGING_PRESENT: usize = ubit::bit(0);
    pub const PAGING_WRITEABLE: usize = ubit::bit(1);
    pub const PAGING_USERMODE: usize = ubit::bit(2);
    pub const PAGING_WRITETHROUGH: usize = ubit::bit(3);
    pub const PAGING_CACHE_DISABLE: usize = ubit::bit(4);
    pub const PAGING_ACCESSED: usize = ubit::bit(5);
    pub const PAGING_DIRTY: usize = ubit::bit(6);
    pub const PAGING_IS_PAGE_FRAME: usize = ubit::bit(7);
    pub const PAGING_IS_PAGE_FRAME_BIT: usize = 7;
    pub const PAGING_GLOBAL: usize = ubit::bit(8);
}

pub mod as_usize {
    pub trait AsUsize {
        fn as_usize(&self) -> usize;
    }

    impl AsUsize for usize {
        fn as_usize(&self) -> usize {
            self.clone()
        }
    }

    impl AsUsize for u64 {
        fn as_usize(&self) -> usize {
            *self as usize
        }
    }

    impl AsUsize for u32 {
        fn as_usize(&self) -> usize {
            *self as usize
        }
    }

    impl AsUsize for u16 {
        fn as_usize(&self) -> usize {
            *self as usize
        }
    }

    impl AsUsize for u8 {
        fn as_usize(&self) -> usize {
            *self as usize
        }
    }
}
