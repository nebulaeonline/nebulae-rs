#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(const_trait_impl)]
#![feature(panic_info_message)]

use core::fmt::Write;

use uefi::prelude::*;

use baselib::common::base::*;
use baselib::common::kernel_statics::*;
use baselib::common::naughty::*;
use baselib::cpu::*;
use baselib::frame_alloc::FrameAllocator;

#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
use baselib::interrupts::*;

#[entry]
#[allow(unused_mut)]
fn uefi_start(_image_handler: uefi::Handle, mut system_table: SystemTable<Boot>) -> Status {
    // mini-contexts to ensure the spinlocks are dropped

    // set up the global uefi table (early boot use) and disable the watchdog timer
    {
        #[cfg(debug_assertions)]
        serial_println!("beginning uefi_start() init");

        // write to our global
        let mut st = unsafe { UEFI_SYSTEM_TABLE_0.lock() };
        *st = unsafe { Some(system_table.unsafe_clone()) };

        #[cfg(debug_assertions)]
        serial_println!("disabling watchdog");

        // disable uefi watchdog timer
        _ = (*st)
            .as_ref()
            .unwrap()
            .boot_services()
            .set_watchdog_timer(0, 65536, None);

        // say our hello; this will be our last display output until driver / framebuffer
        _ = writeln!((*st).as_mut().unwrap().stdout(), "Hello :)");
    }

    #[cfg(debug_assertions)]
    serial_println!("beginning physical frame allocator init");

    // initialize the physical frame allocator
    {
        let _frame_allock_lock = unsafe { USING_FRAME_ALLOCATOR_6.lock() };

        let mut frame_alloc = unsafe { FRAME_ALLOCATOR_3.lock() };
        *frame_alloc = Some(FrameAllocator::new());
        (*frame_alloc).as_mut().unwrap().init();
    }

    #[cfg(debug_assertions)]
    serial_println!("physical frame allocator initialized");

    kernel_main();

    // will never get here
    #[allow(unreachable_code)]
    Status::SUCCESS
}

fn kernel_main() -> () {
    // init exception handling
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    exceptions_init();

    // initialize the kernel's virtual address space
    {
        let mut kernel_vas = unsafe { KERNEL_BASE_VAS_4.lock() };
        *kernel_vas = Some(Vas::new());
        (*kernel_vas).as_mut().unwrap().base_page_table =
            addr_to_ptr_mut::<BasePageTable, PhysAddr>(BasePageTable::new_base());
        (*kernel_vas)
            .as_mut()
            .unwrap()
            .identity_map_based_on_memory_map();
        _ = (*kernel_vas).as_mut().unwrap().init_cr3();
        (*kernel_vas).as_mut().unwrap().switch_to();
        #[cfg(debug_assertions)]
        serial_println!("Kernel VAS initialized");
    }

    // see how many free pages we have after bootstrapping the memory manager
    {
        // USING_FRAME_ALLOCATOR_6 is not required here because this is a non-mutable reference
        let free_pages = unsafe { FRAME_ALLOCATOR_3.lock().as_ref().unwrap().free_page_count() };
        serial_println!(
            "Free pages: {} / {} KB",
            free_pages,
            free_pages << UFACTOR_OF_4K
        );
    }

    // halt
    #[cfg(debug_assertions)]
    serial_println!("Halting in kernel_main()");
    wait_forever();
}
