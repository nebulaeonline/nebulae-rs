#![allow(dead_code)]
use core::ptr;

use uefi::table::boot::MemoryType;

use crate::genesis::*;
use crate::common::base::*;
use crate::arch::x86::asm::{x86_invalidate_page, x86_write_cr3};

pub struct Vas {
    pub cr3: PhysAddr,
    pub base_page_table: *mut BasePageTable,
}

impl AddrSpace for Vas {
    fn new() -> Self {
        Vas {
            cr3: 0usize.as_phys(),
            base_page_table: ptr::null_mut(),
        }
    }

    fn switch_to(&mut self) {
        self.init_cr3();

        if self.cr3 == 0usize.as_phys() {
            panic!("Tried to switch to a VAS with a null cr3");
        }

        x86_write_cr3(self.cr3.as_usize());
    }

    fn identity_map_based_on_memory_map(&mut self) {
        #[cfg(debug_assertions)]
        serial_println!("identity_map_based_on_memory_map()");

        for e in iron().uefi_mem_map_0_1.as_ref().unwrap().entries() {
            if e.ty != MemoryType::CONVENTIONAL {
                for i in 0..e.page_count as usize {
                    let page_start = e.phys_start as usize + (i * MEMORY_DEFAULT_PAGE_USIZE);

                    unsafe {
                        self.base_page_table.as_mut().unwrap().identity_map_page(
                            page_start.as_phys(),
                            Owner::Uefi,
                            PageSize::Small,
                            PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                        );
                    }
                }
            }
        }
    }
}

impl Vas {
    pub fn init_cr3(&mut self) -> PhysAddr {
        let mut p = self.base_page_table.addr().as_phys();
        p.inner_or(PAGING_WRITETHROUGH);
        self.cr3 = p;
        p
    }
}

// Page dir / table entries
// Level 4 - 512G, Level 3 - 1G, Level 2 - 2M, Level 1 - 4K
// x86 just has levels 2 (@4MB pages) & 1 (still 4KB pages)
pub type Pte = PhysAddr;

#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

impl AsUsize for PageSize {
    fn as_usize(&self) -> usize {
        self.into_bits()
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
        #[cfg(debug_assertions)]
        serial_println!("calling new_base() for a base page table, preparing to allocate a new pml4");

        let iron = iron();

        #[cfg(debug_assertions)]
        serial_println!("iron @ {:?}", iron as *const Nebulae as usize);

        let new_pml4_base = 
            iron.frame_alloc_internal_0_2
                .lock()
                .as_mut()
                .unwrap()
                .alloc_frame_single(Owner::Memory, PageSize::Small);

        #[cfg(debug_assertions)]
        serial_println!("new_pml4_base @ {:?}", new_pml4_base);

        match new_pml4_base {
            None => panic!("Out of memory when allocating for a new pml4 for a new vas"),
            Some(np) => {
                serial_println!("new pml4 @ {:?}", np);
                let pml4 = raw::raw_to_static_ref_mut::<BasePageTable, PhysAddr>(np);
                pml4.identity_map_page(
                    np,
                    Owner::Memory,
                    PageSize::Small,
                    PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                );
                new_pml4_base.unwrap()
            }
        }
    }

    fn virt_to_phys(&self, v: VirtAddr) -> PhysAddr {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = v.get_page_table_indexes();

        let pdpt: &mut PageTable;
        let pd: &mut PageTable;
        let pt: &mut PageTable;

        // check our entry in the pml4 table, which maps 512GB chunks
        if self.entries[pml4_idx] == 0usize.as_phys() {
            // if the entry is 0, then there's nothing to do
            return 0usize.as_phys();
        }

        // check our entry in the pdpt table, which maps 1GB chunks
        // create a reference to our pdpt
        pdpt = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(self.entries[pml4_idx].align_4k());

        // see if we're getting a 1GB page. if so, return it
        if ubit::is_bit_set(pdpt.entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            return pdpt.entries[pdpt_idx].align_1g();
        } // PageSize::Huge

        if pdpt.entries[pdpt_idx] == 0usize.as_phys() {
            // if the entry is 0, then there's nothing to do
            return 0usize.as_phys();
        }

        // create a reference to our pd
        pd = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(pdpt.entries[pdpt_idx].align_4k());

        // see if we're getting a 2MB page. if so, return it
        if ubit::is_bit_set(pd.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            return pd.entries[pd_idx].align_2m();
        } // PageSize::Medium

        if pd.entries[pd_idx] == 0usize.as_phys() {
            // if the entry is 0, then there's nothing to do
            return 0usize.as_phys();
        }

        // This is a 4KB page

        // create a reference to our pt
        pt = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(pd.entries[pd_idx].align_4k());

        // return our small page
        return pt.entries[pt_idx].align_4k();
    }

