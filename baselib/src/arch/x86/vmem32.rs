#![allow(dead_code)]

use uefi::table::boot::MemoryType;

use crate::common::*;

use crate::arch::x86::asm::{x86_invalidate_page, x86_write_cr3};

use core::ptr;

// CONSTANTS

// STATICS

// THE REST
type Cr3 = PhysAddr;

pub struct Vas {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub cr3: Cr3,
    pub base_page_table: *mut BasePageTable,
}
impl Vas {
    pub fn new() -> Self {
        Vas {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            cr3: PhysAddr(0),
            base_page_table: ptr::null_mut(),
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn init_cr3(&mut self) -> PhysAddr {
        let mut p = ref_type_to_addr::<BasePageTable, PhysAddr>(unsafe { self.base_page_table.as_mut().unwrap() });
        p.inner_or(PAGING_WRITETHROUGH);
        self.cr3 = p;
        p
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn switch_to(&mut self) {
        self.init_cr3();
        
        if self.cr3 == PhysAddr(0) { panic!("Tried to switch to a VAS with a null cr3"); }

        x86_write_cr3(self.cr3.as_usize());
    }

    pub fn identity_map_based_on_memory_map(&mut self) {
        for e in unsafe { UEFI_MEMORY_MAP_1.lock().as_ref().unwrap().entries() } {
            if e.ty != MemoryType::CONVENTIONAL {
                for i in 0..e.page_count as usize {
                    let page_start = e.phys_start as usize + (i * MEMORY_DEFAULT_PAGE_USIZE);

                    unsafe {
                        self.base_page_table.as_mut().unwrap().identity_map_page(
                            PhysAddr(page_start),
                            PageSize::Small,
                            PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);
                        }
                }
            }
        }
    }
}

// Page dir / table entries
// Level 4 - 512G, Level 3 - 1G, Level 2 - 2M, Level 1 - 4K
// x86 just has levels 2 (@4MB pages) & 1 (still 4KB pages)
pub type Pte = PhysAddr;

#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PageSize {
    Small = USIZE_4K,
    Medium = USIZE_4M,
}
impl PageSize {
    pub const fn into_bits(self) -> usize {
        self as _
    }

    pub const fn from_bits(value: usize) -> Self {
        match value {
            USIZE_4K => PageSize::Small,
            USIZE_4M => PageSize::Medium,
            _ => PageSize::Small,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct PageTable {    
    pub entries: [Pte; X86_PAGE_TABLE_MAX_ENTRIES],
}

pub type BasePageTable = PageTable;

impl PageDir for BasePageTable {
    fn new_base() -> PhysAddr {
        let new_pd_base = unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().alloc_page() };
                
        match new_pd_base {
            None => panic!("Out of memory when allocating for a new pd for a new vas"),
            Some(np) => {
                serial_println!("new pd @ {:?}", np);
                let pd = addr_to_ref_type_mut::<BasePageTable, PhysAddr>(np);
                pd.identity_map_page(np, PageSize::Small, PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);
                new_pd_base.unwrap()
            },
        }
    }

    fn virt_to_phys(&self, v: VirtAddr) -> PhysAddr {
        let (pd_idx, pt_idx) = v.get_page_table_indexes();

        let pt: &mut PageTable;

        // check our entry in the pd table, which maps 4MB chunks
        if self.entries[pd_idx] == PhysAddr(0) {
            // if the entry is 0, then there's nothing to do
            return PhysAddr(0);
        } 

        // see if we're getting a 4MB page. if so, return it
        if is_bit_set_usize(self.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            return self.entries[pd_idx].align_4m();
        } // PageSize::Medium

        // create a reference to our pt
        pt = addr_to_ref_type_mut::<PageTable, PhysAddr>(self.entries[pd_idx].align_4k());

        // return our small page
        return pt.entries[pt_idx].align_4k();
    }

    fn map_page(&mut self, p: PhysAddr, v: VirtAddr, size: PageSize, flags: usize) -> Option<VirtAddr> {
        
        let (pd_idx, pt_idx) = v.get_page_table_indexes();

        let pt: &mut PageTable;

        match size {
            PageSize::Small => {
                if !v.is_aligned_4k() { return None; }
            },
            PageSize::Medium => {
                if !v.is_aligned_4m() { return None; }
            },
        }

        // see if we're doing a 4MB page. if so, mark it as a page and clean up if necessary
        if size == PageSize::Medium {
            
            // Since we're mapping a medium page, we need to remove any mappings that may have been
            // present as 4KB pages underneath, so we don't have a memory leak
            if self.entries[pd_idx] != PhysAddr(0) {
                // all of the entries in the page directory either point to page tables that themselves point
                // to 4KB pages, or they point to 4MB page frame entries. We need to de-allocate the page table
                // entry if it's not a page frame entry
                if !is_bit_set_usize(self.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                    // de-allocate the page table
                    unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(self.entries[pd_idx].align_4k()) }
                    self.unmap_page(VirtAddr(self.entries[pd_idx].align_4k().as_usize()), PageSize::Small);
                }
            }

            // Map our medium page
            self.entries[pd_idx] = p;
            self.entries[pd_idx].inner_or(
                flags |
                PAGING_IS_PAGE_FRAME
            );

            // Now that we've cleaned up the already mapped pages,
            // and we've mapped the 4MB page, we can return
            x86_invalidate_page(v.as_usize());
            return Some(v);
        } // PageSize::Medium

        // check our entry in the pd table, which maps 4MB chunks
        // create a new pd if one does not exist
        if self.entries[pd_idx] == PhysAddr(0) {
            let new_pt_base = unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().alloc_page() };
            
            match new_pt_base {
                None => return None,
                Some(np) => {
                    self.entries[pd_idx] = np;
                    self.identity_map_page(np, PageSize::Small, PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);
                    self.entries[pd_idx].inner_or(PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);
                },
            }
        }

        // create a reference to our pt
        pt = addr_to_ref_type_mut::<PageTable, PhysAddr>(self.entries[pd_idx].align_4k());

        // This must be a 4KB page

        // we already know we have a good pd & pt, so just map the page
        // no page frame flag for 4KB pages
        pt.entries[pt_idx] = p;
        pt.entries[pt_idx].inner_or(
            flags
        );
        Some(v)
    }

    fn unmap_page(&mut self, v: VirtAddr, size: PageSize) {
        let (pd_idx, pt_idx) = v.get_page_table_indexes();

        let pt: &mut PageTable;

        match size {
            PageSize::Small => {
                if !v.is_aligned_4k() { panic!("In x86, small pages must be 4KB aligned to unmap"); }
            },
            PageSize::Medium => {
                if !v.is_aligned_4m() { panic!("In x86, medium pages must be 4MB aligned to unmap"); }
            },
        }

        // check our entry in the pd table, which maps 4MB chunks

        // if the page directory is already 0, then there's nothing to do
        if self.entries[pd_idx] == PhysAddr(0) {
            // if the entry is already 0, then there's nothing to do
            return;
        } 

        // if this is a 4MB page, all we need to do is set it to 0 and leave
        if is_bit_set_usize(self.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            self.entries[pd_idx] = PhysAddr(0);
            x86_invalidate_page(v.as_usize());
            return;
        }

        // this must be a 4KB page

        // create a reference to our pt
        pt = addr_to_ref_type_mut::<PageTable, PhysAddr>(self.entries[pd_idx].align_4k());

        // Unmap our small page
        pt.entries[pt_idx] = PhysAddr(0);
        x86_invalidate_page(v.as_usize());

    }

    fn identity_map_page(&mut self, p: PhysAddr, size: PageSize, flags: usize) {
        self.map_page(p, VirtAddr(p.align_4k().as_usize()), size, flags);
    }

    fn alloc_page(&mut self, v: VirtAddr, size: PageSize, flags: usize) -> VirtAddr {
        let new_page_frame_base = unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().alloc_page() };
            
        // Looks like we were not able to obtain a page
        if new_page_frame_base.is_none() { return VirtAddr(0); }

        // Map the new page frame to where it was requested
        self.map_page(new_page_frame_base.unwrap(), v, size, flags);

        // Return the virtual address of the new page frame
        v
    }

    fn alloc_pages(&mut self, size_in_pages: usize, v: VirtAddr, page_size: PageSize, flags: usize) -> Option<VirtAddr> {
        let mut allocated_pages: usize = 0;

        for i in 0..size_in_pages {
            let page_base = unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().alloc_page() };
            if page_base.is_some() {
                allocated_pages += 1;

                self.map_page(page_base.unwrap(), 
                    VirtAddr(v.as_usize() + (i * page_size.into_bits())), 
                    page_size, 
                    flags
                ); 
            } else {
                // we need to deallocate and unmap any pages that failed and return failure
                let mut cv: VirtAddr = v;

                for _j in 0..allocated_pages {
                    self.unmap_page(cv, page_size); 
                    unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(self.virt_to_phys(cv)); }
                    
                    cv.inner_inc_by_default_page_size();
                }

                return None;
            }
        }

        Some(v)
    }

    fn alloc_pages_contiguous(&mut self, size_in_pages: usize, v: VirtAddr, page_size: PageSize, flags: usize) -> Option<VirtAddr> {
        let mut allocated_pages: usize = 0;

        for i in 0..size_in_pages {
            let page_base = unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().alloc_page() };
            if page_base.is_some() {
                allocated_pages += 1;

                self.map_page(page_base.unwrap(), 
                    VirtAddr(v.as_usize() + (i * page_size.into_bits())), 
                    page_size, 
                    flags
                ); 
            } else {
                // we need to deallocate and unmap any pages that failed and return failure
                let mut cv: VirtAddr = v;

                for _j in 0..allocated_pages {
                    self.unmap_page(cv, page_size); 
                    unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(self.virt_to_phys(cv)); }
                    
                    cv.inner_inc_by_default_page_size();
                }

                return None;
            }
        }

        Some(v)
    }
}