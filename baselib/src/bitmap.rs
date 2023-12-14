use crate::common::*;

use core::ptr;
use core::slice;
use core::ops::Range;
use core::cell::Cell;

pub trait BitmapOps {
    // base_vaddr is ignored if pre_allocated_base is not 0
    fn new() -> Bitmap;
    fn init(&self, item_cap: usize, base_vaddr: VirtAddr, pre_allocated_base: VirtAddr) -> bool;

    fn size_in_uintn(&self) -> usize;
    fn size_in_pages(&self) -> usize;
    fn size_in_bytes(&self) -> usize;

    fn calc_size_in_uintn(capacity: usize) -> usize;
    fn calc_size_in_default_pages(capacity: usize) -> usize;
    fn calc_item_index(item: usize) -> usize;
    fn calc_item_bit_index(item: usize) -> usize;

    fn get_bitmap_ref_mut(&self) -> &mut [Uintn];
    fn get_bitmap_ref(&self) -> & [Uintn];

    fn set(&self, item: usize);
    fn clear(&self, item: usize);
    fn is_set(&self, item: usize) -> bool;
    fn is_clear(&self, item: usize) -> bool;
    fn capacity(&self) -> usize;
    fn bit_set_count(&self) -> usize;
    fn bit_clear_count(&self) -> usize;
    fn set_range(&self, start_item: usize, end_item: usize);
    fn clear_range(&self, start_item: usize, end_item: usize);
    fn set_all(&self);
    fn clear_all(&self);
    fn is_empty(&self) -> bool;
    fn is_full(&self) -> bool;
    fn find_first_set(&self) -> Option<usize>;
    fn find_first_clear(&self) -> Option<usize>;
    fn find_next_set(&self, item: usize) -> Option<usize>;
    fn find_next_clear(&self, item: usize) -> Option<usize>;
    fn find_last_set(&self) -> Option<usize>;
    fn find_last_clear(&self) -> Option<usize>;
    fn find_prev_set(&self, item: usize) -> Option<usize>;
    fn find_prev_clear(&self, item: usize) -> Option<usize>;
    fn find_first_set_region(&self, reqd_item_count: usize) -> Option<usize>;
    fn find_first_clear_region(&self, reqd_item_count: usize) -> Option<usize>;
    fn find_next_set_region(&self, item: usize, reqd_item_count: usize) -> Option<usize>;
    fn find_next_clear_region(&self, item: usize, reqd_item_count: usize) -> Option<usize>;
    fn find_last_set_region(&self, reqd_item_count: usize) -> Option<usize>;
    fn find_last_clear_region(&self, reqd_item_count: usize) -> Option<usize>;
    fn find_prev_set_region(&self, item: usize, reqd_item_count: usize) -> Option<usize>;
    fn find_prev_clear_region(&self, item: usize, reqd_item_count: usize) -> Option<usize>;
    fn find_set_from_item(&self, item: usize) -> Option<usize>;
    fn find_clear_from_item(&self, item: usize) -> Option<usize>;
    fn find_set_region(&self, item: usize, reqd_item_count: usize) -> Option<usize>;
    fn find_clear_region(&self, item: usize, reqd_item_count: usize) -> Option<usize>;
    fn find_set_region_in_range(&self, item: usize, reqd_item_count: usize, range: Range<usize>) -> Option<usize>;
    fn find_clear_region_in_range(&self, item: usize, reqd_item_count: usize, range: Range<usize>) -> Option<usize>;
}

pub struct Bitmap {
    bitmap: Cell<*mut Uintn>,
    capacity_in_units: Cell<usize>,
    units_free: Cell<usize>,
    size_in_uintn: Cell<usize>,
    size_in_pages: Cell<usize>,
    size_in_bytes: Cell<usize>,
}
impl Drop for Bitmap {
    
