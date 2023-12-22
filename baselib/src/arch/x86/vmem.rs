#![allow(dead_code)]

use uefi::table::boot::MemoryType;

use crate::genesis::*;
use crate::common::base::*;
use crate::arch::x86::asm::{x86_invalidate_page, x86_write_cr3};

use core::ptr;

pub mod pages {
    use super::*;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub enum PageStatus {
        Free,
        Reserved,
        Alloc,
    }

    #[repr(C)]
    pub struct PageInfo {
        pub phys_base: PhysAddr,
        pub size: usize,
        pub uefi_flags: usize,
        pub status: PageStatus,
        pub owner: Owner,
        pub purpose: MemoryType,
        pub flags: usize,
        pub page_size: PageSize,
    }

    // calculate the amount of memory given the number of default sized pages
    #[inline(always)]
    pub const fn pages_to_bytes(page_count: usize, page_size: PageSize) -> usize {
        page_count * page_size.into_bits()
    }

    // calculates the number of pages given the number of bytes and the page size
    // returns true if the number of bytes is evenly divisible by the specified page size
    // returns false otherwise
    #[inline(always)]
    pub const fn bytes_to_pages(bytes: usize, page_size: PageSize) -> (usize, bool) {
        let page_count = (bytes + page_size.into_bits() - 1) / page_size.into_bits();
        let remainder = bytes % MEMORY_DEFAULT_PAGE_USIZE;

        (page_count, remainder == 0)
    }

    pub const fn calc_pages_reqd(size: usize, page_size: PageSize) -> usize {
        (size + page_size.into_bits() - 1) / page_size.into_bits()
    }

    // calculates the page index (in MEMORY_DEFAULT_PAGE_SIZE units) given a byte address
    #[inline(always)]
    pub const fn usize_to_page_index(raw: usize) -> usize {
        raw >> MEMORY_DEFAULT_SHIFT
    }

    // calculates the page index (in MEMORY_DEFAULT_PAGE_SIZE units) given a byte address
    #[inline(always)]
    pub fn addr_to_page_index(addr: impl MemAddr + AsUsize + Sized) -> usize {
        addr.as_usize() >> MEMORY_DEFAULT_SHIFT
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub struct Vas<'a> {
    pub cr3: PhysAddr,
    pub base_page_table: Option<&'a mut BasePageTable>,
    pub owner: Owner,
}
impl<'a> AddrSpace for Vas<'a> {
    fn new() -> Self {
        Vas {
            cr3: 0usize.as_phys(),
            base_page_table: None,
            owner: Owner::Memory,
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn switch_to(&mut self) {
        self.init_cr3();

        if self.cr3 == 0usize.as_phys() {
            panic!("VAS::switch_to() -> Tried to switch to an address space with a null cr3");
        }

        x86_write_cr3(self.cr3.as_usize());
    }

    fn identity_map_critical(&mut self) {
        #[cfg(debug_assertions)]
        serial_println!("VAS::identity_map_critical() -> entering fn");
        
        // FIXME
    }
}

impl<'a> Vas<'a> {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn init_cr3(&mut self) -> PhysAddr {
        
        // get the address of the base page table
        let mut p = raw::ref_to_raw::<BasePageTable, PhysAddr>(self.base_page_table.as_mut().unwrap());
        p.inner_or(PAGING_WRITETHROUGH);
        self.cr3 = p;
        p
    }
}

// Page dir / table entries
// Level 4 - 512G, Level 3 - 1G, Level 2 - 2M, Level 1 - 4K
// x86 just has levels 2 (@4MB pages) & 1 (still 4KB pages)
pub type Pte = PhysAddr;

#[cfg(target_arch = "x86")]
#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PageSize {
    Small = USIZE_4K,
    Medium = USIZE_4M,
}

#[cfg(target_arch = "x86")]
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

#[cfg(target_arch = "x86_64")]
#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PageSize {
    Small = USIZE_4K,
    Medium = USIZE_2M,
    Huge = USIZE_1G,
}

#[cfg(target_arch = "x86_64")]
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
        serial_println!("BasePageTable::new_base() -> preparing to allocate a new base paging struct");

