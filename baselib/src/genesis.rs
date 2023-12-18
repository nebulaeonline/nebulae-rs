use core::mem;

use crate::common::base::*;
use crate::common::kernel_statics::*;
use crate::bitmap::*;

#[repr(C)]
pub struct GenesisBlock {
    pub magic: usize,
    pub genesis_ptr: &'static mut GenesisBlock,
    pub conv_pages: usize,
    pub total_pages: usize,
    pub phys_mem_max: PhysAddr,
    pub page_info: *mut [pages::PageInfo],
    pub mem_regions: *mut [MemRegionDescr],
    pub region_bitmap: *mut Bitmap, 
    pub base_vas: spin::Mutex<Vas>,
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
pub fn locate_genesis_block() -> (&'static mut GenesisBlock, usize, PhysAddr) {
    static mut GBI: Option<(&mut GenesisBlock, usize, PhysAddr)> = None;

    if unsafe { GBI.is_some() } {
        unsafe { return (GBI.as_mut().unwrap().0, GBI.as_ref().unwrap().1, GBI.as_ref().unwrap().2); }
    }

    use uefi::table::boot::*;
    
    let mut largest_conv_block: PhysAddr = PhysAddr::from(usize::MIN);
    let mut largest_conv_block_size = 0usize;
    let mut smallest_conv_block: PhysAddr = PhysAddr::from(usize::MAX);
    let mut smallest_conv_block_size = usize::MAX;
    let mut conv_page_count: usize = 0usize;

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

    // Allocate memory for the bitmap using UEFI allocate_pages.
    let st = unsafe { UEFI_SYSTEM_TABLE_0.lock().as_mut().unwrap().unsafe_clone() };
    let mut mm: MemoryMap;

    // Figure out how many we need (the map size will change)
    let mm_sizes = st.boot_services().memory_map_size();
    let mut mm_pages_reqd = pages::calc_pages_reqd(
        mm_sizes.map_size, PageSize::Small);
    let mut mm_total_allocation = pages::pages_to_bytes(mm_pages_reqd);
    
    // if we have less than 64 entries worth of free space, add a page to the
    // allocation
    if mm_total_allocation - mm_sizes.map_size < (64 * mm_sizes.entry_size) {
        mm_pages_reqd += 1;
        mm_total_allocation += MEMORY_DEFAULT_PAGE_USIZE;
    }

    // actually allocate memory for the map
    let mm_result = st.boot_services().allocate_pages(
        AllocateType::AnyPages,
        MemoryType::custom(MEMORY_TYPE_UEFI_MEM_MAP), // Use the custom memory type
        mm_pages_reqd,
    );

    // make sure we got valid results; panic if not
    match mm_result {
        Ok(frame) => {
            mm = unsafe {
                st.boot_services().memory_map(
                    core::slice::from_raw_parts_mut(
                        frame as *mut u8,
                        pages::pages_to_bytes(mm_pages_reqd),
                    )
                ).unwrap_or_else(|e| {
                    panic!("Physical frame allocator failed to read the UEFI memory map: {:?}", e);
                })
            };
        }
        Err(_) => panic!("Failed to allocate memory for physical frame allocator"),
    }

    // Process the memory map
    let mut max_phys_present: usize = 0;
    
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

        if e.ty == MemoryType::CONVENTIONAL {
            max_phys_present =
                e.phys_start as usize + pages::pages_to_bytes(e.page_count as usize);

            conv_page_count += e.page_count.as_usize();
            let pb = pages::pages_to_bytes(e.page_count.as_usize());
            if pb > largest_conv_block_size {
                largest_conv_block = PhysAddr::from(e.phys_start);
                largest_conv_block_size = pb;
            }
            if pb < smallest_conv_block_size {
                smallest_conv_block = PhysAddr::from(e.phys_start);
                smallest_conv_block_size = pb;
            }
        }
    }

    #[cfg(debug_assertions)]
    serial_println!("max conventional address: 0x{:0x}", max_phys_present);

