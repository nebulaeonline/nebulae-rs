// Essentials
use crate::serial_println;
// Core Rust Items
use core::mem;
use core::sync::atomic::{AtomicBool, Ordering};
use core::fmt::Write;
// External Items
use uefi::prelude::*;
use uefi::table::boot::*;
// Internal Items
use crate::kernel_statics::*;
use crate::structures::bitmap::*;
use crate::common::base::*;
use crate::cpu::*;
use crate::frame_alloc::*;
#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
use crate::interrupts::*;

#[repr(C)]
pub struct Nebulae<'a> {
    pub magic: usize,
    pub genesis_ptr: Option<&'a mut Nebulae<'a>>,
    pub conv_pages: usize,
    pub total_pages: usize,
    pub phys_mem_max: PhysAddr,
    pub page_info: Option<*mut [pages::PageInfo]>,
    pub page_info_max: usize,
    pub mem_regions: Option<*mut [MemRegionDescr]>,
    pub mem_regions_max: usize,
    pub uefi_mem_map_0_1: Option<MemoryMap<'a>>,
    pub frame_alloc_internal_0_2: spin::Mutex<Option<TreeAllocator>>,
    pub frame_alloc_0_3: spin::Mutex<bool>,
    pub base_vas_internal_0_4: spin::Mutex<bool>,
    pub base_vas_0_5: spin::Mutex<Option<Vas>>,
}
impl<'a> Nebulae<'a> {
    
    // no better time than now
    pub fn new(neb: fn(usize) -> usize) -> Nebulae<'a> {
        
        Nebulae {
            magic: neb(NEBULAE),
            genesis_ptr: None,
            conv_pages: 0,
            total_pages: 0,
            phys_mem_max: 0usize.as_phys(),
            page_info: None,
            page_info_max: 0,
            mem_regions: None,
            mem_regions_max: 0,
            uefi_mem_map_0_1: None,
            frame_alloc_internal_0_2: spin::Mutex::new(None),
            frame_alloc_0_3: spin::Mutex::new(false),
            base_vas_internal_0_4: spin::Mutex::new(false),
            base_vas_0_5: spin::Mutex::new(None),
        }
    }
}

// This is our genesis block function. For children,
// you may use any function that obeys the signature.
fn nebulae(let_the_magic_in: usize) -> usize {
    let kernel = let_the_magic_in;
    kernel
}

// Here. Goes. Nothing.
pub fn kernel_main(n: *mut Nebulae) {
    serial_println!("kernel_main() called with new stack: {}", unsafe { (*n).magic });

}

