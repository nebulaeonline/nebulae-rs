use crate::common::base::*;
use crate::kernel_statics::*;

use core::cell::Cell;
use core::ops::Range;
use core::ptr;
use core::slice;

pub trait BitmapOps {
    fn new(owner: Owner) -> Bitmap;
    fn get_owner() -> Owner;

    fn init_phys_fixed(&self, item_cap: usize, base_addr: PhysAddr) -> bool;
    fn init_phys_frame(&self, item_cap: usize) -> bool;

    fn init_virt_frame(&self, item_cap: usize, base_addr: VirtAddr) -> bool;
    fn init_virt_vmem_fixed(&self, item_cap: usize, base_addr: VirtAddr) -> bool;
    fn init_virt_direct(&self, item_cap: usize, base_addr: VirtAddr) -> bool;
    fn init_virt_vmem(&self, item_cap: usize) -> bool;

    fn size_in_usize(&self) -> usize;
    fn size_in_pages(&self) -> usize;
    fn size_in_bytes(&self) -> usize;

    fn get_bitmap_ref_mut(&self) -> &mut [usize];
    fn get_bitmap_ref(&self) -> &[usize];

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
    fn find_set_region_in_range(
        &self,
        item: usize,
        reqd_item_count: usize,
        range: Range<usize>,
    ) -> Option<usize>;
    fn find_clear_region_in_range(
        &self,
        item: usize,
        reqd_item_count: usize,
        range: Range<usize>,
    ) -> Option<usize>;
}

#[repr(usize)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BitmapTyp {
    PhysFixed,
    PhysFrame,
    VirtFrame,
    VirtFixed,
    VirtDirect,
    VirtVmem,
}

pub struct Bitmap {
    bitmap: Cell<*mut usize>,
    capacity_in_units: Cell<usize>,
    units_free: Cell<usize>,
    size_in_usize: Cell<usize>,
    size_in_pages: Cell<usize>,
    size_in_bytes: Cell<usize>,
    typ: Cell<BitmapTyp>,
    owner: Cell<Owner>,
}
impl Drop for Bitmap {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        serial_println!(
            "bitmap::drop(): bitmap: 0x{:016x}, typ: {:?}",
            self.bitmap.get() as usize,
            self.typ.get()
        );

        // if our bitmap is not null, then we need to free the memory
        // associated with the bitmap or else we will leak memory
        if self.bitmap.get() != ptr::null_mut() && *self.typ.get_mut() != BitmapTyp::PhysFixed {
            // we no longer have to worry about the frame allocator,
            // as all of its functions are now handled by vmem
            unsafe {
                KERNEL_BASE_VAS_4
                    .lock()
                    .as_mut()
                    .unwrap()
                    .base_page_table
                    .as_mut()
                    .unwrap()
                    .dealloc_pages_contiguous(
                        raw::ptr_to_raw::<usize, VirtAddr>(self.bitmap.get() as *const usize),
                        self.size_in_pages.get(),
                        self.owner.get(),
                        PageSize::Small,
                    );
            }
            self.bitmap.set(ptr::null_mut());
        }
    }
}

impl BitmapOps for Bitmap {
    fn new(owner: Owner) -> Bitmap {
        #[cfg(debug_assertions)]
        serial_println!("bitmap::new(): owner: {:?}", owner);
        
        Bitmap {
            bitmap: Cell::new(ptr::null_mut()),
            capacity_in_units: Cell::new(0),
            units_free: Cell::new(0),
            size_in_usize: Cell::new(0),
            size_in_pages: Cell::new(0),
            size_in_bytes: Cell::new(0),
            typ: Cell::new(BitmapTyp::PhysFixed),
            owner: Cell::new(owner),
        }
    }

    // returns the owner
    fn get_owner() -> Owner {
        Owner::System
    }

