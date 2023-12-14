use crate::common::*;
use crate::bitmap::*;

use core::ptr;
use core::ops::Range;

// CONSTANTS


// STATICS

// THE REST

// NOTE ON ALLOCATORS AND BITMAP USAGE --
// The bitmap used to track allocations operates opposite a normal bitmap;
// a 1 indicates a free page, and a 0 indicates an allocated page.
// This is done to be able to use special CPU instructions to search the
// bitmap.  So the language of set*() and clear*() is reversed from normal.

pub struct FrameAllocator {
    bitmap: Bitmap,
    buffer: *mut [u8; 2 * MEMORY_DEFAULT_PAGE_USIZE],
    buffer_allocated: bool,
}

impl FrameAllocator {
    
    pub fn new() -> Self {
        Self {
            bitmap: Bitmap::new(),
            buffer: ptr::null_mut(),
            buffer_allocated: false,
        }
    }

    pub fn dealloc_buffer(&mut self) {
        match self.buffer_allocated {
            true => {
                let buf_phys = ptr_mut_to_addr::<[u8; 2 * MEMORY_DEFAULT_PAGE_USIZE], PhysAddr>(self.buffer);

                // dealloc the pages
                unsafe { KERNEL_BASE_VAS_4.lock().as_mut().unwrap().base_page_table.as_mut().unwrap()
                    .dealloc_pages_contiguous(buf_phys.into(), USIZE_8K, PageSize::Small); }
            },
            false => {
                return;
            }
        }
        self.buffer = ptr::null_mut();
    }

    // at this point we are still in uefi boot services mode
    // as soon as the frame allocator is initialized, we will
    // exit uefi boot services mode
    pub fn init(&mut self) {
        let mut mm: MemoryMap;
        
        #[cfg(debug_assertions)]
        serial_println!("allocating memory for uefi memory map");

        // Allocate memory for the bitmap using UEFI allocate_pages.
        let tst = unsafe { UEFI_SYSTEM_TABLE_0.lock().as_mut().unwrap().unsafe_clone() };

        let mm_result = tst.boot_services().allocate_pages(
            AllocateType::AnyPages,
            MemoryType::custom(MEMORY_TYPE_UEFI_MEM_MAP), // Use the custom memory type
            2,
        );

        match mm_result {
            Ok(frame) => {
                self.buffer = addr_to_ptr_mut::<[u8; 2 * MEMORY_DEFAULT_PAGE_USIZE], PhysAddr>(PhysAddr(frame as usize));
            },
            Err(_) => panic!("Failed to allocate temporary storage for uefi memory map")
        }

        // Read the memory map an initial time so we know how large our bitmap needs to be
        // to cover all of physical memory.
        mm = unsafe { UEFI_SYSTEM_TABLE_0.lock().as_ref().unwrap().boot_services().memory_map(self.buffer.as_mut().unwrap()).unwrap_or_else(
            |e| {
                panic!("Physical frame allocator failed to read the UEFI memory map: {:?}", e);
            }
        ) };

        // Find the highest physical address in the memory map. (CONVENTIONAL memory only)
        let mut max_phys_present: usize = 0;
        for e in mm.entries() {
            #[cfg(debug_assertions)]
            serial_println!("ty: {:?} ps: 0x{:08x} pc: {} flags: {:?}", e.ty, e.phys_start, e.page_count, e.att);
            
            if e.ty == MemoryType::CONVENTIONAL {
                max_phys_present = e.phys_start as usize + (MEMORY_DEFAULT_PAGE_USIZE * e.page_count as usize) - 1;
            }
        }

        #[cfg(debug_assertions)]
        serial_println!("max conventional address: 0x{:0x}", max_phys_present);

        let max_phys_uintn_idx = calc_pages_reqd(max_phys_present) / MACHINE_UBITS;
        
        #[cfg(debug_assertions)]
        serial_println!("max physical bitmap index: 0x{:0x}", max_phys_uintn_idx);

        // store max phys addr in global
        {
            let mut max_phys = unsafe { PHYS_MEM_MAX_2.lock() };
            *max_phys = Some(PhysAddr(max_phys_present));
        }
        // store max phys addr idx in global
        {
            let mut max_phys_idx = unsafe { PHYS_MEM_MAX_UINTN_IDX_2.lock() };
            *max_phys_idx = Some(max_phys_uintn_idx);
        }

        #[cfg(debug_assertions)]
        serial_println!("allocating memory for physical frame allocator");

        // pages req'd for bitmap = pages to cover physical memory / bits per Uintn
        let pages_reqd = Bitmap::calc_size_in_default_pages(calc_pages_reqd(max_phys_present));

        // Allocate memory for the bitmap using UEFI allocate_pages.
        let result = unsafe { UEFI_SYSTEM_TABLE_0.lock().as_ref().unwrap().boot_services().allocate_pages(
            AllocateType::AnyPages,
            MemoryType::custom(MEMORY_TYPE_BOOT_FRAMER), // Use the custom memory type
            pages_reqd,
        ) };

        if result.is_err() {
            panic!("failed to allocate {} pages for physical frame allocator: {:?}", pages_reqd, result.err().unwrap());
        }

        #[cfg(debug_assertions)]
        serial_println!("physical frame allocator memory allocated");

        #[cfg(debug_assertions)]
        serial_println!("attaching physical frame allocator to allocated memory");

        match result {
            Ok(frame) => {
                self.bitmap.init(
                    (max_phys_uintn_idx + 1) * MACHINE_UBITS, 
                    VirtAddr(0), 
                    VirtAddr(frame as usize)
                );
            }
            Err(_) => panic!("Failed to allocate memory for physical frame allocator"),
        }

        #[cfg(debug_assertions)]
        serial_println!("terminating uefi services");

        // exit uefi            
        (_, mm) = unsafe { UEFI_SYSTEM_TABLE_0.lock().as_ref().unwrap().unsafe_clone().exit_boot_services(MemoryType::custom(MEMORY_TYPE_UEFI_MEM_MAP)) };

        // store memory map in global
        let mut map = unsafe { UEFI_MEMORY_MAP_1.lock() };
        *map = Some(mm);

        #[cfg(debug_assertions)]
        serial_println!("uefi services terminated");

        #[cfg(debug_assertions)]
        serial_println!("clearing frame allocator memory");

        // clear the bitmap (set all memory to allocated)
        self.bitmap.clear_all();

        #[cfg(debug_assertions)]
        serial_println!("configuring frame allocator for installed memory & peripherals");

        // Initialize the bitmap based on the UEFI memory map.
        for descriptor in (*map).as_ref().unwrap().entries() {
            if descriptor.ty == MemoryType::CONVENTIONAL {
                let start_page =
                    descriptor.phys_start as usize / MEMORY_DEFAULT_PAGE_USIZE;
                let end_page = start_page
                    + descriptor.page_count as usize;
                        
                self.bitmap.set_range(start_page, end_page);
            }
        }
    }

