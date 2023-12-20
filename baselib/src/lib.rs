#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(stmt_expr_attributes)]
#![feature(iter_advance_by)]
#![feature(const_trait_impl)]
#![feature(abi_x86_interrupt)]
//#![allow(incomplete_features)]
#![feature(type_alias_impl_trait)]
// #![feature(generic_const_exprs)]
#![feature(slice_ptr_get)]

// baselib mods
pub mod common;
pub mod cpu;
pub mod frame_alloc;
pub mod genesis;
pub mod interrupts;
pub mod kalloc;
pub mod kernel_statics;
pub mod memory;
pub mod panic;
pub mod status;
pub mod structures;

// baselib::arch mods
pub mod arch {
    #[cfg(target_arch = "aarch64")]
    pub mod aa64 {
        pub mod asm;
        pub mod async_exception;
        pub mod cpu;
        pub mod exception;
        pub mod serial;
        pub mod vmem64;
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub mod x86 {
        pub mod asm;
        pub mod cache_descriptor;
        pub mod cpu;
        pub mod serial;

        #[cfg(target_arch = "x86")]
        pub mod vmem32;
        #[cfg(target_arch = "x86_64")]
        pub mod vmem64;

        #[cfg(target_arch = "x86_64")]
        pub mod interrupts;
    }
}
