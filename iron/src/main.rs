// language features / compiler directives
#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(const_trait_impl)]
#![feature(panic_info_message)]

// using hw rng for entropy?
#[cfg(all(feature = "hwrngseed", target_arch = "x86"))]
use crate::arch::x86::random::*;
#[cfg(all(feature = "hwrngseed", target_arch = "x86_64"))]
use crate::arch::x86::random::*;

// common includes
use baselib::common::base::*;
// uefi bringup includes
use baselib::bringup::uefi::*;
use ::uefi::prelude::*;

// uefi bringup entry point
#[entry]
#[allow(unused_mut)]
fn uefi_start(_image_handler: ::uefi::Handle, mut system_table: SystemTable<Boot>) -> Status {
    
    // set the boot method
    unsafe { KERNEL_BOOT_METHOD = KernelBootMethod::Uefi; }

    // if cfg!(not(feature = "hwrngseed")) {
    //     // collect 8 bytes of entropy from the user
        
    // }

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_print!("nebulae::uefi_start() -> disabling uefi watchdog timer...");

    // disable the uefi watchdog timer
    let wd_result = system_table.boot_services().set_watchdog_timer(
                    0,
                    NEBULAE,
                    None,
    );

    // panic if we can't disable the watchdog timer
    if wd_result.is_err() {
        panic!("unable to disable uefi watchdog timer");
    }

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("watchdog timer disabled");

    // set the uefi system table pointer
    // -> once set, the pointer cannot be changed
    // -> once purged, the pointer cannot be retrieved
    if !uefi_system_table(Some(system_table), false).is_some() {
        panic!("uefi_start() -> unable to store uefi system table pointer");
    }
    
    // enjoy the rest of your trip!
    uefi_pre_init();

    // should not get back here
    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("fell back through to uefi_start() -> halting system");
    loop {}
    
    #[allow(unreachable_code)]
    Status::SUCCESS
}

// multiboot bringup entry point
#[allow(dead_code)]
fn multiboot_start() -> ! {
    // set the boot method
    unsafe { KERNEL_BOOT_METHOD = KernelBootMethod::Multiboot; }

    // enjoy the rest of your trip!
    //multiboot_init();

    // should never return
    loop {}
}