    pub fn alloc_page(&mut self) -> Option<PhysAddr> {
        // Find an available page in the bitmap.
        let first_frame = self.bitmap.find_first_set();

        match first_frame {
            Some(frame) => {
                // Mark the page as allocated in the bitmap.
                self.bitmap.clear(frame);

                Some(PhysAddr(frame * MEMORY_DEFAULT_PAGE_USIZE))
            },
            None => None,
        }
    }

    pub fn dealloc_page(&mut self, page_base: PhysAddr) {
        self.bitmap.set(page_base.as_usize() / MEMORY_DEFAULT_PAGE_USIZE);
    }

    pub fn free_page_count(&self) -> usize {
        self.bitmap.bit_set_count()
    }

    pub fn alloc_contiguous(&mut self, size: usize) -> Option<PhysAddr> {
        let page_count = calc_pages_reqd(size);
        let frame_base = self.bitmap.find_first_set_region(page_count);
        
        match frame_base {
            Some(frame) => {
                // Mark the page as allocated in the bitmap.
                self.bitmap.clear_range(frame, frame + page_count);

                Some(PhysAddr(frame * MEMORY_DEFAULT_PAGE_USIZE))
            },
            None => None,
        }
    }

    pub fn alloc_contiguous_page_aligned(&mut self, size: usize, page_size: PageSize) -> Option<PhysAddr> {
        let mut current_page_idx = 0usize;
        let reqd_page_count = calc_pages_reqd(size);
        let max_phys_idx = unsafe { PHYS_MEM_MAX_UINTN_IDX_2.lock().as_ref().unwrap().clone() };

        while current_page_idx <= max_phys_idx
        {
            let try_alloc_page = self.bitmap.find_set_region_in_range(
                current_page_idx,
                reqd_page_count,
                Range { start: current_page_idx, end: current_page_idx });

            if try_alloc_page.is_some() {
                // allocate
                self.bitmap.clear_range(current_page_idx, try_alloc_page.unwrap());
                return Some(PhysAddr(try_alloc_page.unwrap() * MEMORY_DEFAULT_PAGE_USIZE));
            }

            // no dice this time, so increment the current_page_idx to the next
            // idx that is page_size page aligned
            current_page_idx += page_size.into_bits() / MEMORY_DEFAULT_PAGE_USIZE;
        }
        None
    }

    pub fn dealloc_contiguous(&mut self, page_base: PhysAddr, size: usize) {
        let page_count = calc_pages_reqd(size);
        let start_idx = page_base.as_usize() / MEMORY_DEFAULT_PAGE_USIZE;
        self.bitmap.set_range(start_idx, start_idx + page_count);
    }

    pub fn is_memory_frame_free(&self, page_base: PhysAddr) -> bool {
        self.bitmap.is_set(page_base.as_usize() / MEMORY_DEFAULT_PAGE_USIZE)
    }

    pub fn is_frame_index_free(&self, page_idx: usize) -> bool {
        self.bitmap.is_set(page_idx)
    }
}