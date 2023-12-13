#![no_std]
#![feature(abi_x86_interrupt)]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(const_trait_impl)]
#![feature(stmt_expr_attributes)]
#![feature(iter_advance_by)]

pub mod constants;
#[cfg(target_pointer_width = "64")]
pub mod constants64;
pub mod common;
pub mod debug_assert;
pub mod bit;
pub mod cpu;
pub mod memory;
pub mod bitmap;
pub mod frame_alloc;
pub mod kalloc;
pub mod interrupts;
pub mod status;
pub mod util;
pub mod panic;

pub mod arch {
    #[cfg(target_arch = "aarch64")]
    pub mod aarch64 {
        pub mod asm;
        pub mod serial;
        pub mod cpu;
        pub mod async_exception;
        pub mod exception;

        #[cfg(target_arch = "aarch64")]
        pub mod constants;
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub mod x86 {
        pub mod asm;
        pub mod serial;
        pub mod cpu;
        pub mod cache_descriptor;

        #[cfg(target_arch = "x86")]
        pub mod constants_x32;
        #[cfg(target_arch = "x86")]
        pub mod vmem32;

        #[cfg(target_arch = "x86_64")]
        pub mod constants_x64;
        #[cfg(target_arch = "x86_64")]
        pub mod vmem64;
        
        #[cfg(target_arch = "x86_64")]
        pub mod interrupts;                
    }
}
