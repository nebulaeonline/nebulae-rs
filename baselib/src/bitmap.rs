use crate::common::*;

use core::ptr;
use core::slice;
use core::ops::Range;

pub trait BitmapOps {
    // base_vaddr is ignored if pre_allocated_base is not 0
    fn new() -> Bitmap;
    fn init(&mut self, item_cap: usize, base_vaddr: VirtAddr, pre_allocated_base: VirtAddr) -> bool;

    fn size_in_uintn(&self) -> usize;
    fn size_in_pages(&self) -> usize;
    fn size_in_bytes(&self) -> usize;

    fn calc_size_in_uintn(capacity: usize) -> usize;
    fn calc_size_in_default_pages(capacity: usize) -> usize;
    fn calc_item_index(item: usize) -> usize;
    fn calc_item_bit_index(item: usize) -> usize;

    fn get_bitmap_ref_mut(&mut self) -> &mut [Uintn];
    fn get_bitmap_ref(&self) -> & [Uintn];

    fn set(&mut self, item: usize);
    fn clear(&mut self, item: usize);
    fn is_set(&self, item: usize) -> bool;
    fn is_clear(&self, item: usize) -> bool;
    fn capacity(&self) -> usize;
    fn bit_set_count(&self) -> usize;
    fn bit_clear_count(&self) -> usize;
    fn set_range(&mut self, start_item: usize, end_item: usize);
    fn clear_range(&mut self, start_item: usize, end_item: usize);
    fn set_all(&mut self);
    fn clear_all(&mut self);
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
    bitmap: *mut Uintn,
    capacity_in_units: usize,
    units_free: usize,
    size_in_uintn: usize,
    size_in_pages: usize,
    size_in_bytes: usize,
    init: bool,
}
impl BitmapOps for Bitmap {
    fn new() -> Bitmap {
        Bitmap {
            bitmap: ptr::null_mut(),
            capacity_in_units: 0,
            units_free: 0,
            size_in_uintn: 0,
            size_in_pages: 0,
            size_in_bytes: 0,
            init: false,
        }
    }

    // init without specifying a pre-allocated base (VirtAddr(0)) MUST NOT be used before the virtual memory
    // subsystem is initialized; that code path depends on map_page() being available
    fn init(&mut self, item_cap: usize, base_vaddr: VirtAddr, pre_allocated_base: VirtAddr) -> bool {

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
            self.bitmap = addr_to_ptr_mut::<Uintn, VirtAddr>(bitmap_phys_base.unwrap().into());
        } else {
            self.bitmap = addr_to_ptr_mut::<Uintn, VirtAddr>(pre_allocated_base.into());
        }

        self.capacity_in_units = item_cap;
        self.units_free = item_cap;
        self.size_in_uintn = size_in_uintn;
        self.size_in_pages = size_in_pages;
        self.size_in_bytes = size_in_bytes;
        self.init = true;
        true
    }

    #[inline(always)]
    fn size_in_uintn(&self) -> usize {
        debug_assert!(self.init);
        self.size_in_uintn
    }

    #[inline(always)]
    fn size_in_pages(&self) -> usize {
        debug_assert!(self.init);
        self.size_in_pages
    }

    #[inline(always)]
    fn size_in_bytes(&self) -> usize {
        debug_assert!(self.init);
        self.size_in_bytes
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
    fn get_bitmap_ref_mut(&mut self) -> &mut [Uintn] {
        debug_assert!(self.init);

        unsafe { slice::from_raw_parts_mut::<Uintn>(self.bitmap, self.size_in_uintn()) }
    }

    #[inline(always)]
    fn get_bitmap_ref(&self) -> & [Uintn] {
        debug_assert!(self.init);

        unsafe { slice::from_raw_parts::<Uintn>(self.bitmap, self.size_in_uintn()) }
    }

    fn set(&mut self, item: usize) {
        debug_assert!(self.init);

        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        let bitmap_ref = self.get_bitmap_ref_mut();
        bitmap_ref[item_index] |= 1 << item_bit_index;
        self.units_free -= 1;
    }

    fn clear(&mut self, item: usize) {
        debug_assert!(self.init);

        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        let bitmap_ref = self.get_bitmap_ref_mut();
        bitmap_ref[item_index] &= !(1 << item_bit_index);
        self.units_free += 1;
    }

    fn is_set(&self, item: usize) -> bool {
        debug_assert!(self.init);

        let item_index = Bitmap::calc_item_index(item);
        let item_bit_index = Bitmap::calc_item_bit_index(item);

        let bitmap_ref = self.get_bitmap_ref();
        (bitmap_ref[item_index] & (1 << item_bit_index)) != 0
    }

    fn is_clear(&self, item: usize) -> bool {
        debug_assert!(self.init);

        !self.is_set(item)
    }

    fn capacity(&self) -> usize {
        debug_assert!(self.init);

        self.capacity_in_units
    }

    fn bit_set_count(&self) -> usize {
        debug_assert!(self.init);

        self.capacity_in_units - self.units_free
    }

    fn bit_clear_count(&self) -> usize {
        debug_assert!(self.init);

        self.units_free
    }

    fn set_range(&mut self, start_item: usize, end_item: usize) {
        debug_assert!(self.init);

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
        self.units_free -= (end_item - start_item) + 1;
    }

    fn clear_range(&mut self, start_item: usize, end_item: usize) {
        debug_assert!(self.init);

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
        self.units_free += (end_item - start_item) + 1;
    }

    fn set_all(&mut self) {
        debug_assert!(self.init);

        let uintn_idx_max = self.size_in_uintn();
        let bitmap = self.get_bitmap_ref_mut();
        for i in 0..uintn_idx_max {
            bitmap[i] = Uintn::MAX;
        }
        self.units_free = 0;
    }

    fn clear_all(&mut self) {
        debug_assert!(self.init);

        let uintn_idx_max = self.size_in_uintn();
        let bitmap = self.get_bitmap_ref_mut();
        for i in 0..uintn_idx_max {
            bitmap[i] = Uintn::MIN;
        }
        self.units_free = self.capacity_in_units;
    }

    fn is_empty(&self) -> bool {
        debug_assert!(self.init);

        let bitmap = self.get_bitmap_ref();
        for i in 0..self.size_in_uintn() {
            if bitmap[i] != Uintn::MIN {
                return false;
            }
        }
        true
    }

    fn is_full(&self) -> bool {
        debug_assert!(self.init);

        let bitmap = self.get_bitmap_ref();
        for i in 0..self.size_in_uintn() {
            if bitmap[i] != Uintn::MAX {
                return false;
            }
        }
        true
    }

    fn find_first_set(&self) -> Option<usize> {
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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
        debug_assert!(self.init);

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