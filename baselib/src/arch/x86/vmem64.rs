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
    pub cr3: Cr3,
    pub base_page_table: *mut BasePageTable,
}
impl Vas {
    pub fn new() -> Self {
        Vas {
            cr3: PhysAddr(0),
            base_page_table: ptr::null_mut(),
        }
    }

    pub fn init_cr3(&mut self) -> PhysAddr {
        let mut p = ref_type_to_addr::<BasePageTable, PhysAddr>(unsafe { self.base_page_table.as_mut().unwrap() });
        p.inner_or(PAGING_WRITETHROUGH);
        self.cr3 = p;
        p
    }

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
    Medium = USIZE_2M,
    Huge = USIZE_1G,
}
impl PageSize {
    pub const fn into_bits(self) -> usize {
        self as _
    }

    pub const fn from_bits(value: usize) -> Self {
        match value {
            USIZE_4K => PageSize::Small,
            USIZE_2M => PageSize::Medium,
            USIZE_1G => PageSize::Huge,
            _ => PageSize::Small,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct PageTable {    
    pub entries: [Pte; PAGE_TABLE_MAX_ENTRIES],
}

pub type BasePageTable = PageTable;

impl PageDir for BasePageTable {
    fn new_base() -> PhysAddr {
        
        let new_pml4_base = unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().alloc_page() };
                
        match new_pml4_base {
            None => panic!("Out of memory when allocating for a new pml4 for a new vas"),
            Some(np) => {
                serial_println!("new pml4 @ {:?}", np);
                let pml4 = addr_to_ref_type_mut::<BasePageTable, PhysAddr>(np);
                pml4.identity_map_page(np, PageSize::Small, PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);
                new_pml4_base.unwrap()
            },
        }
    }

    fn virt_to_phys(&self, v: VirtAddr) -> PhysAddr {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = v.get_page_table_indexes();

        let pdpt: &mut PageTable;
        let pd: &mut PageTable;
        let pt: &mut PageTable;

        // check our entry in the pml4 table, which maps 512GB chunks
        if self.entries[pml4_idx] == PhysAddr(0) {
            // if the entry is 0, then there's nothing to do
            return PhysAddr(0);
        } 

        // check our entry in the pdpt table, which maps 1GB chunks
        // create a reference to our pdpt
        pdpt = addr_to_ref_type_mut::<PageTable, PhysAddr>(self.entries[pml4_idx].align_4k());

        // see if we're getting a 1GB page. if so, return it
        if is_bit_set_usize(pdpt.entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            return pdpt.entries[pdpt_idx].align_1g();
        } // PageSize::Huge
        
        if pdpt.entries[pdpt_idx] == PhysAddr(0) {
            // if the entry is 0, then there's nothing to do
            return PhysAddr(0);
        }

        // create a reference to our pd
        pd = addr_to_ref_type_mut::<PageTable, PhysAddr>(pdpt.entries[pdpt_idx].align_4k());

        // see if we're getting a 2MB page. if so, return it
        if is_bit_set_usize(pd.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            return pd.entries[pd_idx].align_2m();
        } // PageSize::Medium

        if pd.entries[pd_idx] == PhysAddr(0) {
            // if the entry is 0, then there's nothing to do
            return PhysAddr(0);
        }

        // This is a 4KB page

        // create a reference to our pt
        pt = addr_to_ref_type_mut::<PageTable, PhysAddr>(pd.entries[pd_idx].align_4k());

        // return our small page
        return pt.entries[pt_idx].align_4k();
    }

    fn map_page(&mut self, p: PhysAddr, v: VirtAddr, size: PageSize, flags: usize) -> Option<VirtAddr> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = v.get_page_table_indexes();

        let pdpt: &mut PageTable;
        let pdpt_is_new: bool;
        let pd: &mut PageTable;
        let pd_is_new: bool;
        let pt: &mut PageTable;

        match size {
            PageSize::Small => {
                if !p.is_aligned_4k() { return None; }
            },
            PageSize::Medium => {
                if !p.is_aligned_2m() { return None; }
            },
            PageSize::Huge => {
                if !p.is_aligned_1g() { return None; }
            }
        }

        // check our entry in the pml4 table, which maps 512GB chunks
        // create a new pdpt if one does not exist
        if self.entries[pml4_idx] == PhysAddr(0) {
            let new_pdpt_base = unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().alloc_page() };
            
            match new_pdpt_base {
                None => return None,
                Some(np) => {
                    self.entries[pml4_idx] = np;
                    self.identity_map_page(np, PageSize::Small, PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);                    
                    self.entries[pml4_idx].inner_or(PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);
                    pdpt_is_new = true;
                }
            }
        } else {
            pdpt_is_new = false;
        }


        // check our entry in the pdpt table, which maps 1GB chunks
        // create a reference to our pdpt
        pdpt = addr_to_ref_type_mut::<PageTable, PhysAddr>(self.entries[pml4_idx].align_4k());

        // see if we're doing a 1GB page. if so, mark it as a page and clean up if necessary
        if size == PageSize::Huge {
            // Since we're mapping a huge page, we need to remove any mappings that may have been
            // present as 2MB or 4KB pages underneath, so we don't have a memory leak
            if !pdpt_is_new { // if we just created this pdpt, then there's nothing to remove

                // don't do anything if this was already a 1GB page, since there will be no pds or pts to remove
                if !is_bit_set_usize(pdpt.entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {

                    // follow the old pdpt entry to its page directory
                    let local_pd = addr_to_ref_type_mut::<PageTable, PhysAddr>(pdpt.entries[pdpt_idx].align_4k());

                    // if they're not 2MB page entries, then we need to
                    // de-allocate every page table under this page directory
                    for i in 0..PAGE_TABLE_MAX_ENTRIES {
                        if !is_bit_set_usize(local_pd.entries[i].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                            unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(local_pd.entries[i].align_4k()); }
                            self.unmap_page(VirtAddr(local_pd.entries[i].align_4k().as_usize()), PageSize::Small);
                        }
                    }

                    // now de-allocate the page directory itself
                    unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(pdpt.entries[pdpt_idx].align_4k()) }
                    self.unmap_page(VirtAddr(pdpt.entries[pdpt_idx].align_4k().as_usize()), PageSize::Small);
                }
            }

            // Map our huge page
            pdpt.entries[pdpt_idx] = p;
            pdpt.entries[pdpt_idx].inner_or(
                flags |
                PAGING_IS_PAGE_FRAME
            );

            // Now that we've cleaned up the already mapped pages,
            // and we've mapped the 1GB page, we can return
            x86_invalidate_page(v.as_usize());
            return Some(v);
        } // PageSize::Huge
        
