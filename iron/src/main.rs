#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(const_trait_impl)]
#![feature(panic_info_message)]

use uefi::prelude::*;

use baselib::genesis::*;

#[entry]
#[allow(unused_mut)]
fn uefi_start(_image_handler: uefi::Handle, mut system_table: SystemTable<Boot>) -> Status {
    
    // enjoy your trip
    kernel_prep(&mut system_table);

    // will never get here
    #[allow(unreachable_code)]
    Status::SUCCESS
}