    fn map_page(
        &mut self,
        p: PhysAddr,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
    ) -> Option<VirtAddr> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = v.get_page_table_indexes();

        let pdpt: &mut PageTable;
        let pdpt_is_new: bool;
        let pd: &mut PageTable;
        let pd_is_new: bool;
        let pt: &mut PageTable;

        match page_size {
            PageSize::Small => {
                if !p.is_aligned_4k() {
                    return None;
                }
            }
            PageSize::Medium => {
                if !p.is_aligned_2m() {
                    return None;
                }
            }
            PageSize::Huge => {
                if !p.is_aligned_1g() {
                    return None;
                }
            }
        }

        // check our entry in the pml4 table, which maps 512GB chunks
        // create a new pdpt if one does not exist
        if self.entries[pml4_idx] == 0usize.as_phys() {
            let new_pdpt_base = 
                iron().frame_alloc_internal_0_2
                    .lock()
                    .as_mut()
                    .unwrap()
                    .alloc_frame_single(Owner::System, PageSize::Small);

            match new_pdpt_base {
                None => return None,
                Some(np) => {
                    self.entries[pml4_idx] = np;
                    self.identity_map_page(
                        np,
                        Owner::Memory,
                        PageSize::Small,
                        PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                    );
                    self.entries[pml4_idx]
                        .inner_or(PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH);
                    pdpt_is_new = true;
                }
            }
        } else {
            pdpt_is_new = false;
        }

        // check our entry in the pdpt table, which maps 1GB chunks
        // create a reference to our pdpt
        pdpt = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(self.entries[pml4_idx].align_4k());