        // create a new pd if one does not exist
        if pdpt.entries[pdpt_idx] == PhysAddr(0) {
            let new_pd_base = unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().alloc_page() };
            
            match new_pd_base {
                None => return None,
                Some(np) => {
                    pdpt.entries[pdpt_idx] = np;
                    self.identity_map_page(np, PageSize::Small, PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);
                    pdpt.entries[pdpt_idx].inner_or(PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);
                    pd_is_new = true;
                },
            }
        } else {
            pd_is_new = false;
        }

        // create a reference to our pd
        pd = addr_to_ref_type_mut::<PageTable, PhysAddr>(pdpt.entries[pdpt_idx].align_4k());

        // see if we're doing a 2MB page. if so, mark it as a page and clean up if necessary
        if size == PageSize::Medium {
            // Since we're mapping a medium page, we need to remove any mappings that may have been
            // present as 4KB pages underneath, so we don't have a memory leak
            if !pd_is_new { // if we just created this pd, then there's nothing to remove

                {
                    // all of the entries in the page directory are either page tables that point
                    // to 4KB pages, or they are 2MB page table entries. We need to de-allocate 
                    // every page table under this page directory
                    for i in 0..PAGE_TABLE_MAX_ENTRIES {
                        // don't do anything if this was already a 2MB page, since there is no page table in that case
                        if !is_bit_set_usize(pd.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                        
                            // de-allocate every page under this page table
                            unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(pd.entries[i].align_4k()); }
                            self.unmap_page(VirtAddr(pd.entries[i].align_4k().as_usize()), PageSize::Small);
                        }
                    }

                    // now de-allocate the page table itself
                    unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(pd.entries[pd_idx].align_4k()) }
                    self.unmap_page(VirtAddr(pd.entries[pd_idx].align_4k().as_usize()), PageSize::Small);
                }
            }

            // Map our medium page
            pd.entries[pd_idx] = p;
            pd.entries[pd_idx].inner_or(
                flags |
                PAGING_IS_PAGE_FRAME
            );

            // Now that we've cleaned up the already mapped pages,
            // and we've mapped the 2MB page, we can return
            x86_invalidate_page(v.as_usize());
            return Some(v);
        } // PageSize::Medium

        // This must be a 4KB page

        // create a new pt if one does not exist
        if pd.entries[pd_idx] == PhysAddr(0) {
            let new_pt_base = unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().alloc_page() };
            
            match new_pt_base {
                None => return None,
                Some(np) => {
                    pd.entries[pd_idx] = np;
                    self.identity_map_page(np, PageSize::Small, PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);
                    pd.entries[pd_idx].inner_or(PAGING_PRESENT | PAGING_WRITABLE | PAGING_WRITETHROUGH);
                },
            }
        }
        
        // create a reference to our pt
        pt = addr_to_ref_type_mut::<PageTable, PhysAddr>(pd.entries[pd_idx].align_4k());

        // Map our small page
        // no page frame flag for 4KB pages
        pt.entries[pt_idx] = p;
        pt.entries[pt_idx].inner_or(
            flags
        );
        x86_invalidate_page(v.as_usize());
        Some(v)
    }

    fn unmap_page(&mut self, v: VirtAddr, size: PageSize) {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = v.get_page_table_indexes();

        let pdpt: &mut PageTable;
        let pd: &mut PageTable;
        let pt: &mut PageTable;

        match size {
            PageSize::Small => {
                if !v.is_aligned_4k() { panic!("In x86_64, small pages must be 4KB aligned to unmap"); }
            },
            PageSize::Medium => {
                if !v.is_aligned_2m() { panic!("In x86_64, medium pages must be 2MB aligned to unmap"); }
            },
            PageSize::Huge => {
                if !v.is_aligned_1g() { panic!("In x86_64, huge pages must be 1GB aligned to unmap"); }
            }
        }

        // check our entry in the pml4 table, which maps 512GB chunks
        if self.entries[pml4_idx] == PhysAddr(0) {
            // if the entry is already 0, then there's nothing to do
            return;
        } 

        // check our entry in the pdpt table, which maps 1GB chunks
        // create a reference to our pdpt
        pdpt = addr_to_ref_type_mut::<PageTable, PhysAddr>(self.entries[pml4_idx].align_4k());

        // see if we're unmapping a 1GB page. if so, mark it as zero and clean up if necessary
        if size == PageSize::Huge {
            // Since we're unmapping a huge page, we need to remove any mappings that may have been
            // present as 2MB or 4KB pages underneath, so we don't have a memory leak
        
            // don't do anything if this was already a 1GB page, since there will be no pds or pts to remove
            if !is_bit_set_usize(pdpt.entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {

                // follow the old pdpt entry to its page directory
                let local_pd = addr_to_ref_type_mut::<PageTable, PhysAddr>(pdpt.entries[pdpt_idx].align_4k());

                // if they're not 2MB page entries, then we need to
                // de-allocate every page table under this page directory
                for i in 0..PAGE_TABLE_MAX_ENTRIES {
                    if !is_bit_set_usize(local_pd.entries[i].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                        unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(local_pd.entries[i].align_4k()); }
                        self.unmap_page(VirtAddr(local_pd.entries[i].align_4k().as_usize()), PageSize::Small);
                    }
                }

                // now de-allocate the page directory itself
                unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(pdpt.entries[pdpt_idx].align_4k()) }
                self.unmap_page(VirtAddr(pdpt.entries[pdpt_idx].align_4k().as_usize()), PageSize::Small);
            }
            
            // Unmap our huge page
            pdpt.entries[pdpt_idx] = PhysAddr(0);
            x86_invalidate_page(v.as_usize());
        } // PageSize::Huge
        
        if pdpt.entries[pdpt_idx] == PhysAddr(0) {
            // if the entry is already 0, then there's nothing to do
            return;
        }

        // create a reference to our pd
        pd = addr_to_ref_type_mut::<PageTable, PhysAddr>(pdpt.entries[pdpt_idx].align_4k());

        // see if we're unmapping a 2MB page. if so, mark it as a 0 and clean up if necessary
        if size == PageSize::Medium {
            // Since we're unmapping a medium page, we need to remove any mappings that may have been
            // present as 4KB pages underneath, so we don't have a memory leak
            {
                // all of the entries in the page directory are either page tables that point
                // to 4KB pages, or they are 2MB page table entries. We need to de-allocate 
                // every page table under this page directory
                for i in 0..PAGE_TABLE_MAX_ENTRIES {
                    // don't do anything if this was already a 2MB page, since there is no page table in that case
                    if !is_bit_set_usize(pd.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                    
                        // de-allocate every page under this page table
                        unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(pd.entries[i].align_4k()); }
                        self.unmap_page(VirtAddr(pd.entries[i].align_4k().as_usize()), PageSize::Small);
                    }
                }

                // now de-allocate the page directory itself
                unsafe { FRAME_ALLOCATOR_3.lock().as_mut().unwrap().dealloc_page(pdpt.entries[pdpt_idx].align_4k()) }
                self.unmap_page(VirtAddr(pdpt.entries[pdpt_idx].align_4k().as_usize()), PageSize::Small);
            }

            // unmap our medium page
            pd.entries[pd_idx] = PhysAddr(0);
            x86_invalidate_page(v.as_usize());          
        } // PageSize::Medium

        if pd.entries[pd_idx] == PhysAddr(0) {
            // if the entry is already 0, then there's nothing to do
            return;
        }

        // This is a 4KB page

        // create a reference to our pt
        pt = addr_to_ref_type_mut::<PageTable, PhysAddr>(pd.entries[pd_idx].align_4k());

        // Unmap our small page
        // no page frame flag for 4KB pages
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