        let iron = iron();

        #[cfg(debug_assertions)]
        serial_println!("BasePageTable::new_base() -> iron @ 0x{:08x}", iron as *const Nebulae as usize);

        let new_pd_base = 
            iron.frame_alloc_internal_0_2
                .lock()
                .as_mut()
                .unwrap()
                .alloc_frame(MEMORY_DEFAULT_PAGE_USIZE, PageSize::Small, Owner::Memory);

        
        
        match new_pd_base {
            None => panic!("BasePageTable::new_base() -> out of memory when allocating for a new base paging struct"),
            Some(np) => {
                #[cfg(debug_assertions)]
                serial_println!("BasePageTable::new_base() -> new base paging struct @ 0x{:0x}", new_pd_base.unwrap());

                #[cfg(debug_assertions)]
                serial_println!("BasePageTable::new_base() -> identity mapping base paging struct @ 0x{:0x}", new_pd_base.unwrap());

                let pd = raw::abracadabra_static_ref_mut::<BasePageTable>(np);
                pd.identity_map_page(
                    np,
                    Owner::Memory,
                    PageSize::Small,
                    PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                );

                #[cfg(debug_assertions)]
                serial_println!("BasePageTable::new_base() -> identity mapping complete");

                new_pd_base.unwrap()
            }
        }
    }

    #[cfg(target_arch = "x86")]
    fn map_page(
        &mut self,
        p: PhysAddr,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
    ) -> Option<VirtAddr> {
        let (pd_idx, pt_idx) = v.get_page_table_indexes();

        let pt: &mut PageTable;

        match page_size {
            PageSize::Small => {
                if !v.is_aligned_4k() {
                    return None;
                }
            }
            PageSize::Medium => {
                if !v.is_aligned_4m() {
                    return None;
                }
            }
        }

        // see if we're doing a 4MB page. if so, mark it as a page and clean up if necessary
        if page_size == PageSize::Medium {
            // Since we're mapping a medium page, we need to remove any mappings that may have been
            // present as 4KB pages underneath, so we don't have a memory leak
            if self.entries[pd_idx] != 0usize.as_phys() {
                // all of the entries in this page directory either point to page tables that themselves point
                // to 4KB pages, or they point to 4MB page frame entries. We need to de-allocate the page table
                // entry if it's not a page frame entry
                if !ubit::is_bit_set(self.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                    // de-allocate the page table
                    iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                        self.entries[pd_idx].align_4k(),
                        Owner::Nobody,
                    );
                    let v = self.entries[pd_idx].align_4k().as_usize();
                    self.unmap_page(v.as_virt(), Owner::Nobody, page_size);
                    x86_invalidate_page(v);
                }
            }

            // Map our medium page
            self.entries[pd_idx] = p;
            self.entries[pd_idx].inner_or(flags | PAGING_IS_PAGE_FRAME);

            // Now that we've cleaned up the already mapped pages,
            // and we've mapped the 4MB page, we can return
            x86_invalidate_page(v.as_usize());
            return Some(v);
        } // PageSize::Medium

        // check our entry in the pd table, which maps 4MB chunks
        // create a new pd if one does not exist
        if self.entries[pd_idx] == 0usize.as_phys() {
            let new_pt_base = 
                iron().frame_alloc_internal_0_2
                    .lock()
                    .as_mut()
                    .unwrap()
                    .alloc_frame_single(Owner::System, PageSize::Small);

            match new_pt_base {
                None => return None,
                Some(np) => {
                    self.entries[pd_idx] = np;
                    self.identity_map_page(
                        np,
                        Owner::Memory,
                        PageSize::Small,
                        PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                    );
                    self.entries[pd_idx]
                        .inner_or(PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH);
                }
            }
        }

        // create a reference to our pt
        pt = raw::abracadabra_static_ref_mut::<PageTable, PhysAddr>(self.entries[pd_idx].align_4k());

        // This must be a 4KB page

        // we already know we have a good pd & pt, so just map the page
        // no page frame flag for 4KB pages
        pt.entries[pt_idx] = p;
        pt.entries[pt_idx].inner_or(flags);
        x86_invalidate_page(v.as_usize());
        Some(v)
    }

    #[cfg(target_arch = "x86_64")]
    fn map_page(
        &mut self,
        p: PhysAddr,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
    ) -> Option<VirtAddr> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = v.get_page_table_indexes();

        #[cfg(debug_assertions)]
        serial_println!("BasePageTable::map_page() -> mapping page @ 0x{:0x} to 0x{:0x} with size {} and flags 0x{:0x}", p, v, page_size.as_usize(), flags);

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

        #[cfg(debug_assertions)]
        serial_println!("BasePageTable::map_page() -> page size is valid");

        #[cfg(debug_assertions)]
        serial_println!("BasePageTable::map_page() -> self.entries == 0x{:0x}", self.entries.as_ptr() as usize);

        #[cfg(debug_assertions)]
        serial_println!("BasePageTable::map_page() -> pml4_idx == {}", pml4_idx);

        // check our entry in the pml4 table, which maps 512GB chunks
        // create a new pdpt if one does not exist
        if self.entries[pml4_idx] == 0usize.as_phys() {
            #[cfg(debug_assertions)]
            serial_println!("BasePageTable::map_page() -> allocating frame for new pdpt");
            
            let new_pdpt_base = 
                iron().frame_alloc_internal_0_2
                    .lock()
                    .as_mut()
                    .unwrap()
                    .alloc_frame(MEMORY_DEFAULT_PAGE_USIZE, PageSize::Small, Owner::System);

            match new_pdpt_base {
                None => return None,
                Some(np) => {
                    #[cfg(debug_assertions)]
                    serial_println!("BasePageTable::map_page() -> new frame obtained for new pdpt @ 0x{:0x}", np);

                    self.entries[pml4_idx] = np;
                    
                    #[cfg(debug_assertions)]
                    serial_println!("BasePageTable::map_page() -> identity mapping new pdpt @ 0x{:0x}", np);
                   
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

            #[cfg(debug_assertions)]
            serial_println!("BasePageTable::map_page() -> new pdpt frame created & identity mapped");
        } else {
            #[cfg(debug_assertions)]
            serial_println!("BasePageTable::map_page() -> pdpt located: 0x{:0x}", self.entries[pml4_idx]);

            pdpt_is_new = false;
        }

        // check our entry in the pdpt table, which maps 1GB chunks
        // create a reference to our pdpt
        pdpt = raw::abracadabra_static_ref_mut::<PageTable>(self.entries[pml4_idx].align_4k());

        // see if we're doing a 1GB page. if so, mark it as a page and clean up if necessary
        if page_size.as_usize() == PageSize::Huge.as_usize() {
            // Since we're mapping a huge page, we need to remove any mappings that may have been
            // present as 2MB or 4KB pages underneath, so we don't have a memory leak
            if !pdpt_is_new {
                // if we just created this pdpt, then there's nothing to remove

                // don't do anything if this was already a 1GB page, since there will be no pds or pts to remove
                if !ubit::is_bit_set(pdpt.entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                    // follow the old pdpt entry to its page directory
                    let local_pd = raw::abracadabra_static_ref_mut::<PageTable>(
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
                    .alloc_frame(MEMORY_DEFAULT_PAGE_USIZE, PageSize::Small, Owner::System);

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
        pd = raw::abracadabra_static_ref_mut::<PageTable>(pdpt.entries[pdpt_idx].align_4k());

        // see if we're doing a 2MB page. if so, mark it as a page and clean up if necessary
        if page_size.as_usize() == PageSize::Medium.as_usize() {
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
                    .alloc_frame(MEMORY_DEFAULT_PAGE_USIZE, PageSize::Small, Owner::System);

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
        pt = raw::abracadabra_static_ref_mut::<PageTable>(pd.entries[pd_idx].align_4k());

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

    #[cfg(target_arch = "x86")]
    fn unmap_page(&mut self, v: VirtAddr, _owner: Owner, size: PageSize) {
        let (pd_idx, pt_idx) = v.get_page_table_indexes();

        let pt: &mut PageTable;

        match size {
            PageSize::Small => {
                if !v.is_aligned_4k() {
                    panic!("In x86, small pages must be 4KB aligned to unmap");
                }
            }
            PageSize::Medium => {
                if !v.is_aligned_4m() {
                    panic!("In x86, medium pages must be 4MB aligned to unmap");
                }
            }
        }

        // check our entry in the pd table, which maps 4MB chunks

        // if the page directory is already 0, then there's nothing to do
        if self.entries[pd_idx] == 0usize.as_phys() {
            // if the entry is already 0, then there's nothing to do
            return;
        }

        // if this is a 4MB page, all we need to do is set it to 0 and leave
        if ubit::is_bit_set(self.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            self.entries[pd_idx] = 0usize.as_phys();
            x86_invalidate_page(v.as_usize());
            return;
        }

        // this must be a 4KB page

        // create a reference to our pt
        pt = raw::abracadabra_static_ref_mut::<PageTable, PhysAddr>(self.entries[pd_idx].align_4k());

        // Unmap our small page
        pt.entries[pt_idx] = 0usize.as_phys();
        x86_invalidate_page(v.as_usize());
    }

    #[cfg(target_arch = "x86_64")]
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

            if page_size.as_usize() > PageSize::Small.as_usize() {
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
        pdpt = raw::abracadabra_static_ref_mut::<PageTable>(self.entries[pml4_idx].align_4k());

        // see if we're unmapping a 1GB page. if so, mark it as zero and clean up if necessary
        if page_size.as_usize() == PageSize::Huge.as_usize() {
            // Since we're unmapping a huge page, we need to remove any mappings that may have been
            // present as 2MB or 4KB pages underneath, so we don't have a memory leak

            // don't do anything if this was already a 1GB page, since there will be no pds or pts to remove
            if !ubit::is_bit_set(pdpt.entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                // follow the old pdpt entry to its page directory
                let local_pd = raw::abracadabra_static_ref_mut::<PageTable>(
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
        pd = raw::abracadabra_static_ref_mut::<PageTable>(pdpt.entries[pdpt_idx].align_4k());

        // see if we're unmapping a 2MB page. if so, mark it as a 0 and clean up if necessary
        if page_size.as_usize() == PageSize::Medium.as_usize() {
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
        pt = raw::abracadabra_static_ref_mut::<PageTable>(pd.entries[pd_idx].align_4k());

        // Clear our owner information
        fn_unmap_page_info(pt.entries[pt_idx].align_4k(), page_size);

        // Unmap our small page
        // no page frame flag for 4KB pages
        pt.entries[pt_idx] = 0usize.as_phys();
        x86_invalidate_page(v.as_usize());
    }

    #[cfg(target_arch = "x86")]
    fn virt_to_phys(&self, v: VirtAddr) -> PhysAddr {
        let (pd_idx, pt_idx) = v.get_page_table_indexes();

        let pt: &mut PageTable;

        // check our entry in the pd table, which maps 4MB chunks
        if self.entries[pd_idx] == 0usize.as_phys() {
            // if the entry is 0, then there's nothing to do
            return 0usize.as_phys();
        }

        // see if we're getting a 4MB page. if so, return it
        if ubit::is_bit_set(self.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            return self.entries[pd_idx].align_4m();
        } // PageSize::Medium

        // create a reference to our pt
        pt = raw::abracadabra_static_ref_mut::<PageTable, PhysAddr>(self.entries[pd_idx].align_4k());

        // return our small page
        return pt.entries[pt_idx].align_4k();
    }

    #[cfg(target_arch = "x86_64")]
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
        pdpt = raw::abracadabra_static_ref_mut::<PageTable>(self.entries[pml4_idx].align_4k());

        // see if we're getting a 1GB page. if so, return it
        if ubit::is_bit_set(pdpt.entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            return pdpt.entries[pdpt_idx].align_1g();
        } // PageSize::Huge

        if pdpt.entries[pdpt_idx] == 0usize.as_phys() {
            // if the entry is 0, then there's nothing to do
            return 0usize.as_phys();
        }

        // create a reference to our pd
        pd = raw::abracadabra_static_ref_mut::<PageTable>(pdpt.entries[pdpt_idx].align_4k());

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
        pt = raw::abracadabra_static_ref_mut::<PageTable>(pd.entries[pd_idx].align_4k());

        // return our small page
        return pt.entries[pt_idx].align_4k();
    }

    fn dealloc_page(&mut self, v: VirtAddr, owner: Owner, page_size: PageSize) {
        self.unmap_page(v, owner, page_size);
        iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
            self.virt_to_phys(v),
            Owner::Nobody,
        );
    }

    fn dealloc_pages_contiguous(
        &mut self,
        v: VirtAddr,
        size: usize,
        owner: Owner,
        page_size: PageSize,
    ) {
        let page_count = pages::calc_pages_reqd(size, PageSize::Small);
        let mut cv: VirtAddr = v;

        for _i in 0..page_count {
            self.unmap_page(cv, owner, page_size);
            iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                self.virt_to_phys(cv),
                Owner::Nobody,
            );
            cv.inner_inc_by_page_size(page_size);
        }
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
                .alloc_frame(MEMORY_DEFAULT_PAGE_USIZE, page_size, owner);

        // Looks like we were not able to obtain a page
        if new_page_frame_base.is_none() {
            return 0usize.as_virt();
        }

        // Map the new page frame to where it was requested
        self.map_page(new_page_frame_base.unwrap(), v, owner, page_size, flags);

        // fill the allocated memory with the bit pattern
        unsafe {
            ptr::write_bytes(
                raw::abracadabra_ptr_mut::<u8, VirtAddr>(v),
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
        let size_in_pages = pages::calc_pages_reqd(size, PageSize::Small);
        let mut allocated_pages: usize = 0;

        for i in 0..size_in_pages {
            let page_base = 
                iron().frame_alloc_internal_0_2
                    .lock()
                    .as_mut()
                    .unwrap()
                    .alloc_frame(MEMORY_DEFAULT_PAGE_USIZE, page_size, owner);

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
                // we need to deallocate and unmap any pages that were allocated and return failure
                let mut cv: VirtAddr = v;

                for _j in 0..allocated_pages {
                    self.unmap_page(cv, owner, page_size);
                    iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_frame(
                        self.virt_to_phys(cv),
                        Owner::Nobody,
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
                    raw::abracadabra_ptr_mut::<u8, VirtAddr>(v),
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
        let page_base = 
            iron().frame_alloc_internal_0_2
                .lock()
                .as_mut()
                .unwrap()
                .alloc_frame(size, page_size, owner);
            
        if page_base.is_some() {
            for i in 0..size_in_pages {
                self.map_page(
                    page_base.unwrap(),
                    VirtAddr(v.as_usize() + (i * page_size.as_usize())),
                    owner,
                    page_size,
                    flags,
                );
            }
        } else {
            return None;
        }

        // fill the allocated memory with the bit pattern
        unsafe {
            ptr::write_bytes(
                raw::abracadabra_ptr_mut::<u8, VirtAddr>(v),
                bit_pattern.into_bits(),
                size,
            );
        }

        Some(v)
    }
}