// This function preps what needs to be done to run the kernel
// It is called from uefi_start() in iron/src/main.rs
// It preps the things needed for regular kernel operation
// At the end, it sets up a fresh stack and calls kernel_main()
pub fn kernel_prep(st: &mut SystemTable<Boot>) {
    // Function guard - atomic fuse
    static mut FUSE: AtomicBool = AtomicBool::new(false);

    if unsafe { FUSE.load(Ordering::SeqCst) } {
        return;
    } else {
        unsafe {
            FUSE.store(true, Ordering::SeqCst);
        }
    }
    
    //-----------------------------------------------------------------------------------
    
    // init begin

    // make sure st is not null
    // if st.is_null() {
    //     panic!("uefi system table pointer is null");
    // }

    #[cfg(debug_assertions)]
    serial_println!("beginning uefi_start() init");

    #[cfg(debug_assertions)]
    serial_println!("disabling watchdog");

    // disable uefi watchdog timer
    let wd_result = st.boot_services().set_watchdog_timer(
                    0,
                    NEBULAE as u64 + u16::MAX as u64,
                        None,
                );

    // panic if we can't disable the watchdog timer
    if wd_result.is_err() {
        panic!("Failed to disable UEFI watchdog timer");
    }

    #[cfg(debug_assertions)]
    serial_println!("watchdog disabled");

    // say our hello; this will be our last display output until display driver / framebuffer
    _ = writeln!(st.stdout(), "The nebulae says hello :)");

    
    //-----------------------------------------------------------------------------------

    // memory map

    #[cfg(debug_assertions)]
    serial_println!("beginning genesis block init");

    let mut largest_conv_block: PhysAddr = usize::MIN.as_phys();
    let mut largest_conv_block_size = 0usize;
    let mut smallest_conv_block: PhysAddr = usize::MAX.as_phys();
    let mut smallest_conv_block_size = usize::MAX;
    let mut conv_page_count: usize = 0usize;
    let mut max_phys_present: usize = 0;

    // if the largest conventional block
    // is page_aligned, then we will pull the
    // last page away from the smallest
    // conventional block. If it is not page
    // aligned, then we will pull the first page
    // from the largest block.  In the event
    // the largest and smallest blocks are the same,
    // we'll just take the very last frame.
    // This will be our genesis block.
    #[cfg(debug_assertions)]
    serial_println!("allocating memory for uefi memory map");

    // Get the memory map
    let mut mm: MemoryMap;
    
    // query for the size of the memory map
    let mm_size_struct = st
        .boot_services()
        .memory_map_size();

    // do some size calcs
    let (mut mm_size_in_pages, _) = pages::bytes_to_pages(mm_size_struct.map_size, PageSize::Small);
    
    // we are going to do some allocations, and uefi always seems
    // to report a smaller map than exit_boot_services() needs anyway,
    // let's allocate an extra page if there's not 25 entries free
    if pages::pages_to_bytes(mm_size_in_pages, PageSize::Small) - mm_size_struct.map_size < 25 * mem::size_of::<MemoryDescriptor>() {
        mm_size_in_pages += 1;
    }
    
    #[cfg(debug_assertions)]
    serial_println!("allocating {} page(s) for memory map", mm_size_in_pages);

    // Allocate memory for the memory map
    let mm_alloc_result = st
        .boot_services().allocate_pages(
                AllocateType::MaxAddress(SIZE_2G),
                MemoryType::custom(MEMORY_TYPE_UEFI_MEM_MAP), // Use the custom memory type
                mm_size_in_pages,
            );

    // panic if we can't allocate memory for the memory map
    if mm_alloc_result.is_err() {
        panic!("Failed to allocate memory for UEFI memory map");
    } else {
        // allocation via uefi was successful, so get the memory map

        // store the base address of our allocated memory
        let buf_addr = mm_alloc_result.unwrap().as_phys();

        // zero the memory to be used for the memory map
        raw::memset_size_aligned(buf_addr, pages::pages_to_bytes(mm_size_in_pages, PageSize::Small), 0);

        // yes, these buffers point to the same place-
        // but sometimes uefi is finicky and we need to try twice.
        // I don't want to actually allocate twice, so we'll just
        // use the same buffer twice.
        let buf = raw::abracadabra_array::<u8>(buf_addr, pages::pages_to_bytes(mm_size_in_pages, PageSize::Small));
        let buf2 = raw::abracadabra_array::<u8>(buf_addr, pages::pages_to_bytes(mm_size_in_pages, PageSize::Small));

        let uefi_result = 
            st.boot_services().memory_map(unsafe { buf.as_mut().unwrap() });

        if uefi_result.is_err() {
            // try one more time to get the map
            // clean up the buffer first
            raw::memset_size_aligned(buf_addr, pages::pages_to_bytes(mm_size_in_pages, PageSize::Small), 0);
            
            let uefi_try2 = 
                st.boot_services().memory_map(unsafe { buf2.as_mut().unwrap() });

            if uefi_try2.is_err() {
                panic!("Failed to get UEFI memory map");
            } else {
                mm = uefi_try2.unwrap();
            }
        } else {
            mm = uefi_result.unwrap();
        }
    }

    // We have the memory map, so let's process it
    for e in mm.entries() {
        
        // output mm in debug mode
        #[cfg(debug_assertions)]
        serial_println!(
            "ty: {:?} ps: 0x{:08x} pc: {} flags: {:?}",
            e.ty,
            e.phys_start,
            e.page_count,
            e.att
        );

        // we are only interested in conventional memory
        // for deciding on a genesis block
        if e.ty == MemoryType::CONVENTIONAL {
            max_phys_present = e.phys_start as usize + pages::pages_to_bytes(e.page_count as usize, PageSize::Small);

            conv_page_count += e.page_count.as_usize();
            let pb = pages::pages_to_bytes(e.page_count.as_usize(), PageSize::Small);
            if pb > largest_conv_block_size {
                largest_conv_block = e.phys_start.as_phys();
                largest_conv_block_size = pb;
            }
            if pb < smallest_conv_block_size {
                smallest_conv_block = e.phys_start.as_phys();
                smallest_conv_block_size = pb;
            }
        }
    }

    #[cfg(debug_assertions)]
    serial_println!("max conventional address: 0x{:0x}", max_phys_present);

    #[cfg(debug_assertions)]
    serial_println!("beginning memory allocation for physical frame allocator");

    //-----------------------------------------------------------------------------------
    
    // physical allocator

    // see how many pages we need to track in the physical frame allocator
    let phys_range_page_count = pages::calc_pages_reqd(max_phys_present, PageSize::Small);

    // page info structs for every page of physical memory in the system
    let page_info_pages_reqd = pages::calc_pages_reqd(
        mem::size_of::<pages::PageInfo>() * phys_range_page_count,
        PageSize::Small);

    // the bitmap needs to cover conv_page_count potential nodes in the tree (most degraded case)
    // the bitmap indicates free or used spots in the mem region descriptor array
    // mem regions are managed by the frame allocator
    let bitmap_pages_reqd = 
        bitindex::calc_bitindex_size_in_pages(phys_range_page_count, PageSize::Small);
    
    // mem region descriptor structs to cover conv_page_count nodes (again, most degraded case)
    let storage_pages_reqd = pages::calc_pages_reqd(
        phys_range_page_count * core::mem::size_of::<MemRegionDescr>(),
        PageSize::Small);

    #[cfg(debug_assertions)]
    serial_println!(
        "physical frame allocator requires {} pages",
        bitmap_pages_reqd + page_info_pages_reqd + storage_pages_reqd
    );

    #[cfg(debug_assertions)]
    serial_println!("allocating memory for physical frame allocator bitmap");

    // Allocate memory for the bitmap using UEFI allocate_pages().
    let bitmap_alloc_result = st
            .boot_services()
            .allocate_pages(
                AllocateType::MaxAddress(SIZE_2G),
                MemoryType::custom(MEMORY_TYPE_BOOT_FRAMER_BITMAP), // Use the custom memory type
                bitmap_pages_reqd);
    
    if bitmap_alloc_result.is_err() {
        panic!("Failed to allocate memory for physical frame allocator tree bitmap");
    } else {
        #[cfg(debug_assertions)]
        serial_println!("physical frame allocator bitmap allocated");
    }

    #[cfg(debug_assertions)]
    serial_println!("allocating memory for memory info structs");

    // Allocate for page info
    let page_info_alloc_result = st
            .boot_services()
            .allocate_pages(
                AllocateType::MaxAddress(SIZE_2G),
                MemoryType::custom(MEMORY_TYPE_MEMORY_SUBSYSTEM), // Use the custom memory type
                page_info_pages_reqd,
    );

    if page_info_alloc_result.is_err() {
        panic!("Failed to allocate memory for memory info");
    } else {
        #[cfg(debug_assertions)]
        serial_println!("memory allocated for memory info structs");        
    }

    #[cfg(debug_assertions)]
    serial_println!("allocating memory for physical frame allocator node storage");

    // Allocate memory for node storage.
    let node_storage_alloc_result = st
            .boot_services()
            .allocate_pages(
                AllocateType::MaxAddress(SIZE_2G),
                MemoryType::custom(MEMORY_TYPE_MEMORY_SUBSYSTEM), // Use the custom memory type
                storage_pages_reqd,
            );

    if node_storage_alloc_result.is_err() {
        panic!("Failed to allocate memory for physical frame allocator node storage");
    }

    #[cfg(debug_assertions)]
    serial_println!("memory allocated for physical frame allocator");

    // now it's time to exit boot services and re-process our memory map

    #[cfg(debug_assertions)]
    serial_println!("terminating uefi services");
    
    (_, mm) = unsafe { st.unsafe_clone() }.exit_boot_services(MemoryType::custom(MEMORY_TYPE_UEFI_MEM_MAP));

    #[cfg(debug_assertions)]
    serial_println!("uefi services terminated");

    //-----------------------------------------------------------------------------------

    // genesis block

    // calculate the location of the genesis block; we need to re-process the
    // new memory map since we did a bunch of allocations just a moment ago
    for e in mm.entries() {

        // we are only interested in conventional memory
        // for deciding on a genesis block
        if e.ty == MemoryType::CONVENTIONAL {
            let pb = pages::pages_to_bytes(e.page_count.as_usize(), PageSize::Small);
            if pb > largest_conv_block_size {
                largest_conv_block = e.phys_start.as_phys();
                largest_conv_block_size = pb;
            }
            if pb < smallest_conv_block_size {
                smallest_conv_block = e.phys_start.as_phys();
                smallest_conv_block_size = pb;
            }
        }
    }

    // are the smallest and largest the same block?
    let genesis_block = if largest_conv_block == smallest_conv_block {
        // if so, then we can just grab the last page
        PhysAddr(align_down(
            largest_conv_block.as_usize() + largest_conv_block_size - 1,
            MEMORY_DEFAULT_PAGE_USIZE,
        ))
    } else {
        // otherwise, we need to see if the largest block's base address is page aligned
        // if it is, then we need to grab the last page from the smallest block
        // if it isn't, then we can grab the first page from the largest block
        if largest_conv_block.is_aligned(PageSize::Small.into_bits()) {
            PhysAddr(align_down(
                smallest_conv_block.as_usize() + smallest_conv_block_size - 1,
                MEMORY_DEFAULT_PAGE_USIZE,
            ))
        } else {
            PhysAddr(
                align_down(largest_conv_block.as_usize(), MEMORY_DEFAULT_PAGE_USIZE)
            )
        }
    };
    
    #[cfg(debug_assertions)]
    serial_println!("genesis block location determined: 0x{:016x}", genesis_block);

    // instantiate the genesis block
    let gb_base = raw::abracadabra::<Nebulae>(genesis_block);
    unsafe { *gb_base = Nebulae::new(nebulae) };
    let gb = unsafe { gb_base.as_mut().unwrap() };

    // set the basic kernel table parameters
    gb.magic = NEBULAE;
    gb.genesis_ptr = Some(raw::raw_to_static_ref_mut::<Nebulae, PhysAddr>(PhysAddr::from(
        gb as *mut Nebulae as usize),
    ));
    gb.conv_pages = conv_page_count;
    gb.total_pages = phys_range_page_count;
    gb.phys_mem_max = PhysAddr(max_phys_present.as_usize());

    //-----------------------------------------------------------------------------------

    // memory structures init

    #[cfg(debug_assertions)]
    serial_println!("beginning base allocator config");

    // write the page info structs with default info

    // get the physical address of the buffer allocated for page info structs
    let pageinfo_addr = page_info_alloc_result.unwrap().as_phys();
    
    #[cfg(debug_assertions)]
    serial_println!("zeroing memory for page info structs");

    // zero the allocated memory
    raw::memset_size_aligned(pageinfo_addr, pages::pages_to_bytes(page_info_pages_reqd, PageSize::Small), 0);
    
    #[cfg(debug_assertions)]
    serial_println!("memory zeroed for page info structs");

    #[cfg(debug_assertions)]
    serial_println!("initializing page info structs");

    gb.page_info = 
        Some(raw::abracadabra_array::<pages::PageInfo>(pageinfo_addr, gb.total_pages));
    let pageinfo = unsafe { gb.page_info.unwrap().as_mut().unwrap() };
    let mut p = 0usize.as_phys();

    for i in 0..gb.total_pages {
        pageinfo[i] = pages::PageInfo {
            phys_base: p.clone(),
            size: MEMORY_DEFAULT_PAGE_USIZE,
            uefi_flags: 0,
            status: pages::PageStatus::Free,
            owner: Owner::Nobody,
            purpose: MemoryType::CONVENTIONAL,
            flags: 0usize,
            page_size: PageSize::Small,
        };
        p.inner_inc_by_page_size(PageSize::Small);
    }
    
    #[cfg(debug_assertions)]
    serial_println!("page info structs initialized");

    // set up the final bits of gb with the proper structures
    gb.uefi_mem_map_0_1 = Some(mm);

    // don't do this @ home kids
    unsafe { UEFI_MEMORY_MAP_0 = gb.uefi_mem_map_0_1.as_ref() };

    //-----------------------------------------------------------------------------------

    // frame allocator init

    #[cfg(debug_assertions)]
    serial_println!("beginning physical frame allocator init");

    // set up & initialize the frame allocator
    let mut frame_alloc_temp = gb.frame_alloc_internal_0_2.lock();
    (*frame_alloc_temp) = Some(TreeAllocator::new());
    let frame_alloc = (*frame_alloc_temp).as_mut().unwrap();
    
    // wire up the bitmap allocation to the frame allocator
    frame_alloc.bitmap.as_mut().unwrap().bitmap
        .set(Some(raw::abracadabra::<usize>(bitmap_alloc_result.as_ref().unwrap().as_phys())));
    frame_alloc.bitmap.as_mut().unwrap()
        .init_phys_fixed(
            gb.total_pages, 
            bitmap_alloc_result
                .as_ref().unwrap().as_phys());

    // wire up the node storage allocation to the frame allocator and then
    // the node storage to the genesis block
    frame_alloc.phys_base.set(node_storage_alloc_result.as_ref().unwrap().as_phys());
    gb.mem_regions = Some(raw::abracadabra_array::<MemRegionDescr>(
        node_storage_alloc_result.as_ref().unwrap().as_phys(),
        gb.total_pages,
    ));

    frame_alloc.init();

    // unlock the frame allocator
    unsafe { gb.frame_alloc_internal_0_2.force_unlock() };

    #[cfg(debug_assertions)]
    serial_println!("physical frame allocator initialized");

    #[cfg(debug_assertions)]
    serial_println!("clearing the physical frame allocator bitmap");

    // initialize the bitmap (remember, our bitmaps are used in
    // reverse when used in the context of free/allocated items)
    raw::memset_size_aligned(
        bitmap_alloc_result.as_ref().unwrap().as_phys(),
        pages::pages_to_bytes(bitmap_pages_reqd, PageSize::Small),
        usize::MAX,
    );

    #[cfg(debug_assertions)]
    serial_println!("physical frame allocator bitmap clear");

    #[cfg(debug_assertions)]
    serial_println!("physical frame allocator bitmap struct initialized");

    #[cfg(debug_assertions)]
    serial_println!("initializing physical frame allocator node storage");

    // now we need to go through the memory map one final time and add all the regions
    // to their respective trees
    for e in gb.uefi_mem_map_0_1.as_ref().unwrap().entries() {
        if e.phys_start as usize >= gb.phys_mem_max.as_usize() {
            break;
        }

        if e.ty == MemoryType::CONVENTIONAL {
            frame_alloc.add_region(
                e.phys_start.as_phys(),
                e.page_count as usize * MEMORY_DEFAULT_PAGE_USIZE,
                true,
                0,
                Owner::Nobody,
            );
        } else {
            let mut frame_addr: PhysAddr = e.phys_start.as_phys();

            for _i in 0..e.page_count as usize {
                let frame_idx = pages::usize_to_page_index(frame_addr.as_usize());

                pageinfo[frame_idx].status = pages::PageStatus::Reserved;
                pageinfo[frame_idx].owner = Owner::System;
                pageinfo[frame_idx].purpose = e.ty;
                pageinfo[frame_idx].uefi_flags = e.att.bits() as usize;
                
                frame_addr.inner_inc_by_page_size(PageSize::Small);
            }

            frame_alloc.add_region(
                e.phys_start.as_phys(),
                e.page_count as usize * MEMORY_DEFAULT_PAGE_USIZE,
                false,
                0,
                Owner::System,
            );
        }
    }

    #[cfg(debug_assertions)]
    serial_println!("physical frame allocator node storage initialized");
    #[cfg(debug_assertions)]
    serial_println!("physical frame allocator initialized");

    #[cfg(debug_assertions)]
    serial_println!("enabling exception handling");

    // init exception handling
    #[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
    exceptions_init();

    #[cfg(debug_assertions)]
    serial_println!("exception handling enabled");

    //-----------------------------------------------------------------------------------

    // paging init

    #[cfg(debug_assertions)]
    serial_println!("beginning paging init");

    #[cfg(debug_assertions)]
    serial_println!("Initializing kernel VAS");

    // initialize the kernel's virtual address space - x86 & x64
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let mut kernel_vas = gb.base_vas_0_5.lock();
        (*kernel_vas) = Some(Vas::new());

        #[cfg(debug_assertions)]
        serial_println!("base page table created");

        #[cfg(debug_assertions)]
        serial_println!("conjuring base page table");

        (*kernel_vas).as_mut().unwrap().base_page_table =
            raw::abracadabra::<BasePageTable>(BasePageTable::new_base());

        #[cfg(debug_assertions)]
        serial_println!("base page table materialized");

        #[cfg(debug_assertions)]
        serial_println!("identity mapping memory map");

        (*kernel_vas).as_mut().unwrap().identity_map_based_on_memory_map();

        #[cfg(debug_assertions)]
        serial_println!("Memory map identity mapped");

        #[cfg(debug_assertions)]
        serial_println!("initializing base address register & switching to nebulae VAS");

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        _ = (*kernel_vas).as_mut().unwrap().init_cr3();

        // switch address spaces
        (*kernel_vas).as_mut().unwrap().switch_to();

        #[cfg(debug_assertions)]
        serial_println!("welcome to nebulae.");
    }

    //-----------------------------------------------------------------------------------

    // fin.

    // see how many free pages we have after bootstrapping the memory manager
    {
        // USING_FRAME_ALLOCATOR_6 is not required here because this is a non-mutable reference executed
        // as a single statement
        let free_pages = frame_alloc.free_page_count();
        serial_println!(
            "Free pages: {} / {} KB",
            free_pages,
            free_pages << UFACTOR_OF_4K
        );
    }

    // initialize the kernel's virtual address space
    // {
    //     serial_println!("Initializing kernel VAS");
    //     let mut kernel_vas = unsafe { KERNEL_BASE_VAS_4.lock() };
    //     *kernel_vas = Some(Vas::new());
    //     (*kernel_vas).as_mut().unwrap().base_page_table =
    //         raw::raw_to_ptr_mut::<BasePageTable, PhysAddr>(BasePageTable::new_base());
    //     (*kernel_vas)
    //         .as_mut()
    //         .unwrap()
    //         .identity_map_based_on_memory_map();

    //     serial_println!("Memory map identity mapped");

    //     _ = (*kernel_vas).as_mut().unwrap().init_cr3();
    //     (*kernel_vas).as_mut().unwrap().switch_to();

    //     #[cfg(debug_assertions)]
    //     serial_println!("Kernel VAS initialized");
    // }

    // set up a new stack and jump to kernel_main()
    kernel_main(gb_base);

    // halt
    #[cfg(debug_assertions)]
    serial_println!("Fell through kernel_main(), halting back in kernel_prep()");
    wait_forever();
}