    // this function assumes there's enough runway to allocate the bitmap
    fn init_phys_fixed(&self, item_cap: usize, base_addr: PhysAddr) -> bool {
        #[cfg(debug_assertions)]
        serial_println!(
            "bitmap::init_phys_fixed(): item_cap: {}, base_addr: 0x{:016x}",
            item_cap,
            base_addr.as_usize()
        );

        // figure out how many usize's we need to cover the
        // requested capacity
        let size_in_usize = bitindex::calc_bitindex_size_in_usize(item_cap);

        // calculate the size in bytes and pages
        let size_in_bytes = bitindex::calc_bitindex_size_in_bytes(item_cap);
        let size_in_pages = pages::calc_pages_reqd(size_in_bytes, PageSize::Small);

        self.capacity_in_units.set(item_cap);
        self.units_free.set(item_cap);
        self.size_in_usize.set(size_in_usize);
        self.size_in_pages.set(size_in_pages);
        self.size_in_bytes.set(size_in_bytes);

        // set our raw pointer to the base address
        self.bitmap
            .set(raw::raw_to_ptr_mut::<usize, PhysAddr>(base_addr.into()));

        true
    }

    // this function attempts to allocate space for the bitmap
    // from the physical frame allocator. if it fails, it will
    // return false. the bitmap is not fit for use.
    fn init_phys_frame(&self, item_cap: usize) -> bool {
        // figure out how many usize's we need to cover the
        // requested capacity
        let size_in_usize = bitindex::calc_bitindex_size_in_usize(item_cap);

        // calculate the size in bytes and pages
        let size_in_bytes = bitindex::calc_bitindex_size_in_bytes(item_cap);
        let size_in_pages = pages::calc_pages_reqd(size_in_bytes, PageSize::Small);

        // we will first try to get contiguous memory for the bitmap.
        let bitmap_phys_base = unsafe {
            FRAME_ALLOCATOR_3
                .lock()
                .as_mut()
                .unwrap()
                .alloc_default_pages(size_in_bytes, Owner::System)
        };

        if bitmap_phys_base.is_some() {
            // we were able to successfully allocate contiguous memory for the bitmap
            // now map the pages used for the bitmap
            for i in (bitmap_phys_base.unwrap().as_usize()
                ..(bitmap_phys_base.unwrap().as_usize() + size_in_bytes))
                .step_by(MEMORY_DEFAULT_PAGE_USIZE)
            {
                unsafe {
                    KERNEL_BASE_VAS_4
                        .lock()
                        .as_mut()
                        .unwrap()
                        .base_page_table
                        .as_mut()
                        .unwrap()
                        .identity_map_page(
                            PhysAddr(i),
                            self.owner.get(),
                            PageSize::Small,
                            PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                        );
                }
            }
        } else {
            // unfortunately, a bitmap doesn't work without contiguous memory, and
            // this being a physical allocation, we can't just map the pages contiguously
            return false;
        }

        self.capacity_in_units.set(item_cap);
        self.units_free.set(item_cap);
        self.size_in_usize.set(size_in_usize);
        self.size_in_pages.set(size_in_pages);
        self.size_in_bytes.set(size_in_bytes);

        self.bitmap.set(raw::raw_to_ptr_mut::<usize, VirtAddr>(
            bitmap_phys_base.unwrap().into(),
        ));

        true
    }

