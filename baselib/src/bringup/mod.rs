// Purpose: Bringup code for the kernel.

// KERNEL BRINGUP PROCEDURE
// 1. The kernel is loaded by the bootloader / firmware / hypervisor
// 2. The kernel is given control by the bootloader / firmware / hypervisor:
//    a. For uefi, the kernel is given a pointer to the boot services table
//       and control is passed to uefi_start() in iron/src/main.rs.
//    b. For multiboot, the kernel is given a pointer to the multiboot info struct
//       and control is passed to multiboot_start() in iron/src/main.rs.
// 3. The pre-init functions, which will vary by arch/platform/boot method, shall be
//    called xxx_pre_init() and shall be located in baselib/src/bringup/xxx.rs; as
//    an example, the uefi pre-init function is uefi_pre_init() and it is located in
//    baselib/src/bringup/uefi.rs.
//    a. The pre-init functions shall parse the boot information provided by the 
//       bootloader / firmware / hypervisor and shall add the available memory
//       frames to the frame allocator.
//    b. The pre-init functions shall then call the kernel_init() function, which
//       will perform the remaining initialization that is common to all archs/platforms.
// 4. kernel_init() -> upon completion of kernel environment setup, including creation
//    of a new kernel stack, shall call kernel_main() in iron/src/main.rs using a kernel
//    configured address space and the newly created stack.
// --------------------------------------------------------------------------------------

// Submodule(s)
pub mod uefi;

// Rust Items
use core::sync::atomic::{AtomicBool, Ordering};
// External Items
use ::uefi::table::boot::*;
// Internal Items
use crate::common::base::*;
use crate::structures::bitmap::*;
use crate::bringup::uefi::*;
use crate::cpu::*;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use crate::arch::x86::random::*;

// these constants should hold true for all uefi architectures
pub const MEMORY_DEFAULT_PAGE_USIZE: usize = USIZE_4K;
#[cfg(target_pointer_width = "32")]
pub const MEMORY_DEFAULT_PAGE_SIZE: u32 = SIZE_4K as u32;
#[cfg(target_pointer_width = "64")]
pub const MEMORY_DEFAULT_PAGE_SIZE: u64 = SIZE_4K;

// the max number of pages we can allocate before we must use an allocator
pub const MAX_PREBOOT_PAGES: usize = MEMORY_DEFAULT_PAGE_USIZE / core::mem::size_of::<MemBlock<PhysAddr>>();

// the factor for default page size in bits for shifting ops
pub const MEMORY_DEFAULT_SHIFT: usize = UFACTOR_OF_4K;

pub const PAGE_TABLE_ENTRY_UBYTES: usize = core::mem::size_of::<usize>();
pub const PAGE_TABLE_MAX_ENTRIES: usize = MEMORY_DEFAULT_PAGE_USIZE / PAGE_TABLE_ENTRY_UBYTES;

// how often the frame allocator will coalesce free regions
// in terms of de-allocations
pub const FRAME_ALLOCATOR_COALESCE_THRESHOLD_DEALLOC: usize = 100;

// kernel boot methods
#[repr(usize)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KernelBootMethod {
    Undetermined = 0,
    Uefi,
    Bios,
    Multiboot,
    Firmware,
    Bsp,
    Other,
}
impl KernelBootMethod {
    pub fn into_bits(self) -> usize {
        self as _
    }

    pub fn from_bits(bits: usize) -> KernelBootMethod {
        match bits {
            0 => KernelBootMethod::Uefi,
            1 => KernelBootMethod::Bios,
            2 => KernelBootMethod::Multiboot,
            3 => KernelBootMethod::Firmware,
            4 => KernelBootMethod::Bsp,
            _ => KernelBootMethod::Other,
        }
    }
}
impl const AsUsize for KernelBootMethod {
    fn as_usize(&self) -> usize {
        self.into_bits()
    }
}
impl From<usize> for KernelBootMethod {
    fn from(bits: usize) -> KernelBootMethod {
        KernelBootMethod::from_bits(bits)
    }
}

// kernel boot method
pub static mut KERNEL_BOOT_METHOD: KernelBootMethod = KernelBootMethod::Undetermined;