pub fn iron_conv_pages() -> usize {
    let (_, conv_pages, _) = locate_genesis_block();
    conv_pages
}

pub fn iron_phys_mem_max() -> PhysAddr {
    let (_, _, phys_mem_max) = locate_genesis_block();
    phys_mem_max
}

// locate & return the genesis block
// returns ptr to block, number of pages in conventional memory, and max phys addr
// as a PhysAddr
pub fn locate_genesis_block() -> (&'static mut Nebulae<'static>, usize, PhysAddr) {
    static mut GBI: Option<(&mut Nebulae, usize, PhysAddr)> = None;

    if unsafe { GBI.is_some() } {
        unsafe {
            return (
                GBI.as_mut().unwrap().0,
                GBI.as_ref().unwrap().1,
                GBI.as_ref().unwrap().2,
            );
        }
    }

    let mut largest_conv_block: PhysAddr = usize::MIN.as_phys();
    let mut largest_conv_block_size = 0usize;
    let mut smallest_conv_block: PhysAddr = usize::MAX.as_phys();
    let mut smallest_conv_block_size = usize::MAX;
    let mut conv_page_count: usize = 0usize;
    let mut max_phys_present: usize = 0;

    for e in unsafe { UEFI_MEMORY_MAP_0.unwrap().entries() } {
        if e.ty == MemoryType::CONVENTIONAL {
            max_phys_present = e.phys_start as usize + pages::pages_to_bytes(e.page_count as usize, PageSize::Small);

            conv_page_count += e.page_count.as_usize();
            let pb = pages::pages_to_bytes(e.page_count.as_usize(), PageSize::Small);
            if pb > largest_conv_block_size {
                largest_conv_block = e.phys_start.as_phys();
                largest_conv_block_size = pb;
            }
            if pb < smallest_conv_block_size {
                smallest_conv_block = e.phys_start.as_phys();
                smallest_conv_block_size = pb;
            }
        }
    }

    // are the smallest and largest the same block?
    let genesis_block = if largest_conv_block == smallest_conv_block {
        // if so, then we can just grab the last page
        align_down(
            largest_conv_block.as_usize() + largest_conv_block_size - 1,
            MEMORY_DEFAULT_PAGE_USIZE,
        )
    } else {
        // otherwise, we need to see if the largest block's base address is page aligned
        // if it is, then we need to grab the last page from the smallest block
        // if it isn't, then we can grab the first page from the largest block
        if largest_conv_block.is_aligned(PageSize::Small.into_bits()) {
            align_down(
                smallest_conv_block.as_usize() + smallest_conv_block_size - 1,
                MEMORY_DEFAULT_PAGE_USIZE,
            )
        } else {
            align_down(largest_conv_block.as_usize(), MEMORY_DEFAULT_PAGE_USIZE)
        }
    };

    unsafe {
        GBI = Some((
            raw::raw_to_static_ref_mut::<Nebulae, PhysAddr>(genesis_block.as_phys()),
            conv_page_count,
            max_phys_present.as_phys(),
        ));
    }

    (
        raw::raw_to_static_ref_mut(genesis_block.as_phys()),
        conv_page_count,
        max_phys_present.as_phys(),
    )
}