    // this function initializes the bitmap using virtual memory, but allocating
    // via the frame allocator. this is useful for bootstrapping the memory subsystem.
    fn init_virt_frame(&self, item_cap: usize, base_addr: VirtAddr) -> bool {
        // figure out how many usize's we need to cover the
        // requested capacity
        let size_in_usize = bitindex::calc_bitindex_size_in_usize(item_cap);

        // calculate the size in bytes and pages
        let size_in_bytes = bitindex::calc_bitindex_size_in_usize(size_in_usize);
        let size_in_pages = pages::calc_pages_reqd(size_in_bytes, PageSize::Small);

        // if the pre-allocated base is 0, that means we need to allocate directly from the
        // frame allocator; if it's not 0, then the memory has already been allocated for us
        // at that address
        let mut virt_base = base_addr;

        // we will first try to get contiguous memory for the bitmap. if that fails, we will
        // fall back to allocating pages individually
        let bitmap_phys_base = unsafe {
            FRAME_ALLOCATOR_3
                .lock()
                .as_mut()
                .unwrap()
                .alloc_default_pages(size_in_bytes, Owner::System)
        };

        if bitmap_phys_base.is_some() {
            // we were able to successfully allocate contiguous memory for the bitmap
            // now map the pages used for the bitmap
            for i in (bitmap_phys_base.unwrap().as_usize()
                ..(bitmap_phys_base.unwrap().as_usize() + size_in_bytes))
                .step_by(MEMORY_DEFAULT_PAGE_USIZE)
            {
                unsafe {
                    KERNEL_BASE_VAS_4
                        .lock()
                        .as_mut()
                        .unwrap()
                        .base_page_table
                        .as_mut()
                        .unwrap()
                        .map_page(
                            PhysAddr(i),
                            virt_base,
                            self.owner.get(),
                            PageSize::Small,
                            PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                        );
                }
                virt_base.inner_inc_by_page_size(PageSize::Small);
            }
        } else {
            // we need to try and map the pages individually
            let mut allocated_pages: usize = 0;

            for _i in 0..size_in_pages {
                let page_base = unsafe {
                    FRAME_ALLOCATOR_3
                        .lock()
                        .as_mut()
                        .unwrap()
                        .alloc_page(Owner::System, PageSize::Small)
                };
                if page_base.is_some() {
                    allocated_pages += 1;

                    unsafe {
                        KERNEL_BASE_VAS_4
                            .lock()
                            .as_mut()
                            .unwrap()
                            .base_page_table
                            .as_mut()
                            .unwrap()
                            .map_page(
                                page_base.unwrap(),
                                virt_base,
                                self.owner.get(),
                                PageSize::Small,
                                PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                            );
                    }
                    virt_base.inner_inc_by_page_size(PageSize::Small);
                } else {
                    // we failed allocating individually, so we need to free the pages we did allocate
                    // and return false
                    virt_base = base_addr;

                    for _j in 0..allocated_pages {
                        unsafe {
                            let phys_page = KERNEL_BASE_VAS_4
                                .lock()
                                .as_mut()
                                .unwrap()
                                .base_page_table
                                .as_mut()
                                .unwrap()
                                .virt_to_phys(virt_base);

                            KERNEL_BASE_VAS_4
                                .lock()
                                .as_mut()
                                .unwrap()
                                .base_page_table
                                .as_mut()
                                .unwrap()
                                .unmap_page(virt_base, self.owner.get(), PageSize::Small);

                            FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(
                                phys_page,
                                Owner::System,
                                PageSize::Small,
                            );
                        }
                        virt_base.inner_inc_by_page_size(PageSize::Small);
                    }
                    return false;
                }
            }
        }
        self.bitmap.set(raw::raw_to_ptr_mut::<usize, VirtAddr>(
            bitmap_phys_base.unwrap().into(),
        ));

        self.capacity_in_units.set(item_cap);
        self.units_free.set(item_cap);
        self.size_in_usize.set(size_in_usize);
        self.size_in_pages.set(size_in_pages);
        self.size_in_bytes.set(size_in_bytes);
        self.typ.set(BitmapTyp::VirtFrame);
        true
    }

    // this function initializes the bitmap using virtual memory to a
    // fixed location.
    fn init_virt_vmem_fixed(&self, item_cap: usize, base_addr: VirtAddr) -> bool {
        // figure out how many usize's we need to cover the
        // requested capacity
        let size_in_usize = bitindex::calc_bitindex_size_in_usize(item_cap);

        // calculate the size in bytes and pages
        let size_in_bytes = bitindex::calc_bitindex_size_in_usize(size_in_usize);
        let size_in_pages = pages::calc_pages_reqd(size_in_bytes, PageSize::Small);

        let raw_alloc = unsafe {
            KERNEL_BASE_VAS_4
                .lock()
                .as_mut()
                .unwrap()
                .base_page_table
                .as_mut()
                .unwrap()
                .alloc_pages_contiguous_fixed(
                    size_in_pages,
                    base_addr,
                    self.owner.get(),
                    PageSize::Small,
                    0,
                    BitPattern::ZeroZero,
                )
        };

        if raw_alloc.is_some() {
            self.bitmap
                .set(raw::raw_to_ptr_mut::<usize, VirtAddr>(raw_alloc.unwrap()));
        } else {
            return false;
        }

        self.capacity_in_units.set(item_cap);
        self.units_free.set(item_cap);
        self.size_in_usize.set(size_in_usize);
        self.size_in_pages.set(size_in_pages);
        self.size_in_bytes.set(size_in_bytes);
        self.typ.set(BitmapTyp::VirtFixed);

        true
    }