// returns a mutable reference to the base genesis frame (if it has been set, None otherwise);
// once set, the pointer cannot be changed
pub fn base_nebulae_genesis_frame(new_genesis_frame_base: Option<PhysAddr>) -> Option<&'static mut Nebulae<'static>> {

    static mut FUSE: AtomicBool = AtomicBool::new(false);
    static mut GENESIS_FRAME: Option<*mut Nebulae> = None;

    // if we have a new value, and the fuse is not blown, set the pointer
    if new_genesis_frame_base.is_some() {
        if !unsafe { FUSE.load(Ordering::SeqCst) } {
            unsafe {
                // set the fuse
                FUSE.store(true, Ordering::SeqCst);
                
                // store the pointer
                GENESIS_FRAME = Some(raw::abracadabra::<Nebulae>(new_genesis_frame_base.unwrap(), false));

                // blow the main genesis frame fuse for the base nebulae
                base_nebulae_genesis_frame_fuse(false);
            }
        } else {
            // this should only be blown once
            panic!("attempt to re-set base nebulae genesis frame pointer");
        }
    } else if unsafe { !FUSE.load(Ordering::Acquire) || GENESIS_FRAME.is_none() } { // caller is not trying to set, but we have no value yet
        return None;
    }

    // return the nebulae pointer
    unsafe { GENESIS_FRAME.unwrap().as_mut() }
}

