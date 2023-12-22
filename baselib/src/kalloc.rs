use crate::structures::bitmap::*;
use crate::common::base::*;

use crate::vmem::*;

//use core::alloc::{GlobalAlloc, Layout};
use core::alloc::Layout;

trait SubAllocator {
    fn internal_alloc(&self, layout: Layout) -> Option<VirtAddr>;
    fn internal_dealloc(&self, ptr: *mut u8, layout: Layout) -> bool;
    fn internal_alloc_zeroed(&self, layout: Layout) -> Option<VirtAddr>;
    fn internal_realloc(
        &self,
        ptr: *mut u8,
        old_layout: Layout,
        new_size: usize,
    ) -> Option<VirtAddr>;

    fn capacity(&self) -> usize;
    fn used(&self) -> usize;
    fn free(&self) -> usize;
}

// TODO: this needs to be updated when the more robust page size handling code is in place
#[allow(dead_code)]
pub struct MemoryPool {
    name: &'static str,
    //backing_store: PhysAddr,
    page_size: PageSize,
    block_size: usize,
    capacity: usize,
    start: VirtAddr,
    bitmap: Bitmap,
    bitmap_vaddr: VirtAddr,
}
impl MemoryPool {
    #[inline(always)]
    pub fn calc_bitmap_size_in_pages(capacity: usize) -> usize {
        bitindex::calc_bitindex_size_in_pages(capacity, PageSize::Small)
    }

    #[inline(always)]
    pub fn calc_bitmap_size_in_usize(capacity: usize) -> usize {
        bitindex::calc_bitindex_size_in_usize(capacity)
    }

    #[inline(always)]
    pub fn calc_size_in_bytes(capacity: usize, block_size: usize) -> usize {
        block_size * capacity
    }

    #[inline(always)]
    pub fn calc_size_in_pages(capacity: usize, block_size: usize, page_size: PageSize) -> usize {
        pages::calc_pages_reqd(capacity * block_size, page_size)
    }

    pub fn new(
        name: &'static str,
        page_size: PageSize,
        block_size: usize,
        capacity: usize,
        start: VirtAddr,
        bitmap_start: VirtAddr,
    ) -> Self {
        MemoryPool {
            name: name,
            page_size: page_size,
            block_size: block_size,
            capacity: capacity,
            start: start,
            bitmap: Bitmap::new(Owner::Memory),
            bitmap_vaddr: bitmap_start,
        }
    }

    pub fn init(&mut self) -> Option<VirtAddr> {
        // first try and allocate contiguous memory, then fall back to non-contiguous
        let contiguous = {
            iron().base_vas_0_5
                .lock()
                .as_mut()
                .unwrap()
                .base_page_table
                .as_mut()
                .unwrap()
                .alloc_pages_fixed_virtual(
                    pages::calc_pages_reqd(self.capacity * self.block_size, PageSize::Small),
                    self.start,
                    Owner::System,
                    PageSize::Small,
                    PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                    BitPattern::ZeroZero,
                )
        };

        if contiguous.is_some() {
            self.bitmap
                .init_virt_vmem_fixed(self.capacity, self.bitmap_vaddr);
            self.bitmap.set_all();
            return contiguous;
        } else {
            let nc = {
                iron().base_vas_0_5
                    .lock()
                    .as_mut()
                    .unwrap()
                    .base_page_table
                    .as_mut()
                    .unwrap()
                    .alloc_pages_fixed(
                        self.capacity * self.block_size,
                        self.start,
                        Owner::System,
                        PageSize::Small,
                        PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                        BitPattern::ZeroZero,
                    )
            };

            if nc.is_some() {
                self.bitmap
                    .init_virt_vmem_fixed(self.capacity, self.bitmap_vaddr);
                self.bitmap.set_all();
                return nc;
            } else {
                return None;
            }
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        // // we need to free the memory we allocated for the pool;
        // // the bitmap will be de-allocated when the bitmap is dropped
        // // as part of its own custom Drop implementation
        // unsafe {
        //     KERNEL_BASE_VAS_4
        //         .lock()
        //         .as_mut()
        //         .unwrap()
        //         .base_page_table
        //         .as_mut()
        //         .unwrap()
        //         .dealloc_pages_contiguous(
        //             self.start,
        //             self.capacity * self.block_size,
        //             Owner::System,
        //             PageSize::Small,
        //         )
        // };
    }
}

impl SubAllocator for MemoryPool {
    fn internal_alloc(&self, layout: Layout) -> Option<VirtAddr> {
        if layout.size() > self.block_size {
            return None;
        }

        let free_idx = self.bitmap.find_first_set();
        if free_idx.is_some() {
            let free_addr = self.start.as_usize() + (free_idx.unwrap() * self.block_size);
            self.bitmap.clear(free_idx.unwrap());
            return Some(free_addr.as_virt());
        } else {
            return None;
        }
    }

    fn internal_dealloc(&self, ptr: *mut u8, layout: Layout) -> bool {
        let dealloc_idx = (ptr as usize - self.start.as_usize()) / self.block_size;
        self.bitmap.set(dealloc_idx);

        // clear the memory
        let dealloc_begin = ptr as usize;
        let dealloc_end = dealloc_begin + layout.size();
        for i in dealloc_begin..dealloc_end {
            unsafe { core::ptr::write_volatile(i as *mut u8, 0) };
        }
        return true;
    }

    fn internal_alloc_zeroed(&self, layout: Layout) -> Option<VirtAddr> {
        let alloc = self.internal_alloc(layout);

        if alloc.is_none() {
            return None;
        }

        let alloc_begin = alloc.unwrap().as_usize();
        let alloc_end = alloc_begin + layout.size();
        for i in alloc_begin..alloc_end {
            unsafe { core::ptr::write_volatile(i as *mut u8, 0) };
        }
        alloc
    }

    // Memory pool don't do no re-allocin'
    fn internal_realloc(
        &self,
        ptr: *mut u8,
        _old_layout: Layout,
        _new_size: usize,
    ) -> Option<VirtAddr> {
        Some(VirtAddr(ptr as usize))
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn used(&self) -> usize {
        self.bitmap.bit_clear_count()
    }

    fn free(&self) -> usize {
        self.bitmap.bit_set_count()
    }
}

// unsafe impl GlobalAlloc for MemoryPool {
//     unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
//         let alloc = self.internal_alloc(layout);
//         if alloc.is_some() {
//             return addr_to_ptr_mut::<u8, VirtAddr>(alloc.unwrap());
//         } else {
//             return core::ptr::null_mut();
//         }
//     }

//     unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
//         self.internal_dealloc(ptr, layout);
//     }

//     unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
//         let alloc = self.internal_alloc_zeroed(layout);
//         if alloc.is_some() {
//             return addr_to_ptr_mut::<u8, VirtAddr>(alloc.unwrap());
//         } else {
//             return core::ptr::null_mut();
//         }
//     }

//     unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
//         let realloc = self.internal_realloc(ptr, old_layout, new_size);
//         if realloc.is_some() {
//             return addr_to_ptr_mut::<u8, VirtAddr>(realloc.unwrap());
//         } else {
//             return core::ptr::null_mut();
//         }
//     }
// }