    // this function assumes you've allocated enough runway to init the bitmap
    fn init_virt_direct(&self, item_cap: usize, base_addr: VirtAddr) -> bool {
        // figure out how many usize's we need to cover the
        // requested capacity
        let size_in_usize = bitindex::calc_bitindex_size_in_usize(item_cap);

        // calculate the size in bytes and pages
        let size_in_bytes = bitindex::calc_bitindex_size_in_bytes(item_cap);
        let size_in_pages = pages::calc_pages_reqd(size_in_bytes, PageSize::Small);

        self.capacity_in_units.set(item_cap);
        self.units_free.set(item_cap);
        self.size_in_usize.set(size_in_usize);
        self.size_in_pages.set(size_in_pages);
        self.size_in_bytes.set(size_in_bytes);

        // set our raw pointer to the base address
        self.bitmap
            .set(raw::raw_to_ptr_mut::<usize, VirtAddr>(base_addr.into()));

        true
    }

    // this function initializes the bitmap using virtual memory, allowing the
    // vm subsystem to place the bitmap where it wants.
    // TODO
    #[allow(unused_variables)]
    fn init_virt_vmem(&self, item_cap: usize) -> bool {
        true
    }

    #[inline(always)]
    fn size_in_usize(&self) -> usize {
        debug_assert!(self.bitmap.get() != ptr::null_mut());
        self.size_in_usize.get()
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
    fn get_bitmap_ref_mut(&self) -> &mut [usize] {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        unsafe { slice::from_raw_parts_mut::<usize>(self.bitmap.get(), self.size_in_usize()) }
    }

    #[inline(always)]
    fn get_bitmap_ref(&self) -> &[usize] {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        unsafe { slice::from_raw_parts::<usize>(self.bitmap.get(), self.size_in_usize()) }
    }

    fn set(&self, item: usize) {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        let bitmap_ref = self.get_bitmap_ref_mut();
        bitmap_ref[item_index] |= 1 << item_bit_index;
        self.units_free.set(self.units_free.get() - 1);
    }

    fn clear(&self, item: usize) {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        let bitmap_ref = self.get_bitmap_ref_mut();
        bitmap_ref[item_index] &= !(1 << item_bit_index);
        self.units_free.set(self.units_free.get() + 1);
    }

    fn is_set(&self, item: usize) -> bool {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

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
        } else {
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
                        bitmap[j] |= usize::MAX;
                    }
                } else {
                    for j in (start_index + 1)..=(end_index - 1) {
                        bitmap[j] |= usize::MAX;
                    }
                    for z in 0..=end_bit_index {
                        bitmap[end_index] |= 1 << z;
                    }
                }
            }
        }
        self.units_free
            .set(self.units_free.get() + (end_item - start_item) + 1);
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
        } else {
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
                        bitmap[j] = usize::MIN;
                    }
                } else {
                    for j in (start_index + 1)..=(end_index - 1) {
                        bitmap[j] = usize::MIN;
                    }
                    for z in 0..=end_bit_index {
                        bitmap[end_index] &= !(1 << z);
                    }
                }
            }
        }
        self.units_free
            .set(self.units_free.get() - (end_item - start_item) + 1);
    }

    fn set_all(&self) {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let usize_idx_max = self.size_in_usize();
        let bitmap = self.get_bitmap_ref_mut();
        for i in 0..usize_idx_max {
            bitmap[i] = usize::MAX;
        }
        self.units_free.set(0);
    }

    fn clear_all(&self) {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let usize_idx_max = self.size_in_usize();
        let bitmap = self.get_bitmap_ref_mut();
        for i in 0..usize_idx_max {
            bitmap[i] = usize::MIN;
        }
        self.units_free.set(self.capacity_in_units.get());
    }

    fn is_empty(&self) -> bool {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        for i in 0..self.size_in_usize() {
            if bitmap[i] != usize::MIN {
                return false;
            }
        }
        true
    }

    fn is_full(&self) -> bool {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        for i in 0..self.size_in_usize() {
            if bitmap[i] != usize::MAX {
                return false;
            }
        }
        true
    }

    fn find_first_set(&self) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        let bitmap = self.get_bitmap_ref();
        for i in 0..self.size_in_usize() {
            if bitmap[i] != usize::MIN {
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
        for i in 0..self.size_in_usize() {
            if bitmap[i] != usize::MAX {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        for i in item_index..self.size_in_usize() {
            if bitmap[i] != usize::MIN {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        for i in item_index..self.size_in_usize() {
            if bitmap[i] != usize::MAX {
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
        for i in (0..self.size_in_usize()).rev() {
            if bitmap[i] != usize::MIN {
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
        for i in (0..self.size_in_usize()).rev() {
            if bitmap[i] != usize::MAX {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        for i in (0..item_index).rev() {
            if bitmap[i] != usize::MIN {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        for i in (0..item_index).rev() {
            if bitmap[i] != usize::MAX {
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

        for i in 0..self.size_in_usize() {
            if bitmap[i] != usize::MIN {
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

        for i in 0..self.size_in_usize() {
            if bitmap[i] != usize::MAX {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);
        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in item_index..self.size_in_usize() {
            if bitmap[i] != usize::MIN {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in item_index..self.size_in_usize() {
            if bitmap[i] != usize::MAX {
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

        for i in (0..self.size_in_usize()).rev() {
            if bitmap[i] != usize::MIN {
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

        for i in (0..self.size_in_usize()).rev() {
            if bitmap[i] != usize::MAX {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in (0..item_index).rev() {
            if bitmap[i] != usize::MIN {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        for i in (0..item_index).rev() {
            if bitmap[i] != usize::MAX {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        if bitmap[item_index] != usize::MIN {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        if bitmap[item_index] != usize::MAX {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        if bitmap[item_index] != usize::MIN {
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
        let (item_index, item_bit_index) = bitindex::calc_bitindex(item);

        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        if bitmap[item_index] != usize::MAX {
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
    fn find_set_region_in_range(
        &self,
        item: usize,
        reqd_item_count: usize,
        range: Range<usize>,
    ) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        if item > range.end {
            return None;
        }

        let bitmap = self.get_bitmap_ref();
        let (item_index, item_bit_index) = if item < range.start {
            bitindex::calc_bitindex(range.start)
        } else {
            bitindex::calc_bitindex(item)
        };
        let (end_index, _) = bitindex::calc_bitindex(range.end);

        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        // we start at either the item's position, or range.start, whichever
        // is greater; then we iterate through the bitmap until we either
        // find a set region with the required number of items, or we reach
        // range.end. if we reach range.end, we return None.
        for i in item_index..self.size_in_usize() {
            if !found && i > end_index {
                return None;
            }

            if bitmap[i] == usize::MIN {
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
    fn find_clear_region_in_range(
        &self,
        item: usize,
        reqd_item_count: usize,
        range: Range<usize>,
    ) -> Option<usize> {
        debug_assert!(self.bitmap.get() != ptr::null_mut());

        if item > range.end {
            return None;
        }

        let bitmap = self.get_bitmap_ref();
        let (item_index, item_bit_index) = if item < range.start {
            bitindex::calc_bitindex(range.start)
        } else {
            bitindex::calc_bitindex(item)
        };
        let (end_index, _) = bitindex::calc_bitindex(range.end);

        let mut found = false;
        let mut start = 0;
        let mut end = 0;

        // we start at either the item's position, or range.start, whichever
        // is greater; then we iterate through the bitmap until we either
        // find a clear region with the required number of items, or we reach
        // range.end. if we reach range.end, we return None.
        for i in item_index..self.size_in_usize() {
            if !found && i > end_index {
                return None;
            }

            if bitmap[i] == usize::MAX {
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
