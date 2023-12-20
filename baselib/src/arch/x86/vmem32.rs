#![allow(dead_code)]

use uefi::table::boot::MemoryType;

use crate::common::base::*;

use crate::arch::x86::asm::{x86_invalidate_page, x86_write_cr3};

use core::ptr;

// CONSTANTS

// STATICS

// THE REST
pub struct Vas {
    pub cr3: PhysAddr,
    pub base_page_table: *mut BasePageTable,
    pub owner: Owner,
}
impl AddrSpace for Vas {
    fn new() -> Self {
        Vas {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            cr3: PhysAddr(0),
            base_page_table: ptr::null_mut(),
            owner: Owner::System,
        }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn switch_to(&mut self) {
        self.init_cr3();

        if self.cr3 == PhysAddr(0) {
            panic!("Tried to switch to a VAS with a null cr3");
        }

        x86_write_cr3(self.cr3.as_usize());
    }

    fn identity_map_based_on_memory_map(&mut self) {
        for e in iron().uefi_mem_map_0_1.as_ref().unwrap().entries() {
            if e.ty != MemoryType::CONVENTIONAL {
                for i in 0..e.page_count as usize {
                    let page_start = e.phys_start as usize + (i * MEMORY_DEFAULT_PAGE_USIZE);

                    unsafe {
                        self.base_page_table.as_mut().unwrap().identity_map_page(
                            PhysAddr(page_start),
                            Owner::Memory,
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
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn init_cr3(&mut self) -> PhysAddr {
        let mut p = raw::ref_to_raw::<BasePageTable, PhysAddr>(unsafe {
            self.base_page_table.as_mut().unwrap()
        });
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
    pub entries: [Pte; PAGE_TABLE_MAX_ENTRIES],
}

pub type BasePageTable = PageTable;

// TODO - ownership of memory regions in x86 32-bit is not being set properly yet.

impl PageDir for BasePageTable {
    fn new_base() -> PhysAddr {
        let new_pd_base = 
            iron().frame_alloc_internal_0_2
                .lock()
                .as_mut()
                .unwrap()
                .alloc_page(Owner::System, PageSize::Small);

        match new_pd_base {
            None => panic!("Out of memory when allocating for a new pd for a new vas"),
            Some(np) => {
                serial_println!("new pd @ {:?}", np);
                let pd = raw::raw_to_static_ref_mut::<BasePageTable, PhysAddr>(np);
                pd.identity_map_page(
                    np,
                    Owner::Memory,
                    PageSize::Small,
                    PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                );
                new_pd_base.unwrap()
            }
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
        if ubit::is_bit_set(self.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            return self.entries[pd_idx].align_4m();
        } // PageSize::Medium

        // create a reference to our pt
        pt = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(self.entries[pd_idx].align_4k());

        // return our small page
        return pt.entries[pt_idx].align_4k();
    }

    fn map_page(
        &mut self,
        p: PhysAddr,
        v: VirtAddr,
        _owner: Owner,
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
            if self.entries[pd_idx] != PhysAddr(0) {
                // all of the entries in this page directory either point to page tables that themselves point
                // to 4KB pages, or they point to 4MB page frame entries. We need to de-allocate the page table
                // entry if it's not a page frame entry
                if !ubit::is_bit_set(self.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                    // de-allocate the page table
                    iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_page(
                        self.entries[pd_idx].align_4k(),
                        Owner::Nobody,
                        page_size,
                    );
                    let v = VirtAddr(self.entries[pd_idx].align_4k().as_usize());
                    self.unmap_page(v, Owner::Nobody, page_size);
                    x86_invalidate_page(v.as_usize());
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
        if self.entries[pd_idx] == PhysAddr(0) {
            let new_pt_base = 
                iron().frame_alloc_internal_0_2
                    .lock()
                    .as_mut()
                    .unwrap()
                    .alloc_page(Owner::System, PageSize::Small);

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
        pt = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(self.entries[pd_idx].align_4k());

        // This must be a 4KB page

        // we already know we have a good pd & pt, so just map the page
        // no page frame flag for 4KB pages
        pt.entries[pt_idx] = p;
        pt.entries[pt_idx].inner_or(flags);
        x86_invalidate_page(v.as_usize());
        Some(v)
    }

    fn dealloc_page(&mut self, v: VirtAddr, owner: Owner, page_size: PageSize) {
        self.unmap_page(v, owner, page_size);
        iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_page(
            self.virt_to_phys(v),
            Owner::Nobody,
            page_size,
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
            iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_page(
                self.virt_to_phys(cv),
                Owner::Nobody,
                page_size,
            );
            cv.inner_inc_by_page_size(page_size);
        }
    }

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
        if self.entries[pd_idx] == PhysAddr(0) {
            // if the entry is already 0, then there's nothing to do
            return;
        }

        // if this is a 4MB page, all we need to do is set it to 0 and leave
        if ubit::is_bit_set(self.entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            self.entries[pd_idx] = PhysAddr(0);
            x86_invalidate_page(v.as_usize());
            return;
        }

        // this must be a 4KB page

        // create a reference to our pt
        pt = raw::raw_to_static_ref_mut::<PageTable, PhysAddr>(self.entries[pd_idx].align_4k());

        // Unmap our small page
        pt.entries[pt_idx] = PhysAddr(0);
        x86_invalidate_page(v.as_usize());
    }

    fn identity_map_page(&mut self, p: PhysAddr, owner: Owner, page_size: PageSize, flags: usize) {
        self.map_page(
            p,
            VirtAddr(p.align_4k().as_usize()),
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
                .alloc_page(owner, page_size);

        // Looks like we were not able to obtain a page
        if new_page_frame_base.is_none() {
            return VirtAddr(0);
        }

        // Map the new page frame to where it was requested
        self.map_page(new_page_frame_base.unwrap(), v, owner, page_size, flags);

        // fill the allocated memory with the bit pattern
        unsafe {
            ptr::write_bytes(
                raw::raw_to_ptr_mut::<u8, VirtAddr>(v),
                bit_pattern.into_bits(),
                page_size.into_bits(),
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
                    .alloc_page(owner, page_size);

            if page_base.is_some() {
                allocated_pages += 1;

                self.map_page(
                    page_base.unwrap(),
                    VirtAddr(v.as_usize() + (i * page_size.into_bits())),
                    owner,
                    page_size,
                    flags,
                );
            } else {
                // we need to deallocate and unmap any pages that were allocated and return failure
                let mut cv: VirtAddr = v;

                for _j in 0..allocated_pages {
                    self.unmap_page(cv, owner, page_size);
                    iron().frame_alloc_internal_0_2.lock().as_mut().unwrap().dealloc_page(
                        self.virt_to_phys(cv),
                        Owner::Nobody,
                        page_size,
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
                    page_size.into_bits(),
                );
            }
        }
        Some(v)
    }

    fn alloc_pages_contiguous_fixed(
        &mut self,
        size: usize,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
        bit_pattern: BitPattern,
    ) -> Option<VirtAddr> {
        let size_in_pages = pages::calc_pages_reqd(size, PageSize::Small);
        let page_base = 
            iron().frame_alloc_internal_0_2
                .lock()
                .as_mut()
                .unwrap()
                .alloc_page_aligned(size, owner, page_size);
            
        if page_base.is_some() {
            for i in 0..size_in_pages {
                self.map_page(
                    PhysAddr(page_base.unwrap()),
                    VirtAddr(v.as_usize() + (i * page_size.into_bits())),
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
                raw::raw_to_ptr_mut::<u8, VirtAddr>(v),
                bit_pattern.into_bits(),
                size,
            );
        }

        Some(v)
    }
}