// kernel init
pub fn kernel_init(conv_page_count: usize, phys_boundary: PhysAddr, scratch_base_addr: PhysAddr, _mmap_entry_count: usize) {
    
    // signal that we have the memory map in hand
    memory_map_fuse(false);
    
    //-----------------------------------------------------------------------------------
    
    // rng

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_print!("nebulae::uefi_init() -> seeding rng...");

    // use the 2nd scratch page for the rng
    let rng_base = scratch_base_addr.clone().inner_inc_by_page_size(MEMORY_DEFAULT_PAGE_SIZE_ENUM);
    let rng_result = Isaac64Rng::new_with_fixed_buf(rng_base);

    // panic if we can't instantiate the rng
    if rng_result.is_none() {
        panic!("nebulae::uefi_init() -> failed to instantiate rng");
    }

    // rng instantiation was successful, so continue

    // seed the rng with 512 random u32s
    let mut rng_seed: [u32; 512] = [ZERO_U32; 512];
    let fill_result = unsafe { rdseed_slice::<u32>(&mut rng_seed) };

    // panic if we can't fill the rngseed with random data
    if !fill_result {
        panic!("nebulae::uefi_init() -> failed to fill rng seed with random data");
    }

    // seed the rng
    let mut rng = rng_result.unwrap();
    rng.reseed_via_u32_array(&rng_seed);

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("seeded.");
    
    //-----------------------------------------------------------------------------------

    // genesis frame -> nebulae struct

    
    // obtain a mutable reference to the 3rd scratch page
    // as an array of MemoryDescriptor structs
    const MEM_DESCR_PER_PAGE: usize = MEMORY_DEFAULT_PAGE_USIZE / core::mem::size_of::<MemoryDescriptor>();
    let mm_scratch: &mut [MemoryDescriptor; MEM_DESCR_PER_PAGE] =
        unsafe { 
            core::mem::transmute::<PhysAddr, &mut [MemoryDescriptor; MEM_DESCR_PER_PAGE]>(rng_base.clone().inner_inc_by_page_size(MEMORY_DEFAULT_PAGE_SIZE_ENUM))
        };    
    
    let mut new_nebulae_base = rng.ranged_rand_usize(USIZE_512K, usize::min(phys_boundary.as_usize(), USIZE_2G)).align_canon_default().as_phys();
    let mut is_new_base_free = false;

    while !is_new_base_free {
        for e in mm_scratch.iter_mut() {
            if new_nebulae_base.as_usize() >= e.phys_start.as_usize() &&
               new_nebulae_base.as_usize() < e.phys_start.as_usize() + (e.page_count.as_usize() * MEMORY_DEFAULT_PAGE_USIZE) {

                if e.ty != MemoryType::CONVENTIONAL {
                    break;
                }

                is_new_base_free = true;
                break;
            }
        }
        if !is_new_base_free { new_nebulae_base = rng.ranged_rand_usize(USIZE_512K, usize::min(phys_boundary.as_usize(), USIZE_2G)).align_canon_default().as_phys(); }
    }

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("genesis frame location determined: 0x{:08x}.", new_nebulae_base.as_usize());

    // zero the new frame
    raw::memset_aligned(new_nebulae_base, MEMORY_DEFAULT_PAGE_USIZE, BytePattern::ZeroZero.as_usize_pattern());

    // mark it allocated in our scratch page array
    let allocated_frame_array: &mut [MemBlock<PhysAddr>; MAX_PREBOOT_PAGES] = unsafe { core::mem::transmute::<PhysAddr, &mut [MemBlock<PhysAddr>; MAX_PREBOOT_PAGES]>(scratch_base_addr) };
    allocated_frame_array[3].base_addr = new_nebulae_base;
    allocated_frame_array[3].size = MEMORY_DEFAULT_PAGE_USIZE;

    // instantiate the base nebulae struct @ the genesis frame base address
    {
        // instantiate the nebulae struct @ the new genesis frame base address
        let new_neb_id = rng.rand_usize(ZERO_USIZE);
        Nebulae::new_at_phys_fixed(new_nebulae_base, nebulae, new_neb_id, conv_page_count, pages::bytes_to_pages(phys_boundary.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM), phys_boundary, rng_base.clone().inner_inc_by_page_size(MEMORY_DEFAULT_PAGE_SIZE_ENUM));
    
        // sanity check
        let neb = raw::abracadabra::<Nebulae>(new_nebulae_base, false);
        
        if unsafe { (*neb).get_internal_id() } != new_neb_id {
            panic!("nebulae::uefi_init() -> failed to instantiate nebulae struct @ 0x{:08x}", new_nebulae_base.as_usize());
        }
        
        // set the genesis frame pointer
        base_nebulae_genesis_frame(Some(new_nebulae_base));

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("nebulae::uefi_init() -> genesis frame id matches: base nebulae genesis fuse blown.");
    }
    
    //-----------------------------------------------------------------------------------
    
    // initial memory allocations

    // see how many pages we need to track in the physical frame allocator
    let phys_range_page_count = pages::bytes_to_pages(phys_boundary.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM);

    // calc the # of pages required for page info structs (for physical memory)
    let page_info_pages_reqd = 
        pages::bytes_to_pages(
            core::mem::size_of::<pages::PageInfoStruct>() * phys_range_page_count,
        MEMORY_DEFAULT_PAGE_SIZE_ENUM
    );
    let page_info_bytes_reqd = pages::pages_to_bytes(page_info_pages_reqd, MEMORY_DEFAULT_PAGE_SIZE_ENUM);

    // the bitmap needs to accomodate conv_page_count potential nodes in the tree.
    // most degraded case in this design would be every other page being free,
    // and all pages being the smallest page size

    // the bitmap is used to indicate free or used spots in the mem frame descriptor array.
    // mem frames are managed by the frame allocator
    let bitmap_pages_reqd = 
        bitindex::calc_bitindex_size_in_pages(phys_range_page_count, MEMORY_DEFAULT_PAGE_SIZE_ENUM);
    let bitmap_bytes_reqd = pages::pages_to_bytes(bitmap_pages_reqd, MEMORY_DEFAULT_PAGE_SIZE_ENUM);

    // mem region descriptor structs to cover conv_page_count nodes (again, most degraded case)
    let node_storage_pages_reqd = pages::bytes_to_pages(
        phys_range_page_count * core::mem::size_of::<FrameDescr>(),
        MEMORY_DEFAULT_PAGE_SIZE_ENUM);
    let node_storage_bytes_reqd = pages::pages_to_bytes(node_storage_pages_reqd, MEMORY_DEFAULT_PAGE_SIZE_ENUM);

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!(
        "genesis::kernel_init() -> physical frame allocator requires {} pages",
        bitmap_pages_reqd + page_info_pages_reqd + node_storage_pages_reqd
    );

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("genesis::kernel_init() -> allocating memory for physical frame allocator bitmap");

    // Allocate memory for the bitmap
    let mut new_bitmap_base = rng.ranged_rand_usize(USIZE_512K, usize::min(phys_boundary.as_usize(), USIZE_2G)).align_canon_default().as_phys();
    let mut is_new_bitmap_base_free = false;
    
    while !is_new_bitmap_base_free {
        let mut frame_idx = ZERO_USIZE;
        let mut is_new_bitmap_base_allocated = false;

        while allocated_frame_array[frame_idx].size != ZERO_USIZE {
            if new_bitmap_base.as_usize() >= allocated_frame_array[frame_idx].base_addr.as_usize() &&
               new_bitmap_base.as_usize() < allocated_frame_array[frame_idx].base_addr.as_usize() + allocated_frame_array[frame_idx].size {
                
                    is_new_bitmap_base_allocated = true;
                    break;
            } else {
                frame_idx += 1;
            }
        }

        if !is_new_bitmap_base_allocated {
            for e in mm_scratch.iter_mut() {
                if new_bitmap_base.as_usize() >= e.phys_start.as_usize() &&
                   new_bitmap_base.as_usize() < e.phys_start.as_usize() + pages::pages_to_bytes(e.page_count.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM) {

                    // found a match but it's not conventional memory
                    if e.ty != MemoryType::CONVENTIONAL {
                        break;
                    }

                    // make sure we are still within the frame size tolerances
                    let total_frame_size = pages::pages_to_bytes(e.page_count.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM);
                    if new_bitmap_base.as_usize() + bitmap_bytes_reqd > e.phys_start.as_usize() + total_frame_size {
                        break;
                    }

                    is_new_bitmap_base_free = true;
                    break;
                }
            }
        }
        if !is_new_bitmap_base_free { new_bitmap_base = rng.ranged_rand_usize(USIZE_512K, usize::min(phys_boundary.as_usize(), USIZE_2G)).align_canon_default().as_phys(); }
    }
    
    // prepare the memory for bitmap usage (all ones)
    raw::memset_aligned(new_bitmap_base, bitmap_bytes_reqd, BytePattern::FF.as_usize_pattern());

    // mark the bitmap pages as allocated in our scratch array
    allocated_frame_array[4].base_addr = new_bitmap_base;
    allocated_frame_array[4].size = bitmap_bytes_reqd;
    
    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("genesis::kernel_init() -> physical frame allocator bitmap allocated: {} page(s) @ 0x{:08x}", bitmap_pages_reqd, new_bitmap_base.as_usize());

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("genesis::kernel_init() -> allocating memory for physical frame allocator node storage");

    // Allocate memory for node storage
    let mut new_node_storage_base = rng.ranged_rand_usize(USIZE_512K, usize::min(phys_boundary.as_usize(), USIZE_2G)).align_canon_default().as_phys();
    let mut is_new_node_storage_base_free = false;
    
    while !is_new_node_storage_base_free {
        let mut frame_idx = ZERO_USIZE;
        let mut is_new_node_storage_base_allocated = false;

        while allocated_frame_array[frame_idx].size != ZERO_USIZE {
            if new_node_storage_base.as_usize() >= allocated_frame_array[frame_idx].base_addr.as_usize() &&
               new_node_storage_base.as_usize() < allocated_frame_array[frame_idx].base_addr.as_usize() + allocated_frame_array[frame_idx].size {
                
                    is_new_node_storage_base_allocated = true;
                    break;
            } else {
                frame_idx += 1;
            }
        }

        if !is_new_node_storage_base_allocated {
            for e in mm_scratch.iter_mut() {
                if new_node_storage_base.as_usize() >= e.phys_start.as_usize() &&
                   new_node_storage_base.as_usize() < e.phys_start.as_usize() + pages::pages_to_bytes(e.page_count.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM) {

                    // found a match but it's not conventional memory
                    if e.ty != MemoryType::CONVENTIONAL {
                        break;
                    }

                    // make sure we are still within the frame size tolerances
                    let total_frame_size = pages::pages_to_bytes(e.page_count.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM);
                    if new_node_storage_base.as_usize() + node_storage_bytes_reqd > e.phys_start.as_usize() + total_frame_size {
                        break;
                    }

                    is_new_node_storage_base_free = true;
                    break;
                }
            }
        }
        if !is_new_node_storage_base_free { new_node_storage_base = rng.ranged_rand_usize(USIZE_512K, usize::min(phys_boundary.as_usize(), USIZE_2G)).align_canon_default().as_phys(); }
    }

    // zero the memory for node storage
    raw::memset_aligned(new_node_storage_base, node_storage_bytes_reqd, BytePattern::ZeroZero.as_usize_pattern());

    // mark the node storage as allocated in our scratch array
    allocated_frame_array[5].base_addr = new_node_storage_base;
    allocated_frame_array[5].size = node_storage_bytes_reqd;

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("genesis::kernel_prep() -> memory for node storage allocated: {} page(s) @ 0x{:08x}", node_storage_pages_reqd, new_node_storage_base.as_usize());
    
    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("genesis::kernel_init() -> allocating memory for memory info structs");

    // allocate memory for the page info structs
    let mut new_page_info_base = rng.ranged_rand_usize(USIZE_512K, usize::min(phys_boundary.as_usize(), USIZE_2G)).align_canon_default().as_phys();
    let mut is_new_page_info_base_free = false;
    
    while !is_new_page_info_base_free {
        let mut frame_idx = ZERO_USIZE;
        let mut is_new_page_info_base_allocated = false;

        while allocated_frame_array[frame_idx].size != ZERO_USIZE {
            if new_page_info_base.as_usize() >= allocated_frame_array[frame_idx].base_addr.as_usize() &&
               new_page_info_base.as_usize() < allocated_frame_array[frame_idx].base_addr.as_usize() + allocated_frame_array[frame_idx].size {
                
                    is_new_page_info_base_allocated = true;
                    break;
            } else {
                frame_idx += 1;
            }
        }

        if !is_new_page_info_base_allocated {
            for e in mm_scratch.iter_mut() {
                if new_page_info_base.as_usize() >= e.phys_start.as_usize() &&
                   new_page_info_base.as_usize() < e.phys_start.as_usize() + pages::pages_to_bytes(e.page_count.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM) {

                    // found a match but it's not conventional memory
                    if e.ty != MemoryType::CONVENTIONAL {
                        break;
                    }

                    // make sure we are still within the frame size tolerances
                    let total_frame_size = pages::pages_to_bytes(e.page_count.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM);
                    if new_page_info_base.as_usize() + page_info_bytes_reqd > e.phys_start.as_usize() + total_frame_size {
                        break;
                    }

                    is_new_page_info_base_free = true;
                    break;
                }
            }
        }
        if !is_new_page_info_base_free { new_page_info_base = rng.ranged_rand_usize(USIZE_512K, usize::min(phys_boundary.as_usize(), USIZE_2G)).align_canon_default().as_phys(); }
    }

    // zero the memory for page info structs
    raw::memset_aligned(new_page_info_base, page_info_bytes_reqd, BytePattern::ZeroZero.as_usize_pattern());
    
    // mark the page info struct storage as allocated in our scratch array
    allocated_frame_array[6].base_addr = new_page_info_base;
    allocated_frame_array[6].size = page_info_bytes_reqd;

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("genesis::kernel_init() -> memory for memory info structs allocated: {} page(s) @ 0x{:08x}", page_info_pages_reqd, new_page_info_base.as_usize());
    
    // if we booted via uefi, the time is now to exit boot services
    if unsafe { KERNEL_BOOT_METHOD } == KernelBootMethod::Uefi {
        uefi_exit_boot_services();
    }

    //-----------------------------------------------------------------------------------

    // memory structures init

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("sanity check -> iron() is returning 0x{:08x}", iron().unwrap() as *const Nebulae as usize);

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("sanity check -> iron() reports an internal id of 0x{:08x}", iron().unwrap().get_internal_id());
    
    let total_pages = iron().unwrap().get_total_pages();
    
    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("sanity check -> get_total_pages() reports managing {} pages", total_pages);

    // write the page info structs with default info

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("wiring up page info structs to nebulae struct");

    // wire up the page info structs to the nebulae struct
    {
        // lock the page info struct reference
        let mut page_info_struct_array_lock = iron().unwrap().page_info_structs_01.lock_rw_spin();
        
        // swap out the None for a valid reference
        (*page_info_struct_array_lock).replace(
            unsafe { 
                core::slice::from_raw_parts_mut::<pages::PageInfoStruct>(
                    new_page_info_base.as_usize() as *mut pages::PageInfoStruct,
                    total_pages
                )
            }
        );
    }

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("interior data set");
    
    {
        // obtain a mutable reference to the page info structs
        let mut page_info_struct_array_result = iron().unwrap().page_info_structs_01.lock_rw_spin();
        if (*page_info_struct_array_result).is_none() {
            panic!("failed to obtain pointer to page info structs");
        }

        let page_info_struct_array = page_info_struct_array_result.as_mut().unwrap().as_mut().unwrap();
        
        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("page info structs wired.");

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("applying default deny privilege to entire physical range.");

        // local var p is the overall base address
        let mut p = ZERO_USIZE.as_phys();

        // mark all pages in the physical range as 
        // reserved and unusable
        for i in 0..total_pages {
            page_info_struct_array[i] = pages::PageInfoStruct {
                status: pages::PageStatus::Reserved,
                held_by_fiber_id: ZERO_U128,
                flags: ZERO_USIZE,
                is_child: false,
            };
            p.inner_inc_by_page_size(MEMORY_DEFAULT_PAGE_SIZE_ENUM);
        }
    }

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("default deny on all physical pages set; page info structs initialized.");

    // almost there Alice...
    // let neb_sanity = locate_genesis_frame(None).unwrap_or_else(|| {
    //     panic!("failed to dereference pointer to genesis block");
    // });

    // #[cfg(all(debug_assertions, feature = "serialdbg"))]
    // serial_println!("genesis block sanity check: nebulae struct @ 0x{:08x}", neb_sanity as *const Nebulae as usize);

    // // make sure neb_sanity matches neb
    // if neb_sanity as *const Nebulae as usize != neb as *const Nebulae as usize {
    //     panic!("genesis block sanity check failed -> expected nebulae struct @ 0x{:0x}, actual: 0x{:0x}",
    //         neb_sanity as *const Nebulae as usize,
    //         neb as *const Nebulae as usize);
    // }

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("sane.");
    
    //-----------------------------------------------------------------------------------

    // frame allocator init

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("beginning physical frame allocator bringup & init");

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("constructing new TreeAllocator for physical frame management");

    // wire up the bitmap sub-allocator to the frame allocator and then
    // the node storage to the nebulae struct, then the nebulae struct back
    // to the frame allocator. The bitmap controls the node storage slots,
    // and the frame allocator owns the bitmap field, thus completing
    // the cycle (TM)
    
    {    
        // set up & initialize the frame allocator
        let mut frame_alloc_result = iron().unwrap().frame_alloc_internal_04.lock_rw_spin();
        (*frame_alloc_result) = Some(TreeAllocator::new(new_node_storage_base, total_pages));
        let frame_alloc = (*frame_alloc_result).as_mut().unwrap_or_else(|| {
            panic!("failed to dereference pointer to frame allocator");
        });
        
        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("wiring bitmap sub-allocator to physical frame allocator, calling init() on bitmap");

        // wire up the bitmap allocation to the frame allocator
        
        // first set the bitmap struct's bitmap pointer
        frame_alloc.frame_node_slot_bitmap.as_mut().unwrap().bitmap
            .set(Some(raw::abracadabra::<usize>(new_bitmap_base, false)));
        
        // then call the bitmap's physical fixed initializer
        frame_alloc.frame_node_slot_bitmap.as_mut().unwrap()
            .init_phys_fixed(
                total_pages,
                new_bitmap_base);

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("bitmap sub-allocator wired.");

        // now that the allocator is set up with a bitmap, we can call init() on the physical frame allocator
        
        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("calling init() on the physical frame allocator (TreeAllocator)");
        
        frame_alloc.init();
        
        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("node storage wired. adding memory frames to physical frame allocator");

        {
            // now we need to go through the uefi memory map one final time and add all the regions
            // to their respective trees

            // add_mem_frame() should never fail
            // there are enough slots pre-allocated for worst-case
            for e in mm_scratch {
                
                // once we hit a zero size descriptor, we are done
                if e.page_count.as_usize() == ZERO_USIZE {
                    break;
                }

                // if this range contains our genesis block, treat it specially
                if range_contains(
                    e.phys_start.as_usize(), 
                    pages::pages_to_bytes(e.page_count.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM), 
                    new_nebulae_base.as_usize()
                ) {
                    // get stats
                    let parent_region_size = pages::pages_to_bytes(e.page_count.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM);
                    let _gblock_idx: usize;
                    let _parent_region_idx: usize;

                    // we are either splitting the first page of this region
                    // or the last page of this region.
                    // either way, both regions are added to the frame allocator
                    // in their respective trunks
                    if new_nebulae_base.as_usize() == e.phys_start.as_usize() {
                        
                        // add the first page of this region
                        _ = frame_alloc.add_mem_frame(
                            e.phys_start.as_phys(),
                            MEMORY_DEFAULT_PAGE_USIZE,
                            false,
                            0,
                            Owner::Kernel,
                        ).unwrap();

                        // add the rest of this region
                        _ = frame_alloc.add_mem_frame(
                            PhysAddr(e.phys_start.as_usize() + MEMORY_DEFAULT_PAGE_USIZE),
                            parent_region_size - MEMORY_DEFAULT_PAGE_USIZE,
                            true,
                            0,
                            Owner::Nobody,
                        ).unwrap();
                    } else {
                        // split off the last page of this region

                        // the last page
                        _ = frame_alloc.add_mem_frame(
                            PhysAddr(e.phys_start.as_usize() + parent_region_size - MEMORY_DEFAULT_PAGE_USIZE),
                            MEMORY_DEFAULT_PAGE_USIZE,
                            false,
                            0,
                            Owner::Kernel,
                        ).unwrap();

                        // the rest of this region
                        _ = frame_alloc.add_mem_frame(
                            e.phys_start.as_phys(),
                            parent_region_size - MEMORY_DEFAULT_PAGE_USIZE,
                            true,
                            0,
                            Owner::Nobody,
                        ).unwrap();
                    }

                    // we are set with the genesis block now
                    continue;
                } else {
                    if e.ty == MemoryType::CONVENTIONAL {
                        _ = frame_alloc.add_mem_frame(
                            e.phys_start.as_phys(),
                            pages::pages_to_bytes(e.page_count.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM),
                            true,
                            0,
                            Owner::Nobody,
                        );
                    } else {
                        _ = frame_alloc.add_mem_frame(
                            e.phys_start.as_phys(),
                            pages::pages_to_bytes(e.page_count.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM),
                            false,
                            0,
                            Owner::Reserved,
                        );
                    }
                }
            }
        }
    }
    
    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("verifying physical frame allocator parameters");

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("physical frame allocator initialized");

    // blow the physical frame allocator fuse
    frame_alloc_fuse(false);

    //-----------------------------------------------------------------------------------

    // paging init

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("beginning paging init");

    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("Initializing kernel address space");

    // initialize the kernel's virtual address space - x86 & x64
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        // set the base page table
        {
            let mut kernel_vas = iron().unwrap().base_vas_07.lock_rw_spin();
            kernel_vas.as_mut().unwrap().replace(Vas::new());

            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("new base page table instantiated.");

            // set up the new address space
            let new_base_page_table_addr_result = BasePageTable::new_addr_space(Owner::Kernel);
            if new_base_page_table_addr_result.is_none() {
                panic!("failed to instantiate new base page table");
            }

            // assign the pointer to the new base page table to the kernel's
            // virtual address space
            kernel_vas.as_mut().unwrap().as_mut().unwrap().base_page_table.replace(
                unsafe { raw::abracadabra::<BasePageTable>(new_base_page_table_addr_result.unwrap(), false).as_mut().unwrap() }
            );
        }
        
        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("base page table initialized.");

        // #[cfg(all(debug_assertions, feature = "serialdbg"))]
        // serial_println!("programming system memory layout into new base page table...");

        // // this is where we map the kernel's memory layout into the new page directory
        // // we've set up
        // {
        //     // go through the alloc regions and identity map them

        //     // obtain a ref to the frame allocator
        //     // this lock MUST be released prior to mapping pages
        //     let mut frame_alloc = iron().unwrap().frame_alloc_internal_04.lock_rw_spin();

        //     // obtain a reference to the alloc'ed frames' address trunk
        //     let alloc_frames = (*frame_alloc).unwrap();

        //     // obtain a ref to our memory regions
        //     let mem_frame_array = frame_alloc.as_mut().unwrap().as_mut().unwrap().get_mem_frame_descr_array_mut();

        //     #[cfg(all(debug_assertions, feature = "serialdbg"))]
        //     serial_println!("obtained reference to memory regions at 0x{:08x}", raw::ref_to_raw::<FrameDescr, PhysAddr>(&mem_frame_array[0]));
            
        //     // iterate through the alloc'ed regions

        //     // if there is no root node, then there are no alloc'ed regions to identity map
        //     if alloc_frames.root().is_none() {
        //         #[cfg(all(debug_assertions, feature = "serialdbg"))]
        //         serial_println!("no existing memory regions to program.");
        //     } else {
        //         // otherwise, we have some regions to identity map
        //         #[cfg(all(debug_assertions, feature = "serialdbg"))]
        //         serial_println!("found existing memory regions to program.");

        //         // iterate through the node(s), if any;
                
        //         // set the current alloc'ed node to the maximum node
        //         let mut current_alloc_node = alloc_frames.max_node();
                
        //         // iterate through the alloc'ed nodes by address, high to low
        //         while !current_alloc_node.is_none() {
        //             let region_base: u64;
        //             let region_size: u64;
        //             let region_idx: usize;
                    
        //             // get the size and base address of the region
        //             region_base = hi64((*current_alloc_node.unwrap()).key());
        //             region_size = lo64((*current_alloc_node.unwrap()).key());
        //             region_idx = (*current_alloc_node.unwrap()).value();

        //             #[cfg(all(debug_assertions, feature = "serialdbg"))]
        //             serial_println!("-> mapping region id {} @ 0x{:08x} size {}.", region_idx, region_base, region_size);
                    
        //             #[cfg(all(debug_assertions, feature = "serialdbg"))]
        //             unsafe {
        //                 serial_println!("-> region id {}: size: {}, owner: {:?}, idx: {}", 
        //                     region_idx, mem_frame_array[region_idx].mem_block.get().as_ref().unwrap().size,
        //                     mem_frame_array[region_idx].owner.get().as_ref().unwrap().clone(),
        //                     mem_frame_array[region_idx].mem_frame_idx.get().as_ref().unwrap().clone());
        //             }

        //             // calculate the number of pages in the region
        //             let region_page_count = pages::bytes_to_pages(region_size.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM);

        //             #[cfg(all(debug_assertions, feature = "serialdbg"))]
        //             serial_println!("-> region has {} page(s)", region_page_count);

        //             // map each page in the region
        //             let mut current_addr = region_base.as_phys();

        //             #[cfg(all(debug_assertions, feature = "serialdbg"))]
        //             serial_println!("-> identity mapping region page(s)");

        //             for _i in 0..region_page_count {
                        
        //                 // lock the address space and map the pages for this region
        //                 {
        //                     let mut kernel_vas = iron().unwrap().base_vas_07.lock_rw_spin();
                            
        //                     // verify the lock
        //                     if (*kernel_vas).is_none() {
        //                         serial_println!("-> failure mapping page: could not locate kernel vas");
        //                         continue;
        //                     }

        //                     // unfortunately these mappings in early boot are critical
        //                     unsafe {
        //                         (*kernel_vas).as_mut().unwrap_or_else(|| {
        //                                 panic!("-> failure mapping page: could not locate kernel vas lock");
        //                             })
        //                             .base_page_table.as_mut().unwrap_or_else(|| {
        //                                 panic!("-> failure mapping page: could not locate kernel vas base struct pointer");
        //                             }).as_mut().unwrap()
        //                             .map_page(
        //                             current_addr,
        //                             current_addr.as_usize().as_virt(),
        //                             MEMORY_DEFAULT_PAGE_SIZE_ENUM,
        //                             0,                                
        //                         );
        //                     }
        //                 }
                        
        //                 // increment the current address
        //                 current_addr.inner_inc_by_page_size(MEMORY_DEFAULT_PAGE_SIZE_ENUM);
        //             }

        //             // if we just did base zero, we are finished
        //             if region_base == 0 {
        //                 break;
        //             }

        //             // switch the current node the prev node
        //             let key_prev = make128(region_base.as_usize() - 1, 0);
        //             current_alloc_node = alloc_frames.floor_node(key_prev);
        //         }
        //     }
        // }

        // #[cfg(all(debug_assertions, feature = "serialdbg"))]
        // serial_println!("new address space programming complete.");

        // // see how many free pages we have after bootstrapping the memory manager
        // {
        //     let free_pages = iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap()
        //         .free_page_count();
            
        //     serial_println!(
        //         "Free pages: {} / {} KB",
        //         free_pages,
        //         free_pages << UFACTOR_OF_4K
        //     );
        // }
        
        // #[cfg(all(debug_assertions, feature = "serialdbg"))]
        // serial_println!("initializing base register & switching to nebulae address space");

        // // initialize the base address register
        // {            
        //     #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        //     let new_cr3 = iron().unwrap().base_vas_07.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().init_cr3().unwrap();

        //     #[cfg(all(debug_assertions, feature = "serialdbg"))]
        //     serial_println!("base address register initialized: 0x{:08x}", new_cr3);
        // }

        // // switch to the nebulae address space
        // {
        //     // switch address spaces
        //     iron().unwrap().base_vas_07.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().switch_to();
        // }

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("welcome to nebulae.");
    }

    //-----------------------------------------------------------------------------------

    // fin.

    // set up a new stack and jump to kernel_main()
    // one day Johnny, you'll go to -> kernel_main(); too

    // halt
    #[cfg(all(debug_assertions, feature = "serialdbg"))]
    serial_println!("Fell through kernel_main(), halting back in kernel_init()");
    wait_forever();
}