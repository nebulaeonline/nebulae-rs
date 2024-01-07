// Rust
use core::sync::atomic::{AtomicBool, Ordering};
use core::fmt::Write;
// External
use ::uefi::prelude::*;
use ::uefi::table::boot::*;
// Internal
use crate::common::base::*;

// Constants
pub const PREBOOT_SCRATCH_PAGE_COUNT: usize = 3;

// Etc. ->

// Bringup files are for establishing a baseline environment
// for the kernel from various platforms / bootloaders. 
// Because memory maps are unique, we parse them and add the available 
// memory frames to the frame allocator, and then call kernel_init().

// uefi_pre_init() is called from uefi_start() in iron/src/main.rs.
pub fn uefi_pre_init() {
    
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
    
    // uefi pre-init begin

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("nebulae::uefi_pre_init() -> beginning prep for kernel_init()");

    // obtain a handle to our uefi system table
    let st = 
        uefi_system_table(None, false).unwrap_or_else(|| {
            panic!("failed to obtain reference to uefi system table");
        });
    
    // say hello -> this will be our last display output until 
    // we have a working display / framebuffer driver
    _ = writeln!(st.stdout(), "nebulae says hello via uefi :)");

    
    //-----------------------------------------------------------------------------------

    // memory map

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("nebulae::uefi_pre_init() -> parsing memory map");

    let mut conv_page_count: usize = ZERO_USIZE;
    let mut phys_boundary: usize = ZERO_USIZE;

    // memory map var
    let mm: MemoryMap;
    
    // query for the size of the memory map
    let mm_size_struct = st
        .boot_services()
        .memory_map_size();

    // do some size calcs
    let mut mm_size_in_pages = pages::bytes_to_pages(mm_size_struct.map_size, MEMORY_DEFAULT_PAGE_SIZE_ENUM);
    
    // we are going to do some allocations; uefi seems to report a smaller
    // map size than exit_boot_services() ultimately needs, so we are going
    // to allocate an extra page if there's not enough space for at least 
    // 5 additional entries in the map
    if pages::pages_to_bytes(mm_size_in_pages, MEMORY_DEFAULT_PAGE_SIZE_ENUM) - mm_size_struct.map_size < 5 * core::mem::size_of::<MemoryDescriptor>() {
        mm_size_in_pages += 1;
    }
    
    // allocate memory for the memory map (SOLE UEFI ALLOCATION)
    let mm_alloc_result = st
        .boot_services().allocate_pages(
                AllocateType::MaxAddress(SIZE_2G),
                MemoryType::custom(MEMORY_TYPE_UEFI_MEM_MAP), // Use the custom memory type
                mm_size_in_pages,
            );

    // panic if we couldn't allocate memory for the memory map
    if mm_alloc_result.is_err() {
        panic!("nebulae::uefi_pre_init() -> failed to allocate memory for uefi memory map");
    }

    // allocation via uefi was successful, so continue
    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("{} page(s) allocated", mm_size_in_pages);
    
    // store the base address of our allocated memory (safe unwrap, checked above)
    let mm_buf_addr = mm_alloc_result.unwrap().as_phys();

    // get a mutable reference to the buffer allocated via uefi & zero the memory
    let buf = raw::abracadabra_array_mut::<u8>(mm_buf_addr, pages::pages_to_bytes(mm_size_in_pages, MEMORY_DEFAULT_PAGE_SIZE_ENUM), true);
    
    // call uefi to obtain the memory map
    let uefi_result = 
        st.boot_services().memory_map(unsafe { buf.as_mut().unwrap() });

    if uefi_result.is_err() {
        panic!("nebulae::uefi_pre_init() -> failed to obtain uefi memory map");        
    } else {
        mm = uefi_result.unwrap();
    }

    // we are going to comandeer the first 3 physical pages of the first 
    // conventional frame large enough to hold them. we will use these pages
    // as scratch space during the rest of the boot process
    let mut scratch_base_addr: PhysAddr = PhysAddr(NEBULAE_TEST_PATTERN);

    // we have the memory map, so first find some space for our scratch pages
    for e in mm.entries() {
        
        // see if this block is suitable for our scratch pages
        if e.ty == MemoryType::CONVENTIONAL && 
           e.page_count >= PREBOOT_SCRATCH_PAGE_COUNT as u64 {

            scratch_base_addr = PhysAddr(e.phys_start.as_usize());
            break;
        }
    }

    // make sure we found a suitable block for our scratch pages
    // if not, panic -> it's only 12k worth of memory on most systems.
    if scratch_base_addr.is_null() {
        panic!("nebulae::uefi_pre_init() -> failed to locate suitable block for initial scratch");
    }

    // zero the scratch pages
    raw::memset_aligned(
        scratch_base_addr, 
        pages::pages_to_bytes(
            PREBOOT_SCRATCH_PAGE_COUNT, 
            MEMORY_DEFAULT_PAGE_SIZE_ENUM), 
        BytePattern::ZeroZero.as_usize_pattern());

    // the first page of scratch is where we will store info on the memory
    // frames we allocate during bringup; it's nothing fancy, we're
    // just recording the base address and size of the allocated frames
    // until we can manage them with the physical frame allocator

    // the second page of scratch will be used for the rng

    // the third page of scratch will be used as a temporary memory map

    let allocated_frame_array: &mut [MemBlock<PhysAddr>; MAX_PREBOOT_PAGES] = unsafe { core::mem::transmute::<PhysAddr, &mut [MemBlock<PhysAddr>; MAX_PREBOOT_PAGES]>(scratch_base_addr) };
    
    // mark the scratch pages as allocated by storing them in the
    // array located on the first scratch page 
    // (max preboot pages: 512 pages 32-bit, 256 pages 64-bit)
    allocated_frame_array[0].base_addr = scratch_base_addr;
    allocated_frame_array[0].size = MEMORY_DEFAULT_PAGE_USIZE;
    allocated_frame_array[1].base_addr = scratch_base_addr.clone().inner_inc_by_page_size(MEMORY_DEFAULT_PAGE_SIZE_ENUM);
    allocated_frame_array[1].size = MEMORY_DEFAULT_PAGE_USIZE;
    allocated_frame_array[2].base_addr = allocated_frame_array[1].base_addr.clone().inner_inc_by_page_size(MEMORY_DEFAULT_PAGE_SIZE_ENUM);
    allocated_frame_array[2].size = MEMORY_DEFAULT_PAGE_USIZE;
    
    // obtain a mutable reference to the 3rd scratch page
    // as an array of MemoryDescriptor structs
    let mm_scratch: &mut [MemoryDescriptor; MEMORY_DEFAULT_PAGE_USIZE / core::mem::size_of::<MemoryDescriptor>()]
            = unsafe { core::mem::transmute::<PhysAddr, &mut [MemoryDescriptor; MEMORY_DEFAULT_PAGE_USIZE / core::mem::size_of::<MemoryDescriptor>()]>(allocated_frame_array[2].base_addr) };

    // go back through the memory map to get some stats
    // -> print the map entries to the serial console
    // in debug mode
    let mut i = ZERO_USIZE;

    for e in mm.entries() {
        
        // write the descriptor to our scratch page
        mm_scratch[i] = e.clone();
        i += 1;

        // output the map entries in debug mode
        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!(
            "ty: {:?} ps: 0x{:08x} pc: {} flags: {:?}",
            e.ty,
            e.phys_start,
            e.page_count,
            e.att
        );

        // we are only interested in conventional memory for stats
        if e.ty == MemoryType::CONVENTIONAL {
            phys_boundary = e.phys_start as usize + pages::pages_to_bytes(e.page_count as usize, MEMORY_DEFAULT_PAGE_SIZE_ENUM);
            conv_page_count += e.page_count.as_usize();        
        }
    }

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("physical address boundary: 0x{:0x}", phys_boundary);

    kernel_init(conv_page_count, phys_boundary.as_phys(), scratch_base_addr, i);
}

