#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(stmt_expr_attributes)]
#![feature(iter_advance_by)]
#![feature(const_trait_impl)]
#![feature(abi_x86_interrupt)]
//#![allow(incomplete_features)]
#![feature(type_alias_impl_trait)]
#![feature(slice_ptr_get)]
#![feature(const_for)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![feature(strict_provenance)]

// baselib mods
pub mod bringup;
pub mod permissions;
pub mod common;
pub mod cpu;
pub mod rng;
pub mod sync;
pub mod frame_alloc;
pub mod nebulae;
pub mod vmem;
pub mod kalloc;
pub mod memory;
pub mod panic;
pub mod status;
pub mod structures;

// baselib::arch mods
pub mod arch {
    #[cfg(target_arch = "aarch64")]
    pub mod aa64 {
        pub mod asm;
        pub mod cpu;
        pub mod serial;
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub mod x86 {
        pub mod asm;
        pub mod cache_descriptor;
        pub mod cpu;
        pub mod random;
        pub mod serial;        
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn test_runner(tests: &[&dyn Fn()]) {
        serial_println!("running {} tests", tests.len());
        for test in tests {
            test();
        }
    }
}