        // see if we're doing a 1GB page. if so, mark it as a page and clean up if necessary
        if page_size == PageSize::Huge {
            // Since we're mapping a huge page, we need to remove any mappings that may have been
            // present as 2MB or 4KB pages underneath, so we don't have a memory leak
            if !pdpt_is_new {
                // if we just created this pdpt, then there's nothing to remove

                // don't do anything if this was already a 1GB page, since there will be no pds or pts to remove
                if !ubit::is_bit_set(pdpt.entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                    // follow the old pdpt entry to its page directory
                    let local_pd = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(
                        pdpt.entries[pdpt_idx].align_4k(),
                    );

                    // if they're not 2MB page entries, then we need to
                    // de-allocate every page table under this page directory
                    for i in 0..PAGE_TABLE_MAX_ENTRIES {
                        if !ubit::is_bit_set(
                            local_pd.entries[i].as_usize(),
                            PAGING_IS_PAGE_FRAME_BIT,
                        ) {
                            iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                                local_pd.entries[i].align_4k(),
                                Owner::System,
                            );

                            self.unmap_page(
                                local_pd.entries[i].align_4k().as_usize().as_virt(),
                                Owner::Memory,
                                PageSize::Small,
                            );
                        }
                    }

                    // now de-allocate the page directory itself
                    iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                        pdpt.entries[pdpt_idx].align_4k(),
                        Owner::System);

                    self.unmap_page(
                        pdpt.entries[pdpt_idx].align_4k().as_usize().as_virt(),
                        Owner::Memory,
                        PageSize::Small,
                    );
                }
            }

            // Map our huge page
            pdpt.entries[pdpt_idx] = p;
            pdpt.entries[pdpt_idx].inner_or(flags | PAGING_IS_PAGE_FRAME);

            // Now that we've cleaned up the already mapped pages,
            // and we've mapped the 1GB page, we can return
            x86_invalidate_page(v.as_usize());
            // mark the owner for this page in the page info struct
            let pageinfo = unsafe { iron().page_info.unwrap().as_mut().unwrap() };
            let pageidx = pages::usize_to_page_index(p.as_usize());
            pageinfo[pageidx].owner = owner;

            return Some(v);
        } // PageSize::Huge

        // create a new pd if one does not exist
        if pdpt.entries[pdpt_idx] == 0usize.as_phys() {
            let new_pd_base = 
                iron().frame_alloc_internal_0_2
                    .lock()
                    .as_mut()
                    .unwrap()
                    .alloc_frame_single(Owner::System, PageSize::Small);

            match new_pd_base {
                None => return None,
                Some(np) => {
                    pdpt.entries[pdpt_idx] = np;
                    self.identity_map_page(
                        np,
                        Owner::Memory,
                        PageSize::Small,
                        PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                    );
                    pdpt.entries[pdpt_idx]
                        .inner_or(PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH);
                    pd_is_new = true;
                }
            }
        } else {
            pd_is_new = false;
        }

        // create a reference to our pd
        pd = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(pdpt.entries[pdpt_idx].align_4k());

        // see if we're doing a 2MB page. if so, mark it as a page and clean up if necessary
        if page_size == PageSize::Medium {
            // Since we're mapping a medium page, we need to remove any mappings that may have been
            // present as 4KB pages underneath, so we don't have a memory leak
            if !pd_is_new {
                // if we just created this pd, then there's nothing to remove

                {
                    // all of the entries in the page directory are either page tables that point
                    // to 4KB pages, or they are 2MB page table entries. We need to de-allocate
                    // every page table under this page directory
                    for i in 0..PAGE_TABLE_MAX_ENTRIES {
                        // don't do anything if this was already a 2MB page, since there is no page table in that case
                        if !ubit::is_bit_set(
                            pd.entries[pd_idx].as_usize(),
                            PAGING_IS_PAGE_FRAME_BIT,
                        ) {
                            // de-allocate every page under this page table
                            iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                                pd.entries[i].align_4k(),
                                Owner::System,
                            );

                            self.unmap_page(
                                pd.entries[i].align_4k().as_usize().as_virt(),
                                Owner::Memory,
                                PageSize::Small,
                            );
                        }
                    }

                    // now de-allocate the page table itself
                    iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                        pd.entries[pd_idx].align_4k(),
                        Owner::System,
                    );

                    self.unmap_page(
                        pd.entries[pd_idx].align_4k().as_usize().as_virt(),
                        Owner::Memory,
                        PageSize::Small,
                    );
                }
            }

            // Map our medium page
            pd.entries[pd_idx] = p;
            pd.entries[pd_idx].inner_or(flags | PAGING_IS_PAGE_FRAME);

            // Now that we've cleaned up the already mapped pages,
            // and we've mapped the 2MB page, we can return
            x86_invalidate_page(v.as_usize());
            // mark the owner for this page in the page info struct
            let pageinfo = unsafe { iron().page_info.unwrap().as_mut().unwrap() };
            let pageidx = pages::usize_to_page_index(p.as_usize());
            pageinfo[pageidx].owner = owner;

            return Some(v);
        } // PageSize::Medium

        // This must be a 4KB page

        // create a new pt if one does not exist
        if pd.entries[pd_idx] == 0usize.as_phys() {
            let new_pt_base = 
                iron().frame_alloc_internal_0_2
                    .lock()
                    .as_mut()
                    .unwrap()
                    .alloc_frame_single(Owner::System, PageSize::Small);

            match new_pt_base {
                None => return None,
                Some(np) => {
                    pd.entries[pd_idx] = np;
                    self.identity_map_page(
                        np,
                        Owner::Memory,
                        PageSize::Small,
                        PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                    );
                    pd.entries[pd_idx]
                        .inner_or(PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH);
                }
            }
        }

        // create a reference to our pt
        pt = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(pd.entries[pd_idx].align_4k());

        // Map our small page
        // no page frame flag for 4KB pages
        pt.entries[pt_idx] = p;
        pt.entries[pt_idx].inner_or(flags);
        x86_invalidate_page(v.as_usize());
        // mark the owner for this page in the page info struct
        let pageinfo = unsafe { iron().page_info.unwrap().as_mut().unwrap() };
        let pageidx = pages::usize_to_page_index(p.as_usize());
        pageinfo[pageidx].owner = owner;

        Some(v)
    }

    fn dealloc_page(&mut self, v: VirtAddr, owner: Owner, page_size: PageSize) {
        self.unmap_page(v, owner, page_size);
        iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
            self.virt_to_phys(v),
            Owner::System,
        );
    }

    fn dealloc_pages_contiguous(
        &mut self,
        v: VirtAddr,
        size: usize,
        owner: Owner,
        page_size: PageSize,
    ) {
        let page_count = pages::calc_pages_reqd(size, page_size);
        let mut cv: VirtAddr = v;

        for _i in 0..page_count {
            self.unmap_page(cv, owner, page_size);
            iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                self.virt_to_phys(cv),
                Owner::System,
            );
            cv.inner_inc_by_page_size(page_size);
        }
    }

    fn unmap_page(&mut self, v: VirtAddr, owner: Owner, page_size: PageSize) {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = v.get_page_table_indexes();

        let pdpt: &mut PageTable;
        let pd: &mut PageTable;
        let pt: &mut PageTable;

        match page_size {
            PageSize::Small => {
                if !v.is_aligned_4k() {
                    panic!("In x86_64, small pages must be 4KB aligned to unmap");
                }
            }
            PageSize::Medium => {
                if !v.is_aligned_2m() {
                    panic!("In x86_64, medium pages must be 2MB aligned to unmap");
                }
            }
            PageSize::Huge => {
                if !v.is_aligned_1g() {
                    panic!("In x86_64, huge pages must be 1GB aligned to unmap");
                }
            }
        }

        let fn_unmap_page_info = |p: PhysAddr, page_size: PageSize| {
            let pageinfo = unsafe { iron().page_info.unwrap().as_mut().unwrap() };
            let pageidx = pages::usize_to_page_index(p.as_usize());

            if page_size > PageSize::Small {
                // if this is a huge or medium page, then we need to clear the owner and status
                // for every page underneath
                let page_count = pages::calc_pages_reqd(page_size.as_usize(), PageSize::Small);
                for i in 0..page_count {
                    pageinfo[pageidx + i].owner = Owner::Nobody;
                    pageinfo[pageidx + i].status = pages::PageStatus::Free;
                }
            } else {
                pageinfo[pageidx].owner = Owner::Nobody;
                pageinfo[pageidx].status = pages::PageStatus::Free;
            }
        };

        // check our entry in the pml4 table, which maps 512GB chunks
        if self.entries[pml4_idx] == 0usize.as_phys() {
            // if the entry is already 0, then there's nothing to do
            return;
        }

        // check our entry in the pdpt table, which maps 1GB chunks
        // create a reference to our pdpt
        pdpt = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(self.entries[pml4_idx].align_4k());

        // see if we're unmapping a 1GB page. if so, mark it as zero and clean up if necessary
        if page_size == PageSize::Huge {
            // Since we're unmapping a huge page, we need to remove any mappings that may have been
            // present as 2MB or 4KB pages underneath, so we don't have a memory leak

            // don't do anything if this was already a 1GB page, since there will be no pds or pts to remove
            if !ubit::is_bit_set(pdpt.entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                // follow the old pdpt entry to its page directory
                let local_pd = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(
                    pdpt.entries[pdpt_idx].align_4k(),
                );

                // if they're not 2MB page entries, then we need to
                // de-allocate every page table under this page directory
                for i in 0..PAGE_TABLE_MAX_ENTRIES {
                    if !ubit::is_bit_set(local_pd.entries[i].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                        iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                            local_pd.entries[i].align_4k(),
                            Owner::Memory,
                        );

                        self.unmap_page(
                            local_pd.entries[i].align_4k().as_usize().as_virt(),
                            Owner::Memory,
                            PageSize::Small,
                        );
                    }
                }

                // now de-allocate the page directory itself
                iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                    pdpt.entries[pdpt_idx].align_4k(),
                    Owner::System,
                );

                self.unmap_page(
                    pdpt.entries[pdpt_idx].align_4k().as_usize().as_virt(),
                    owner,
                    PageSize::Small,
                );
            }

            // Clear our owner information
            fn_unmap_page_info(pdpt.entries[pdpt_idx].align_1g(), page_size);

            // Unmap our huge page
            pdpt.entries[pdpt_idx] = 0usize.as_phys();
            x86_invalidate_page(v.as_usize());
        } // PageSize::Huge

        if pdpt.entries[pdpt_idx] == 0usize.as_phys() {
            // if the entry is already 0, then there's nothing to do
            return;
        }

        // create a reference to our pd
        pd = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(pdpt.entries[pdpt_idx].align_4k());

        // see if we're unmapping a 2MB page. if so, mark it as a 0 and clean up if necessary
        if page_size == PageSize::Medium {
            // Since we're unmapping a medium page, we need to remove any mappings that may have been
            // present as 4KB pages underneath, so we don't have a memory leak
            {
                // all of the entries in the page directory are either page tables that point
                // to 4KB pages, or they are 2MB page table entries. We need to de-allocate
                // every page table under this page directory
                for i in 0..PAGE_TABLE_MAX_ENTRIES {
                    // don't do anything if this was already a 2MB page, since there is no page table in that case
                    if !ubit::is_bit_set(pd.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                        // de-allocate every page under this page table
                        iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                            pd.entries[i].align_4k(),
                            Owner::Memory,
                        );

                        self.unmap_page(
                            pd.entries[i].align_4k().as_usize().as_virt(),
                            Owner::Memory,
                            PageSize::Small,
                        );
                    }
                }

                // now de-allocate the page directory itself
                iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                    pdpt.entries[pdpt_idx].align_4k(),
                    Owner::Memory,
                );

                self.unmap_page(
                    pdpt.entries[pdpt_idx].align_4k().as_usize().as_virt(),
                    Owner::Memory,
                    PageSize::Small,
                );
            }

            // Clear our owner information
            fn_unmap_page_info(pd.entries[pd_idx].align_2m(), page_size);

            // unmap our medium page
            pd.entries[pd_idx] = 0usize.as_phys();
            x86_invalidate_page(v.as_usize());
        } // PageSize::Medium

        if pd.entries[pd_idx] == 0usize.as_phys() {
            // if the entry is already 0, then there's nothing to do
            return;
        }

        // This is a 4KB page

        // create a reference to our pt
        pt = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(pd.entries[pd_idx].align_4k());

        // Clear our owner information
        fn_unmap_page_info(pt.entries[pt_idx].align_4k(), page_size);

        // Unmap our small page
        // no page frame flag for 4KB pages
        pt.entries[pt_idx] = 0usize.as_phys();
        x86_invalidate_page(v.as_usize());
    }

    fn identity_map_page(&mut self, p: PhysAddr, owner: Owner, page_size: PageSize, flags: usize) {
        self.map_page(
            p,
            p.align_4k().as_usize().as_virt(),
            owner,
            page_size,
            flags,
        );
    }

    fn alloc_page_fixed(
        &mut self,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
        bit_pattern: BitPattern,
    ) -> VirtAddr {
        let new_page_frame_base = 
            iron().frame_alloc_internal_0_2
                .lock()
                .as_mut()
                .unwrap()
                .alloc_frame_single(Owner::System, page_size);

        // Looks like we were not able to obtain a page
        if new_page_frame_base.is_none() {
            return 0usize.as_virt();
        }

        // Map the new page frame to where it was requested
        self.map_page(new_page_frame_base.unwrap(), v, owner, page_size, flags);

        // fill the allocated memory with the bit pattern
        unsafe {
            ptr::write_bytes(
                raw::raw_to_ptr_mut::<u8, VirtAddr>(v),
                bit_pattern.into_bits(),
                page_size.as_usize(),
            );
        }

        // Return the virtual address of the new page frame
        v
    }

    fn alloc_pages_fixed(
        &mut self,
        size: usize,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
        bit_pattern: BitPattern,
    ) -> Option<VirtAddr> {
        let size_in_pages = pages::calc_pages_reqd(size, page_size);
        let mut allocated_pages: usize = 0;

        for i in 0..size_in_pages {
            let page_base = 
                iron().frame_alloc_internal_0_2
                    .lock()
                    .as_mut()
                    .unwrap()
                    .alloc_frame_single(Owner::System, page_size);

            if page_base.is_some() {
                allocated_pages += 1;

                self.map_page(
                    page_base.unwrap(),
                    VirtAddr(v.as_usize() + (i * page_size.as_usize())),
                    owner,
                    page_size,
                    flags,
                );
            } else {
                // we need to deallocate and unmap any pages that failed and return failure
                let mut cv: VirtAddr = v;

                for _j in 0..allocated_pages {
                    self.unmap_page(cv, owner, page_size);
                    iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                        self.virt_to_phys(cv),
                        Owner::System,
                    );

                    cv.inner_inc_by_page_size(PageSize::Small);
                }

                return None;
            }
        }

        // fill the allocated memory with the bit pattern
        for _j in 0..allocated_pages {
            unsafe {
                ptr::write_bytes(
                    raw::raw_to_ptr_mut::<u8, VirtAddr>(v),
                    bit_pattern.into_bits(),
                    page_size.as_usize(),
                );
            }
        }
        Some(v)
    }

    fn alloc_pages_fixed_virtual(
        &mut self,
        size: usize,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
        bit_pattern: BitPattern,
    ) -> Option<VirtAddr> {
        let size_in_pages = pages::calc_pages_reqd(size, page_size);
        let mut vb = v.clone();

        let page_base = 
            iron().frame_alloc_internal_0_2
                .lock()
                .as_mut()
                .unwrap()
                .alloc_frame(size, page_size, Owner::System);
            
        if page_base.is_some() {
            for _i in 0..size_in_pages {
                self.map_page(page_base.unwrap(), vb, owner, page_size, flags);
                vb.inner_inc_by_page_size(page_size);
            }
        } else {
            return None;
        }

        // fill the allocated memory with the bit pattern
        unsafe {
            ptr::write_bytes(
                raw::raw_to_ptr_mut::<u8, VirtAddr>(v),
                bit_pattern.into_bits(),
                size,
            );
        }

        Some(v)
    }
}