pub fn uefi_exit_boot_services() {
    // Function guard - atomic fuse
    static mut FUSE: AtomicBool = AtomicBool::new(false);

    if unsafe { FUSE.load(Ordering::SeqCst) } {
        return;
    } else {
        unsafe {
            FUSE.store(true, Ordering::SeqCst);
        }
    }
    
    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("terminating uefi boot services");

    let (_, _mm) = 
        unsafe { 
            uefi_system_table(
                None, 
                false)
            .unwrap()
            .unsafe_clone()
        }.exit_boot_services(
            MemoryType::custom(MEMORY_TYPE_UEFI_MEM_MAP)
        );

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("uefi boot services terminated");
}

// returns the uefi system table pointer
// once set, the pointer cannot be changed;
// once purged, the pointer cannot be reset
pub fn uefi_system_table(sys_table: Option<SystemTable<Boot>>, purge: bool) -> Option<&'static mut SystemTable<Boot>> {

    static mut FUSE: AtomicBool = AtomicBool::new(false);
    static mut UEFI_SYS_TABLE: Option<SystemTable<Boot>> = None;

    // if we have a new value, and the fuse is not blown, clone the table
    if sys_table.is_some() {           
        if !unsafe { FUSE.load(Ordering::SeqCst) } {
            unsafe {
                FUSE.store(true, Ordering::SeqCst);
                UEFI_SYS_TABLE = Some(sys_table.unwrap().unsafe_clone());
            }
        } else {
            // if the fuse is blown, we can't clone the table
            // -> signal that we were unable to clone the table via None
            return None;
        }
    }

    // return the reference unless it's none
    // -> no need to go any further or purge in that case
    if unsafe { UEFI_SYS_TABLE.is_none() } {
        return None;
    }

    // purge the value if requested
    // -> returns the pointer upon purge
    if purge {
        unsafe {
            UEFI_SYS_TABLE = None;
        }
    }
    unsafe { Some(UEFI_SYS_TABLE.as_mut().unwrap()) }
}