    // are the smallest and largest the same block?
    let genesis_block = if largest_conv_block == smallest_conv_block {
        // if so, then we can just grab the last page
        align_down(largest_conv_block.as_usize() + largest_conv_block_size - 1, MEMORY_DEFAULT_PAGE_USIZE)
    } else {
        // otherwise, we need to see if the largest block's base address is page aligned
        // if it is, then we need to grab the last page from the smallest block
        // if it isn't, then we can grab the first page from the largest block
        if largest_conv_block.is_page_aligned() {
            align_down(smallest_conv_block.as_usize() + smallest_conv_block_size - 1, MEMORY_DEFAULT_PAGE_USIZE)
        } else {
            align_down(largest_conv_block.as_usize(), MEMORY_DEFAULT_PAGE_USIZE)
        }
    };

    unsafe { GBI = Some((raw::raw_to_static_ref_mut::<GenesisBlock, PhysAddr>(PhysAddr::from(genesis_block)), conv_page_count, PhysAddr::from(max_phys_present))); }

    (raw::raw_to_static_ref_mut(PhysAddr::from(genesis_block)), conv_page_count, PhysAddr::from(max_phys_present))
}

// init the genesis block
// returns the location of the page info array
pub fn init_genesis_block() {
    use crate::common::*;
    use core::sync::atomic::{AtomicBool, Ordering};
    
    static mut FUSE: AtomicBool = AtomicBool::new(false);

    if unsafe { FUSE.load(Ordering::Relaxed) } {
        return;
    } else {
        unsafe { FUSE.store(true, Ordering::Relaxed); }
    }

    let mut mm: MemoryMap;

    let (mut gb, conv_page_count, max_phys_mem) = locate_genesis_block();

    // set the basic kernel table parameters
    gb.magic = NEBULAE;
    gb.genesis_ptr = raw::raw_to_static_ref_mut::<GenesisBlock, PhysAddr>(PhysAddr::from(gb as *mut GenesisBlock as usize));
    gb.conv_pages = conv_page_count;
    gb.total_pages = pages::calc_pages_reqd(max_phys_mem.as_usize(), PageSize::Small);
    gb.phys_mem_max = max_phys_mem;

    #[cfg(debug_assertions)]
    serial_println!("allocating memory for physical frame allocator");

    // we need to know how many pages it will take to track the entire physical memory
    // this is the number of pages to track with 1 node per page allocation potential
    // (the most degraded case), so we need to divide by the number of nodes per page
    let bitmap_pages_reqd = bitindex::BitIndex::calc_bitindex_size_in_default_pages(gb.total_pages);
    let page_info_pages_reqd = pages::calc_pages_reqd(mem::size_of::<pages::PageInfo>() * gb.total_pages, PageSize::Small);
    let storage_pages_reqd = pages::calc_pages_reqd(
        gb.total_pages * core::mem::size_of::<MemRegionDescr>(),
        PageSize::Small,
    );

    // Allocate memory for the bitmap using UEFI allocate_pages().
    let bitmap_alloc_result = unsafe {
        UEFI_SYSTEM_TABLE_0
            .lock()
            .as_ref()
            .unwrap()
            .boot_services()
            .allocate_pages(
                AllocateType::AnyPages,
                MemoryType::custom(MEMORY_TYPE_BOOT_FRAMER), // Use the custom memory type
                bitmap_pages_reqd,
            )
    };

    if bitmap_alloc_result.is_err() {
        panic!("Failed to allocate memory for physical frame allocator tree bitmap");
    } else {
        unsafe {
            *gb.region_bitmap = Bitmap::new();
            (*gb.region_bitmap).init(
                gb.total_pages, 
                VirtAddr(ZERO_USIZE), 
                VirtAddr::from(bitmap_alloc_result.unwrap()));
        }
    }

    // Allocate for page info
    let page_info_alloc_result = unsafe {
        UEFI_SYSTEM_TABLE_0
            .lock()
            .as_ref()
            .unwrap()
            .boot_services()
            .allocate_pages(
                AllocateType::AnyPages,
                MemoryType::custom(MEMORY_TYPE_BOOT_FRAMER), // Use the custom memory type
                page_info_pages_reqd,
            )
    };

    if page_info_alloc_result.is_err() {
        panic!("Failed to allocate memory for memory info");
    } else {
        #[cfg(debug_assertions)]
        serial_println!("recording memory info");

        gb.page_info = unsafe {
            core::slice::from_raw_parts_mut::<pages::PageInfo>(
                page_info_alloc_result.unwrap() as *mut pages::PageInfo,
                bitindex::BitIndex::calc_bitindex_size_in_usize(gb.total_pages))
            };

        // write the page info structs with default info
        for i in 0..gb.total_pages {
            unsafe {
                gb.page_info.as_mut().unwrap()[i] = pages::PageInfo {
                    phys_base: PhysAddr::from(i * MEMORY_DEFAULT_PAGE_USIZE),
                    size: MEMORY_DEFAULT_PAGE_USIZE,
                    uefi_flags: 0,
                    status: pages::PageStatus::Free,
                    owner: Owner::Nobody,
                    purpose: MemoryType::CONVENTIONAL,
                    flags: 0usize,
                };
            }
        }

        // now go back through the memory map and mark the pages
        // as appropriate for status and owner info
        for e in mm.entries() {
            if e.phys_start as usize > gb.phys_mem_max.as_usize() {
                break;
            }

            if e.ty != MemoryType::CONVENTIONAL {
                let mut frame_addr: PhysAddr = PhysAddr::from(e.phys_start);
                
                unsafe {
                    for i in 0..e.page_count as usize {
                        let frame_idx = pages::usize_to_page_index(frame_addr.as_usize());

                        gb.page_info.as_mut().unwrap()[frame_idx].status = pages::PageStatus::Reserved;
                        gb.page_info.as_mut().unwrap()[frame_idx].owner = Owner::System;
                        gb.page_info.as_mut().unwrap()[frame_idx].purpose = e.ty;
                        gb.page_info.as_mut().unwrap()[frame_idx].uefi_flags = e.att.bits() as usize;
                        frame_addr.inner_inc_by_page_size(PageSize::Small);
                    }
                }
            }
        }
    }

    // Allocate memory for node storage.
    let storage_alloc_result = unsafe {
        UEFI_SYSTEM_TABLE_0
            .lock()
            .as_ref()
            .unwrap()
            .boot_services()
            .allocate_pages(
                AllocateType::AnyPages,
                MemoryType::custom(MEMORY_TYPE_BOOT_FRAMER), // Use the custom memory type
                storage_pages_reqd,
            )
    };

    if storage_alloc_result.is_err() {
        panic!("Failed to allocate memory for physical frame allocator node storage");
    } else {
        gb.mem_regions = unsafe {
            core::slice::from_raw_parts_mut::<MemRegionDescr>(
                storage_alloc_result.unwrap() as *mut MemRegionDescr,
                bitindex::BitIndex::calc_bitindex_size_in_usize(gb.total_pages))
            };
    }

    #[cfg(debug_assertions)]
    serial_println!("physical frame allocator memory allocated");

    #[cfg(debug_assertions)]
    serial_println!("terminating uefi services");

    // exit uefi
    (_, mm) = unsafe {
        UEFI_SYSTEM_TABLE_0
            .lock()
            .as_ref()
            .unwrap()
            .unsafe_clone()
            .exit_boot_services(MemoryType::custom(MEMORY_TYPE_UEFI_MEM_MAP))
    };

    {
        // store memory map in global
        let mut map = unsafe { UEFI_MEMORY_MAP_1.lock() };
        *map = Some(mm);
    }

    #[cfg(debug_assertions)]
    serial_println!("uefi services terminated");

    #[cfg(debug_assertions)]
    serial_println!("beginning base allocator config");
}