    fn drop(&mut self) {
        // if our bitmap is not null, then we need to free the memory
        // associated with the bitmap or else we will leak memory
        if self.bitmap.get() != ptr::null_mut() {
            // we no longer have to worry about the frame allocator,
            // as all of its functions are now handled by vmem
            unsafe { KERNEL_BASE_VAS_4.lock().as_mut().unwrap().base_page_table.as_mut().unwrap()
                .dealloc_pages_contiguous(ptr_to_addr::<Uintn, VirtAddr>(self.bitmap.get() as *const Uintn), self.size_in_pages.get(), PageSize::Small); 
            }
            self.bitmap.set(ptr::null_mut());
        }
    }
}
impl BitmapOps for Bitmap {
    fn new() -> Bitmap {
        Bitmap {
            bitmap: Cell::new(ptr::null_mut()),
            capacity_in_units: Cell::new(0),
            units_free: Cell::new(0),
            size_in_uintn: Cell::new(0),
            size_in_pages: Cell::new(0),
            size_in_bytes: Cell::new(0),
        }
    }

    // init without specifying a pre-allocated base (VirtAddr(0)) MUST NOT be used before the virtual memory
    // subsystem is initialized; that code path depends on map_page() being available
    fn init(&self, item_cap: usize, base_vaddr: VirtAddr, pre_allocated_base: VirtAddr) -> bool {

        // figure out how many uintn's we need to cover the
        // requested capacity
        let size_in_uintn = Bitmap::calc_size_in_uintn(item_cap);

        // calculate the size in bytes and pages
        let size_in_bytes = size_in_uintn * MACHINE_UBYTES;
        let size_in_pages = calc_pages_reqd(size_in_bytes);

        // if the pre-allocated base is 0, that means we need to allocate directly from the
        // frame allocator; if it's not 0, then the memory has already been allocated for us
        // at that address
        let mut virt_base = base_vaddr;

        if pre_allocated_base.as_usize() == 0 {
            // we will first try to get contiguous memory for the bitmap. if that fails, we will
            // fall back to allocating pages individually
            let bitmap_phys_base = unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap()
                .alloc_contiguous(size_in_bytes) };

            if bitmap_phys_base.is_some() {
                // we were able to successfully allocate contiguous memory for the bitmap
                // now map the pages used for the bitmap
                for i in (bitmap_phys_base.unwrap().as_usize()..(bitmap_phys_base.unwrap().as_usize() + size_in_bytes)).step_by(MEMORY_DEFAULT_PAGE_USIZE) {
                    unsafe { 
                        KERNEL_BASE_VAS_4.lock().as_mut().unwrap().base_page_table.as_mut().unwrap()
                            .map_page(PhysAddr(i), 
                                virt_base, 
                                PageSize::Small, 
                                PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH,
                            );                        
                    }
                    virt_base.inner_inc_by_default_page_size();
                }                                    
            } else {
                // we need to try and map the pages individually
                let mut allocated_pages: usize = 0;

                for _i in 0..size_in_pages {
                    let page_base = unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().alloc_page() };
                    if page_base.is_some() {
                        allocated_pages += 1;

                        unsafe { 
                            KERNEL_BASE_VAS_4.lock().as_mut().unwrap().base_page_table.as_mut().unwrap()
                                .map_page(page_base.unwrap(), 
                                    virt_base, 
                                    PageSize::Small, 
                                    PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH
                                ); 
                        }
                        virt_base.inner_inc_by_default_page_size();
                    } else {
                        // we failed allocating individually, so we need to free the pages we did allocate
                        // and return false
                        virt_base = base_vaddr;

                        for _j in 0..allocated_pages {
                            unsafe { 
                                let phys_page = KERNEL_BASE_VAS_4.lock().as_mut().unwrap().base_page_table.as_mut().unwrap()
                                    .virt_to_phys(virt_base);
                                
                                KERNEL_BASE_VAS_4.lock().as_mut().unwrap().base_page_table.as_mut().unwrap()
                                    .unmap_page(virt_base, PageSize::Small);
                                
                                FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(phys_page);  
                            }
                            virt_base.inner_inc_by_default_page_size();
                        }
                        return false;
                    }
                }
            }
            self.bitmap.set(addr_to_ptr_mut::<Uintn, VirtAddr>(bitmap_phys_base.unwrap().into()));
        } else {
            self.bitmap.set(addr_to_ptr_mut::<Uintn, VirtAddr>(pre_allocated_base.into()));
        }

        self.capacity_in_units.set(item_cap);
        self.units_free.set(item_cap);
        self.size_in_uintn.set(size_in_uintn);
        self.size_in_pages.set(size_in_pages);
        self.size_in_bytes.set(size_in_bytes);

        true
    }

    #[inline(always)]
    fn size_in_uintn(&self) -> usize {
        debug_assert!(self.bitmap.get() != ptr::null_mut());
        self.size_in_uintn.get()
    }

    #[inline(always)]
    fn size_in_pages(&self) -> usize {
        debug_assert!(self.bitmap.get() != ptr::null_mut());
        self.size_in_pages.get()
    }

    #[inline(always)]
    fn size_in_bytes(&self) -> usize {
        debug_assert!(self.bitmap.get() != ptr::null_mut());
        self.size_in_bytes.get()
    }
    
    #[inline(always)]
    fn calc_size_in_uintn(capacity: usize) -> usize {
        (capacity + (MACHINE_UBITS - 1)) / MACHINE_UBITS
    }

    #[inline(always)]
    fn calc_size_in_default_pages(capacity: usize) -> usize {
        let uintn_per_page = MEMORY_DEFAULT_PAGE_USIZE / MACHINE_UBYTES;
        Bitmap::calc_size_in_uintn(capacity) / uintn_per_page
    }

    #[inline(always)]
    fn calc_item_index(item: usize) -> usize {
        item / MACHINE_UBITS
    }

    #[inline(always)]
    fn calc_item_bit_index(item: usize) -> usize {
        item % MACHINE_UBITS
    }

    #[inline(always)]
    fn get_bitmap_ref_mut(&self) -> &mut [Uintn] {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        unsafe { slice::from_raw_parts_mut::<Uintn>(self.bitmap.get(), self.size_in_uintn()) }
    }

    #[inline(always)]
    fn get_bitmap_ref(&self) -> & [Uintn] {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        unsafe { slice::from_raw_parts::<Uintn>(self.bitmap.get(), self.size_in_uintn()) }
    }

    fn set(&self, item: usize) {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        let bitmap_ref = self.get_bitmap_ref_mut();
        bitmap_ref[item_index] |= 1 << item_bit_index;
        self.units_free.set(self.units_free.get() - 1);
    }

    fn clear(&self, item: usize) {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        let bitmap_ref = self.get_bitmap_ref_mut();
        bitmap_ref[item_index] &= !(1 << item_bit_index);
        self.units_free.set(self.units_free.get() + 1);
    }

    fn is_set(&self, item: usize) -> bool {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        let bitmap_ref = self.get_bitmap_ref();
        (bitmap_ref[item_index] & (1 << item_bit_index)) != 0
    }

    fn is_clear(&self, item: usize) -> bool {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        !self.is_set(item)
    }

    fn capacity(&self) -> usize {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        self.capacity_in_units.get()
    }

    fn bit_set_count(&self) -> usize {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        self.capacity_in_units.get() - self.units_free.get()
    }

    fn bit_clear_count(&self) -> usize {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        self.units_free.get()
    }

    fn set_range(&self, start_item: usize, end_item: usize) {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref_mut();

        // Indexes
        let start_index = start_item / MACHINE_UBITS;
        let start_bit_index = start_item % MACHINE_UBITS;
        let end_index = end_item / MACHINE_UBITS;
        let end_bit_index = end_item % MACHINE_UBITS;

        // Set the range
        if start_index == end_index {
            if start_bit_index == end_bit_index {
                bitmap[start_index] |= 1 << start_bit_index;
            } else {
                for z in start_bit_index..end_bit_index {
                    bitmap[start_index] |= 1 << z;
                }                        
            }
        }
        else {
            for z in start_bit_index..MACHINE_UBITS {
                bitmap[start_index] |= 1 << z;                    
            }
            if start_index + 1 == end_index {
                for j in 0..=end_bit_index {
                    bitmap[end_index] |= 1 << j;
                }
            } else {
                if end_bit_index == 0 {
                    for j in (start_index + 1)..=end_index {
                        bitmap[j] |= Uintn::MAX;
                    }
                } else {
                    for j in (start_index + 1)..=(end_index - 1) {
                        bitmap[j] |= Uintn::MAX;
                    }
                    for z in 0..=end_bit_index {
                        bitmap[end_index] |= 1 << z;
                    }
                }
            }
        }
        self.units_free.set(self.units_free.get() - (end_item - start_item) + 1);
    }

    fn clear_range(&self, start_item: usize, end_item: usize) {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref_mut();
        
        // Indexes
        let start_index = start_item / MACHINE_UBITS;
        let start_bit_index = start_item % MACHINE_UBITS;
        let end_index = end_item / MACHINE_UBITS;
        let end_bit_index = end_item % MACHINE_UBITS;

        // Set the range
        if start_index == end_index {
            if start_bit_index == end_bit_index {
                bitmap[start_index] &= !(1 << start_bit_index);
            } else {
                for z in start_bit_index..end_bit_index {
                    bitmap[start_index] &= !(1 << z);
                }                        
            }
        }
        else {
            for z in start_bit_index..MACHINE_UBITS {
                bitmap[start_index] &= !(1 << z);                    
            }
            if start_index + 1 == end_index {
                for j in 0..=end_bit_index {
                    bitmap[end_index] &= !(1 << j);
                }
            } else {
                if end_bit_index == 0 {
                    for j in (start_index + 1)..=end_index {
                        bitmap[j] = Uintn::MIN;
                    }
                } else {
                    for j in (start_index + 1)..=(end_index - 1) {
                        bitmap[j] = Uintn::MIN;
                    }
                    for z in 0..=end_bit_index {
                        bitmap[end_index] &= !(1 << z);
                    }
                }
            }
        }
        self.units_free.set(self.units_free.get() + (end_item - start_item) + 1);
    }

    fn set_all(&self) {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let uintn_idx_max = self.size_in_uintn();
        let bitmap = self.get_bitmap_ref_mut();
        for i in 0..uintn_idx_max {
            bitmap[i] = Uintn::MAX;
        }
        self.units_free.set(0);
    }

    fn clear_all(&self) {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let uintn_idx_max = self.size_in_uintn();
        let bitmap = self.get_bitmap_ref_mut();
        for i in 0..uintn_idx_max {
            bitmap[i] = Uintn::MIN;
        }
        self.units_free.set(self.capacity_in_units.get());
    }

    fn is_empty(&self) -> bool {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        for i in 0..self.size_in_uintn() {
            if bitmap[i] != Uintn::MIN {
                return false;
            }
        }
        true
    }

    fn is_full(&self) -> bool {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        for i in 0..self.size_in_uintn() {
            if bitmap[i] != Uintn::MAX {
                return false;
            }
        }
        true
    }

    fn find_first_set(&self) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        for i in 0..self.size_in_uintn() {
            if bitmap[i] != Uintn::MIN {
                for j in 0..MACHINE_UBITS {
                    if (bitmap[i] & (1 << j)) != 0 {
                        return Some((i * MACHINE_UBITS) + j);
                    }
                }
            }
        }
        None
    }

    fn find_first_clear(&self) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        for i in 0..self.size_in_uintn() {
            if bitmap[i] != Uintn::MAX {
                for j in 0..MACHINE_UBITS {
                    if (bitmap[i] & (1 << j)) == 0 {
                        return Some((i * MACHINE_UBITS) + j);
                    }
                }
            }
        }
        None
    }

    fn find_next_set(&self, item: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        for i in item_index..self.size_in_uintn() {
            if bitmap[i] != Uintn::MIN {
                for j in item_bit_index..MACHINE_UBITS {
                    if (bitmap[i] & (1 << j)) != 0 {
                        return Some((i * MACHINE_UBITS) + j);
                    }
                }
            }
        }
        None
    }

    fn find_next_clear(&self, item: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        for i in item_index..self.size_in_uintn() {
            if bitmap[i] != Uintn::MAX {
                for j in item_bit_index..MACHINE_UBITS {
                    if (bitmap[i] & (1 << j)) == 0 {
                        return Some((i * MACHINE_UBITS) + j);
                    }
                }
            }
        }
        None
    }

    fn find_last_set(&self) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        for i in (0..self.size_in_uintn()).rev() {
            if bitmap[i] != Uintn::MIN {
                for j in (0..MACHINE_UBITS).rev() {
                    if (bitmap[i] & (1 << j)) != 0 {
                        return Some((i * MACHINE_UBITS) + j);
                    }
                }
            }
        }
        None
    }

    fn find_last_clear(&self) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        for i in (0..self.size_in_uintn()).rev() {
            if bitmap[i] != Uintn::MAX {
                for j in (0..MACHINE_UBITS).rev() {
                    if (bitmap[i] & (1 << j)) == 0 {
                        return Some((i * MACHINE_UBITS) + j);
                    }
                }
            }
        }
        None
    }

    fn find_prev_set(&self, item: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        for i in (0..item_index).rev() {
            if bitmap[i] != Uintn::MIN {
                for j in (0..item_bit_index).rev() {
                    if (bitmap[i] & (1 << j)) != 0 {
                        return Some((i * MACHINE_UBITS) + j);
                    }
                }
            }
        }
        None
    }

    fn find_prev_clear(&self, item: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        for i in (0..item_index).rev() {
            if bitmap[i] != Uintn::MAX {
                for j in (0..item_bit_index).rev() {
                    if (bitmap[i] & (1 << j)) == 0 {
                        return Some((i * MACHINE_UBITS) + j);
                    }
                }
            }
        }
        None
    }

    fn find_first_set_region(&self, reqd_item_count: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in 0..self.size_in_uintn() {
            if bitmap[i] != Uintn::MIN {
                for j in 0..MACHINE_UBITS {
                    if (bitmap[i] & (1 << j)) != 0 {
                        if !found {
                            start = (i * MACHINE_UBITS) + j;
                            end = start;
                            found = true;
                        } else {
                            end += 1;
                        }
                    } else {
                        if found {
                            if (end - start) >= reqd_item_count {
                                return Some(start);
                            } else {
                                found = false;
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn find_first_clear_region(&self, reqd_item_count: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in 0..self.size_in_uintn() {
            if bitmap[i] != Uintn::MAX {
                for j in 0..MACHINE_UBITS {
                    if (bitmap[i] & (1 << j)) == 0 {
                        if !found {
                            start = (i * MACHINE_UBITS) + j;
                            end = start;
                            found = true;
                        } else {
                            end += 1;
                        }
                    } else {
                        if found {
                            if (end - start) >= reqd_item_count {
                                return Some(start);
                            } else {
                                found = false;
                            }
                        }
                    }
                }
            }
        }
        None
    }
    
    fn find_next_set_region(&self, item: usize, reqd_item_count: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);
        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in item_index..self.size_in_uintn() {
            if bitmap[i] != Uintn::MIN {
                for j in item_bit_index..MACHINE_UBITS {
                    if (bitmap[i] & (1 << j)) != 0 {
                        if !found {
                            start = (i * MACHINE_UBITS) + j;
                            end = start;
                            found = true;
                        } else {
                            end += 1;
                        }
                    } else {
                        if found {
                            if (end - start) >= reqd_item_count {
                                return Some(start);
                            } else {
                                found = false;
                            }
                        }
                    }
                }
            }
        }
        None
    }
    
    fn find_next_clear_region(&self, item: usize, reqd_item_count: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);
        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in item_index..self.size_in_uintn() {
            if bitmap[i] != Uintn::MAX {
                for j in item_bit_index..MACHINE_UBITS {
                    if (bitmap[i] & (1 << j)) == 0 {
                        if !found {
                            start = (i * MACHINE_UBITS) + j;
                            end = start;
                            found = true;
                        } else {
                            end += 1;
                        }
                    } else {
                        if found {
                            if (end - start) >= reqd_item_count {
                                return Some(start);
                            } else {
                                found = false;
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn find_last_set_region(&self, reqd_item_count: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in (0..self.size_in_uintn()).rev() {
            if bitmap[i] != Uintn::MIN {
                for j in (0..MACHINE_UBITS).rev() {
                    if (bitmap[i] & (1 << j)) != 0 {
                        if !found {
                            start = (i * MACHINE_UBITS) + j;
                            end = start;
                            found = true;
                        } else {
                            end -= 1;
                        }
                    } else {
                        if found {
                            if (start - end) >= reqd_item_count {
                                return Some(start);
                            } else {
                                found = false;
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn find_last_clear_region(&self, reqd_item_count: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in (0..self.size_in_uintn()).rev() {
            if bitmap[i] != Uintn::MAX {
                for j in (0..MACHINE_UBITS).rev() {
                    if (bitmap[i] & (1 << j)) == 0 {
                        if !found {
                            start = (i * MACHINE_UBITS) + j;
                            end = start;
                            found = true;
                        } else {
                            end -= 1;
                        }
                    } else {
                        if found {
                            if (start - end) >= reqd_item_count {
                                return Some(start);
                            } else {
                                found = false;
                            }
                        }
                    }
                }
            }
        }
        None
    }
    
    fn find_prev_set_region(&self, item: usize, reqd_item_count: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);
        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in (0..item_index).rev() {
            if bitmap[i] != Uintn::MIN {
                for j in (0..item_bit_index).rev() {
                    if (bitmap[i] & (1 << j)) != 0 {
                        if !found {
                            start = (i * MACHINE_UBITS) + j;
                            end = start;
                            found = true;
                        } else {
                            end -= 1;
                        }
                    } else {
                        if found {
                            if (start - end) >= reqd_item_count {
                                return Some(start);
                            } else {
                                found = false;
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn find_prev_clear_region(&self, item: usize, reqd_item_count: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);
        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in (0..item_index).rev() {
            if bitmap[i] != Uintn::MAX {
                for j in (0..item_bit_index).rev() {
                    if (bitmap[i] & (1 << j)) == 0 {
                        if !found {
                            start = (i * MACHINE_UBITS) + j;
                            end = start;
                            found = true;
                        } else {
                            end -= 1;
                        }
                    } else {
                        if found {
                            if (start - end) >= reqd_item_count {
                                return Some(start);
                            } else {
                                found = false;
                            }
                        }
                    }
                }
            }
        }
        None
    }
    
    fn find_set_from_item(&self, item: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        if bitmap[item_index] != Uintn::MIN {
            for j in item_bit_index..MACHINE_UBITS {
                if (bitmap[item_index] & (1 << j)) != 0 {
                    return Some((item_index * MACHINE_UBITS) + j);
                }
            }
        }
        None
    }
    
    fn find_clear_from_item(&self, item: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        if bitmap[item_index] != Uintn::MAX {
            for j in item_bit_index..MACHINE_UBITS {
                if (bitmap[item_index] & (1 << j)) == 0 {
                    return Some((item_index * MACHINE_UBITS) + j);
                }
            }
        }
        None
    }
    
    fn find_set_region(&self, item: usize, reqd_item_count: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);
        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        if bitmap[item_index] != Uintn::MIN {
            for j in item_bit_index..MACHINE_UBITS {
                if (bitmap[item_index] & (1 << j)) != 0 {
                    if !found {
                        start = (item_index * MACHINE_UBITS) + j;
                        end = start;
                        found = true;
                    } else {
                        end += 1;
                    }
                } else {
                    if found {
                        if (end - start) >= reqd_item_count {
                            return Some(start);
                        } else {
                            found = false;
                        }
                    }
                }
            }
        }
        None
    }

    fn find_clear_region(&self, item: usize, reqd_item_count: usize) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);
        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        if bitmap[item_index] != Uintn::MAX {
            for j in item_bit_index..MACHINE_UBITS {
                if (bitmap[item_index] & (1 << j)) == 0 {
                    if !found {
                        start = (item_index * MACHINE_UBITS) + j;
                        end = start;
                        found = true;
                    } else {
                        end += 1;
                    }
                } else {
                    if found {
                        if (end - start) >= reqd_item_count {
                            return Some(start);
                        } else {
                            found = false;
                        }
                    }
                }
            }
        }
        None
    }
    
    // Starting with the specified item slot, will search for a set region of size
    // reqd_item_count. This will short circuit if item > range.end.
    // The range.start and range.end are bounds ONLY for the start of the set region.
    // The start plus the region's item count required can exceed region.end.
    fn find_set_region_in_range(&self, item: usize, reqd_item_count: usize, range: Range<usize>) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        if item > range.end {
            return None;
        }

        let bitmap = self.get_bitmap_ref();
        let item_index = if item < range.start {
            Bitmap::calc_item_index(range.start)
        } else {
            Bitmap::calc_item_index(item)
        };
        let item_bit_index = if item < range.start {
            Bitmap::calc_item_bit_index(range.start)
        } else {
            Bitmap::calc_item_bit_index(item)
        };
        let end_index = Bitmap::calc_item_index(range.end);

        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        // we start at either the item's position, or range.start, whichever
        // is greater; then we iterate through the bitmap until we either
        // find a set region with the required number of items, or we reach
        // range.end. if we reach range.end, we return None.
        for i in item_index..self.size_in_uintn(){
            if !found && i > end_index {
                return None;
            }

            if bitmap[i] == Uintn::MIN {
                found = false;
                continue;
            }

            for j in item_bit_index..MACHINE_UBITS {
                if (bitmap[i] & (1 << j)) != 0 {
                    if !found {
                        start = (i * MACHINE_UBITS) + j;
                        end = start;
                        found = true;
                    } else {
                        end += 1;
                    }
                } else {
                    if found {
                        if (end - start) >= reqd_item_count {
                            if start >= range.start && start <= range.end {
                                return Some(start);
                            } else {
                                found = false;
                            }
                        } else {
                            found = false;
                        }
                    }
                }
            }
        }
        None
    }
    
    // Starting with the specified item slot, will search for a clear region of size
    // reqd_item_count. This will short circuit if item > range.end.
    // The range.start and range.end are bounds ONLY for the start of the clear region.
    // The start plus the region's item count required can exceed region.end.
    fn find_clear_region_in_range(&self, item: usize, reqd_item_count: usize, range: Range<usize>) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        if item > range.end {
            return None;
        }

        let bitmap = self.get_bitmap_ref();
        let item_index = if item < range.start {
            Bitmap::calc_item_index(range.start)
        } else {
            Bitmap::calc_item_index(item)
        };
        let item_bit_index = if item < range.start {
            Bitmap::calc_item_bit_index(range.start)
        } else {
            Bitmap::calc_item_bit_index(item)
        };
        let end_index = Bitmap::calc_item_index(range.end);

        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        // we start at either the item's position, or range.start, whichever
        // is greater; then we iterate through the bitmap until we either
        // find a clear region with the required number of items, or we reach
        // range.end. if we reach range.end, we return None.
        for i in item_index..self.size_in_uintn(){
            if !found && i > end_index {
                return None;
            }

            if bitmap[i] == Uintn::MAX {
                found = false;
                continue;
            }

            for j in item_bit_index..MACHINE_UBITS {
                if (bitmap[i] & (1 << j)) == 0 {
                    if !found {
                        start = (i * MACHINE_UBITS) + j;
                        end = start;
                        found = true;
                    } else {
                        end += 1;
                    }
                } else {
                    if found {
                        if (end - start) >= reqd_item_count {
                            if start >= range.start && start <= range.end {
                                return Some(start);
                            } else {
                                found = false;
                            }
                        } else {
                            found = false;
                        }
                    }
                }
            }
        }
        None
    }
}