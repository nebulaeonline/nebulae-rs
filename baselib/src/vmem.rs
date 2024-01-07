#![allow(dead_code)]
use crate::nebulae::*;
use crate::common::base::*;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use crate::arch::x86::asm::{x86_invalidate_page, x86_write_cr3};

use core::ptr;

#[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
pub const MEMORY_DEFAULT_PAGE_SIZE_ENUM: PageSize = PageSize::Small;

pub mod pages {

    use super::*;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(u8)]
    pub enum PageStatus {
        Free,
        Reserved,
        Alloc,
        Missing,
    }

    #[derive(Debug)]
    #[repr(C)]
    pub struct PageInfoStruct {
        pub status: PageStatus,
        pub held_by_fiber_id: NebulaeId,
        pub flags: usize,
        pub is_child: bool,
    }
    impl PageInfoStruct {
        pub fn new() -> Self {
            PageInfoStruct {
                status: PageStatus::Missing,
                held_by_fiber_id: NEBULAE_ID_NOBODY,
                flags: ZERO_USIZE,
                is_child: false,
            }
        }
    }

    // calculate the amount of memory given the number of page_size sized pages
    #[inline(always)]
    pub const fn pages_to_bytes(page_count: usize, page_size: PageSize) -> usize {
        page_count * page_size.as_const_usize()
    }

    // calculates the number of pages given the number of bytes and the page size
    #[inline(always)]
    pub const fn bytes_to_pages(bytes: usize, page_size: PageSize) -> usize {
        (bytes + page_size.as_const_usize() - 1) / page_size.as_const_usize()
    }

    // calculates the page index (in MEMORY_DEFAULT_PAGE_SIZE units) given an address
    #[inline(always)]
    pub fn addr_to_page_index(addr: impl MemAddr + AsUsize) -> usize {
        addr.as_usize() >> MEMORY_DEFAULT_SHIFT
    }

    // calculates the page index (in MEMORY_DEFAULT_PAGE_SIZE units) given a usize
    #[inline(always)]
    pub fn usize_to_page_index(uaddr: usize) -> usize {
        uaddr >> MEMORY_DEFAULT_SHIFT
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub struct Vas {
    pub cr3: PhysAddr,
    pub base_page_table: Option<*mut BasePageTable>,
    pub owner: Owner,
}
impl AddrSpace for Vas {
    fn new() -> Self {
        Vas {
            cr3: ZERO_USIZE.as_phys(),
            base_page_table: None,
            owner: Owner::Memory,
        }
    }

    fn init(&mut self, base_page_table_addr: PhysAddr) {
        debug_assert!(base_page_table_addr.is_default_page_aligned());

        self.base_page_table = Some(
            raw::abracadabra_ptr_mut::<BasePageTable, PhysAddr>(base_page_table_addr, false),
        )
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn switch_to(&mut self) {
        self.init_cr3();

        if self.cr3 == ZERO_USIZE.as_phys() {
            panic!("vas::switch_to() -> Tried to switch to an address space with a null cr3");
        }

        x86_write_cr3(self.cr3.as_usize());
    }

    fn base_table(&self) -> Option<&BasePageTable> {
        if self.base_page_table.is_some() {
            unsafe { self.base_page_table.unwrap().as_ref() }
        } else {
            None
        }
    }

    fn base_table_mut(&mut self) -> Option<&mut BasePageTable> {
        if self.base_page_table.is_some() {
            unsafe { self.base_page_table.unwrap().as_mut() }
        } else {
            None
        }
    }

    fn as_mut(&mut self) -> &mut Self {
        self
    }

    fn as_ref(&self) -> &Self {
        self
    }
}

impl Vas {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub fn init_cr3(&mut self) -> Option<PhysAddr> {
        
        // if we've called this with a set of null
        // page tables, then we can bail
        if self.base_page_table.is_none() {
            return None;
        }

        // get the address of the base page table
        let mut p = raw::ref_to_raw::<BasePageTable, PhysAddr>(unsafe { self.base_page_table.unwrap().as_ref().unwrap() });
        p.inner_or(PAGING_WRITETHROUGH);
        self.cr3 = p;
        Some(p)
    }
}

// Page dir / table entries
// Level 4 - 512G, Level 3 - 1G, Level 2 - 2M, Level 1 - 4K
// x86 just has levels 2 (@4MB pages) & 1 (4KB pages)
pub type Pte = PhysAddr;

#[cfg(target_arch = "x86")]
#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

    pub const fn as_const_usize(&self) -> usize {
        self.0
    }
}

#[cfg(target_arch = "x86_64")]
#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageSize {
    Small = USIZE_4K,
    Medium = USIZE_2M,
    Huge = USIZE_1G,
}

#[cfg(target_arch = "x86_64")]
impl PageSize {
    #[inline(always)]
    pub const fn into_bits(self) -> usize {
        self as _
    }

    #[inline(always)]
    pub const fn from_bits(value: usize) -> Self {
        match value {
            USIZE_4K => PageSize::Small,
            USIZE_2M => PageSize::Medium,
            USIZE_1G => PageSize::Huge,
            _ => PageSize::Small,
        }
    }

    #[inline(always)]
    pub const fn as_const_usize(&self) -> usize {
        self.into_bits()
    }
}

// this should be able to be truly const, but it's not supported yet
impl const AsUsize for PageSize {
    fn as_usize(&self) -> usize {
        self.into_bits()
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct PageTable {
    pub entries: *mut [Pte; PAGE_TABLE_MAX_ENTRIES],
}
impl PageTable {
    pub fn new(base_addr: PhysAddr) -> Self {
        debug_assert!(base_addr.is_default_page_aligned());

        raw::memset_aligned(base_addr, MEMORY_DEFAULT_PAGE_USIZE, ZERO_USIZE);

        PageTable {
            entries: {
                unsafe { core::mem::transmute::<PhysAddr, *mut [Pte; PAGE_TABLE_MAX_ENTRIES]>(base_addr) }
            }
        }        
    }

    pub fn rebase(&mut self, base_addr: PhysAddr) {
        debug_assert!(base_addr.is_default_page_aligned());
        self.entries = unsafe { core::mem::transmute::<PhysAddr, *mut [Pte; PAGE_TABLE_MAX_ENTRIES]>(base_addr) };        
    }

    #[inline(always)]
    pub fn get_entries(&self) -> &[Pte; PAGE_TABLE_MAX_ENTRIES] {
        unsafe { &*self.entries }
    }

    #[inline(always)]
    pub fn get_entries_mut(&mut self) -> &mut [Pte; PAGE_TABLE_MAX_ENTRIES] {
        unsafe { &mut *self.entries }
    }

    #[inline(always)]
    pub fn set_entry(&mut self, idx: usize, entry: Pte) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx] = entry };
    }

    #[inline(always)]
    pub fn get_entry(&self, idx: usize) -> Pte {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx] }
    }

    #[inline(always)]
    pub fn clear_entry(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx] = ZERO_USIZE.as_phys() };
    }

    #[inline(always)]
    pub fn mark_entry_present(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_or(PAGING_PRESENT) };
    }

    #[inline(always)]
    pub fn mark_entry_not_present(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_and(!PAGING_PRESENT) };
    }

    #[inline(always)]
    pub fn mark_entry_rw(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_or(PAGING_WRITEABLE) };
    }

    #[inline(always)]
    pub fn mark_entry_readonly(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_and(!PAGING_WRITEABLE) };
    }

    #[inline(always)]
    pub fn mark_entry_user(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_or(PAGING_USERMODE) };
    }

    #[inline(always)]
    pub fn mark_entry_supervisor(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_and(!PAGING_USERMODE) };
    }

    #[inline(always)]
    pub fn mark_entry_write_through(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_or(PAGING_WRITETHROUGH) };
    }

    #[inline(always)]
    pub fn mark_entry_not_write_through(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_and(!PAGING_WRITETHROUGH) };
    }

    #[inline(always)]
    pub fn mark_entry_cache_disabled(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_or(PAGING_CACHE_DISABLE) };
    }

    #[inline(always)]
    pub fn mark_entry_cache_enabled(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_and(!PAGING_CACHE_DISABLE) };
    }

    #[inline(always)]
    pub fn mark_entry_accessed(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_or(PAGING_ACCESSED) };
    }

    #[inline(always)]
    pub fn mark_entry_not_accessed(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_and(!PAGING_ACCESSED) };
    }

    #[inline(always)]
    pub fn mark_entry_dirty(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_or(PAGING_DIRTY) };
    }

    #[inline(always)]
    pub fn mark_entry_clean(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_and(!PAGING_DIRTY) };
    }

    #[inline(always)]
    pub fn mark_entry_global(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_or(PAGING_GLOBAL) };
    }

    #[inline(always)]
    pub fn clear_entry_global(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_and(!PAGING_GLOBAL) };
    }

    #[inline(always)]
    pub fn mark_entry_page_frame(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_or(PAGING_IS_PAGE_FRAME) };
    }

    #[inline(always)]
    pub fn mark_entry_page_directory(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_and(!PAGING_IS_PAGE_FRAME) };
    }

    #[inline(always)]
    pub fn is_entry_page_frame(&self, idx: usize) -> bool {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { ubit::is_bit_set(self.entries.as_mut().unwrap()[idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) }
    }

    #[inline(always)]
    pub fn mark_entry_no_execute(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_or(PAGING_NX) };
    }

    #[inline(always)]
    pub fn mark_entry_executable(&mut self, idx: usize) {
        debug_assert!(idx < PAGE_TABLE_MAX_ENTRIES);
        unsafe { self.entries.as_mut().unwrap()[idx].inner_and(!PAGING_NX) };
    }
}

//#[cfg(target_arch = "x86")]

pub type BasePageTable = PageTable;

impl PageDir for BasePageTable {
    // creates a new address space for the specified owner
    fn new_addr_space(owner: Owner) -> Option<PhysAddr> {
        
        // make sure we're not calling this before we
        // have a frame allocator
        debug_assert!(frame_alloc_fuse(true));

        // allocate for a new page directory
        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("BasePageTable::new_addr_space() -> preparing new address space for {:?}", owner);

        let neb = iron().unwrap();

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("BasePageTable::new_addr_space() -> iron: nebulae @ 0x{:08x}", neb as *const Nebulae as usize);

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("BasePageTable::new_addr_space() -> allocating base paging struct");

        // allocate for a new base paging struct
        let new_base_pd_frame_result = 
            neb.frame_alloc_internal_04
                .lock_rw_spin().as_mut().unwrap().as_mut().unwrap()
                .alloc_frame(MEMORY_DEFAULT_PAGE_USIZE, MEMORY_DEFAULT_PAGE_SIZE_ENUM, owner);

        // make sure we got a frame; if not, we're out of memory
        if new_base_pd_frame_result.is_none() {
            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("BasePageTable::new_addr_space() -> out of memory when allocating for a new base paging struct for {:?}", owner);
            return None;
        } else {
            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("BasePageTable::new_addr_space() -> new base paging struct allocated @ 0x{:0x} for {:?}", new_base_pd_frame_result.unwrap(), owner);
        }

        Some(new_base_pd_frame_result.unwrap())
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
                    .alloc_frame_single(Owner::Kernel, MEMORY_DEFAULT_PAGE_SIZE_ENUM);

            match new_pt_base {
                None => return None,
                Some(np) => {
                    self.entries[pd_idx] = np;
                    self.identity_map_page(
                        np,
                        Owner::Memory,
                        MEMORY_DEFAULT_PAGE_SIZE_ENUM,
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
        page_size: PageSize,
        flags: usize,
    ) -> Option<VirtAddr> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = v.get_page_table_indexes();

        debug_assert!(p.is_aligned(page_size.as_usize()) && v.is_aligned(page_size.as_usize()));

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("BasePageTable::map_page() -> mapping page @ 0x{:0x} to 0x{:0x} with size {} and flags 0x{:0x}", p, v, page_size.as_usize(), flags);

        let pdpt: &mut PageTable;
        let pdpt_is_new: bool;
        let pd: &mut PageTable;
        let pd_is_new: bool;
        let pt: &mut PageTable;

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("BasePageTable::map_page() -> self.entries[] == 0x{:0x}", self.entries as usize);

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("BasePageTable::map_page() -> page map level 4 index == {}", pml4_idx);

        let my_entries = unsafe { self.entries.as_mut().unwrap() };

        // check our entry in the pml4 table, which maps 512GB chunks
        // create a new pdpt if one does not exist
        if my_entries[pml4_idx] == ZERO_USIZE.as_phys() {
            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("BasePageTable::map_page() -> allocating frame for new pdpt");
            
            let new_pdpt_base = 
                iron().unwrap().frame_alloc_internal_04.as_mut()
                .lock_rw_spin().as_mut().unwrap().as_mut().unwrap()
                .alloc_frame(MEMORY_DEFAULT_PAGE_USIZE, MEMORY_DEFAULT_PAGE_SIZE_ENUM, Owner::Memory);

            match new_pdpt_base {
                None => return None,
                Some(np) => {
                    #[cfg(all(debug_assertions, feature = "serialdbg"))]
                    serial_println!("BasePageTable::map_page() -> new frame obtained for new pdpt @ 0x{:0x}", np);

                    #[cfg(all(debug_assertions, feature = "serialdbg"))]
                    serial_println!("BasePageTable::map_page() -> identity mapping new pdpt @ 0x{:0x}", np);
                   
                    self.identity_map_page (
                        np,
                        MEMORY_DEFAULT_PAGE_SIZE_ENUM,
                        PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                    );
                    
                    my_entries[pml4_idx] = np;
                    my_entries[pml4_idx]
                        .inner_or(PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH);
                    pdpt_is_new = true;
                }
            }

            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("BasePageTable::map_page() -> new pdpt frame created & identity mapped");
        } else {
            #[cfg(all(debug_assertions, feature = "serialdbg"))]
            serial_println!("BasePageTable::map_page() -> pdpt located: 0x{:0x}", my_entries[pml4_idx]);

            pdpt_is_new = false;
        }

        // check our entry in the pdpt table, which maps 1GB chunks.
        // create a reference to our pdpt
        pdpt = raw::abracadabra_static_ref_mut::<PageTable>(my_entries[pml4_idx].align_canon_default(), false);
        let pdpt_entries = unsafe { pdpt.entries.as_mut().unwrap() };
        
        // see if we're doing a 1GB page. if so, mark it as a page and clean up if necessary
        if page_size == PageSize::Huge {
            
            // Since we're mapping a huge page, we need to remove any mappings that may have been
            // present as 2MB or 4KB pages underneath, so we don't leak memory
            
            // if we just created this pdpt, then there's nothing to remove
            if !pdpt_is_new {

                // check to see if this entry already pointed to a page frame.
                // if this was already a 1GB page frame pointer, then there will
                // be no pds or pts to remove
                if !ubit::is_bit_set(pdpt_entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                    
                    // follow the old pdpt entry to its page directory and obtain a reference to it
                    let local_pd = raw::abracadabra_static_ref_mut::<PageTable>(
                        pdpt_entries[pdpt_idx].align_canon_default(), false,
                    );
                    let local_pd_entries = unsafe { local_pd.entries.as_mut().unwrap() };

                    // this pdpt entry is either a pointer to a 2MB page frame, or a pointer to
                    // another page directory whose entries point to 4KB pages.

                    // if they're not 2MB page entries, then we need to
                    // de-allocate the page tables under this page directory.
                    
                    // the final step is to set the entry to zero, which is done regardless of
                    // whether the entry was a 2MB page or a page directory
                    for i in 0..PAGE_TABLE_MAX_ENTRIES {
                        if !ubit::is_bit_set(
                            local_pd_entries[i].as_usize(),
                            PAGING_IS_PAGE_FRAME_BIT,
                        ) {
                            iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().dealloc_frame(
                                local_pd_entries[i].align_canon_default(),
                                Owner::Memory,
                            );

                            self.unmap_page(
                                local_pd_entries[i].align_canon_default().as_usize().as_virt(),
                                Owner::Memory,
                                MEMORY_DEFAULT_PAGE_SIZE_ENUM,
                            );
                        }
                    }

                    // now de-allocate the page directory itself
                    iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().dealloc_frame(
                        pdpt_entries[pdpt_idx].align_canon_default(),
                        Owner::Memory);

                    self.unmap_page(
                        pdpt_entries[pdpt_idx].align_canon_default().as_usize().as_virt(),
                        Owner::Memory,
                        MEMORY_DEFAULT_PAGE_SIZE_ENUM,
                    );
                }
            }

            // Map our huge page
            pdpt_entries[pdpt_idx] = p;
            pdpt_entries[pdpt_idx].inner_or(flags | PAGING_IS_PAGE_FRAME);

            // Now that we've cleaned up the already mapped pages,
            // and we've mapped the 1GB page, we can return
            x86_invalidate_page(v.as_usize());

            // mark the owner for this page in the page info struct
            let mut page_info_struct_lock = iron().unwrap().page_info_structs_01.lock_rw_spin();
            if page_info_struct_lock.as_ref().is_some() {
                let pageidx = pages::addr_to_page_index(p);
                page_info_struct_lock.as_mut().unwrap().as_mut().unwrap()[pageidx].held_by_fiber_id = ZERO_U128;
            }
            
            return Some(v);
        } // PageSize::Huge

        // create a new pd if one does not exist
        if pdpt_entries[pdpt_idx] == ZERO_USIZE.as_phys() {
            let new_pd_base = 
                iron().unwrap().frame_alloc_internal_04
                    .lock_rw_spin().as_mut().unwrap().as_mut().unwrap()
                    .alloc_frame(MEMORY_DEFAULT_PAGE_USIZE, MEMORY_DEFAULT_PAGE_SIZE_ENUM, Owner::Memory);

            match new_pd_base {
                None => return None,
                Some(np) => {
                    pdpt_entries[pdpt_idx] = np;
                    self.identity_map_page(
                        np,
                        MEMORY_DEFAULT_PAGE_SIZE_ENUM,
                        PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                    );
                    pdpt_entries[pdpt_idx]
                        .inner_or(PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH);
                    pd_is_new = true;
                }
            }
        } else {
            pd_is_new = false;
        }

        // create a reference to our pd
        pd = raw::abracadabra_static_ref_mut::<PageTable>(pdpt_entries[pdpt_idx].align_canon_default(), false);
        let pd_entries = unsafe { pd.entries.as_mut().unwrap() };

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
                            pd_entries[pd_idx].as_usize(),
                            PAGING_IS_PAGE_FRAME_BIT,
                        ) {
                            // de-allocate every page under this page table
                            iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().dealloc_frame(
                                pd_entries[i].align_canon_default(),
                                Owner::Memory,
                            );

                            self.unmap_page(
                                pd_entries[i].align_canon_default().as_usize().as_virt(),
                                Owner::Memory,
                                MEMORY_DEFAULT_PAGE_SIZE_ENUM,
                            );
                        }
                    }

                    // now de-allocate the page table itself
                    iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().dealloc_frame(
                        pd_entries[pd_idx].align_canon_default(),
                        Owner::Memory,
                    );

                    self.unmap_page(
                        pd_entries[pd_idx].align_canon_default().as_usize().as_virt(),
                        Owner::Memory,
                        MEMORY_DEFAULT_PAGE_SIZE_ENUM,
                    );
                }
            }

            // Map our medium page
            pd_entries[pd_idx] = p;
            pd_entries[pd_idx].inner_or(flags | PAGING_IS_PAGE_FRAME);

            // Now that we've cleaned up the already mapped pages,
            // and we've mapped the 2MB page, we can return
            x86_invalidate_page(v.as_usize());
            
            return Some(v);
        } // PageSize::Medium

        // This must be a 4KB page

        // create a new pt if one does not exist
        if pd_entries[pd_idx] == ZERO_USIZE.as_phys() {
            let new_pt_base = 
                iron().unwrap().frame_alloc_internal_04
                    .lock_rw_spin().as_mut().unwrap().as_mut().unwrap()
                    .alloc_frame(MEMORY_DEFAULT_PAGE_USIZE, MEMORY_DEFAULT_PAGE_SIZE_ENUM, Owner::Memory);

            match new_pt_base {
                None => return None,
                Some(np) => {
                    pd_entries[pd_idx] = np;
                    self.identity_map_page(
                        np,
                        MEMORY_DEFAULT_PAGE_SIZE_ENUM,
                        PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH,
                    );
                    pd_entries[pd_idx]
                        .inner_or(PAGING_PRESENT | PAGING_WRITEABLE | PAGING_WRITETHROUGH);
                }
            }
        }

        // create a reference to our pt
        pt = raw::abracadabra_static_ref_mut::<PageTable>(pd_entries[pd_idx].align_canon_default(), false);
        let pt_entries = unsafe { pt.entries.as_mut().unwrap() };

        // Map our small page
        // no page frame flag for 4KB pages
        pt_entries[pt_idx] = p;
        pt_entries[pt_idx].inner_or(flags);

        // signal that the old page mapping is no longer valid
        x86_invalidate_page(v.as_usize());

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
    fn unmap_page(&mut self, v: VirtAddr, owner: Owner, page_size: PageSize) -> bool {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = v.get_page_table_indexes();

        let pdpt: &mut PageTable;
        let pd: &mut PageTable;
        let pt: &mut PageTable;

        match page_size {
            PageSize::Small => {
                if !v.is_aligned_4k() {
                    return false;
                }
            }
            PageSize::Medium => {
                if !v.is_aligned_2m() {
                    return false;
                }
            }
            PageSize::Huge => {
                if !v.is_aligned_1g() {
                    return false;
                }
            }
        }

        let fn_unmap_page_info = |p: PhysAddr, page_size: PageSize| {
            let mut page_info_struct_results = iron().unwrap().page_info_structs_01.lock_rw_spin();
            if page_info_struct_results.is_none() {
                return;
            }

            let page_info_structs = (*page_info_struct_results).as_mut().unwrap();
            let page_idx = pages::addr_to_page_index(p);

            if page_size > MEMORY_DEFAULT_PAGE_SIZE_ENUM {
                // if this is a huge or medium page, then we need to clear the owner and status
                // for every page underneath
                let page_count = pages::bytes_to_pages(page_size.as_usize(), MEMORY_DEFAULT_PAGE_SIZE_ENUM);
                for i in 0..page_count {
                    page_info_structs[page_idx + i].held_by_fiber_id = ZERO_U128;
                    page_info_structs[page_idx + i].status = pages::PageStatus::Free;
                }
            } else {
                page_info_structs[page_idx].held_by_fiber_id = ZERO_U128;
                page_info_structs[page_idx].status = pages::PageStatus::Free;
            }
        };

        let my_entries = unsafe { self.entries.as_mut().unwrap() };

        // check our entry in the pml4 table, which maps 512GB chunks
        if my_entries[pml4_idx] == ZERO_USIZE.as_phys() {
            // if the entry is already 0, then there's nothing to do
            return true;
        }

        // check our entry in the pdpt table, which maps 1GB chunks
        // create a reference to our pdpt
        pdpt = raw::abracadabra_static_ref_mut::<PageTable>(my_entries[pml4_idx].align_canon_default(), false);
        let pdpt_entries = unsafe { pdpt.entries.as_mut().unwrap() };

        // see if we're unmapping a 1GB page. if so, mark it as zero and clean up if necessary
        if page_size == PageSize::Huge {
            // Since we're unmapping a huge page, we need to remove any mappings that may have been
            // present as 2MB or 4KB pages underneath, so we don't have a memory leak

            // don't do anything if this was already a 1GB page, since there will be no pds or pts to remove
            if !ubit::is_bit_set(pdpt_entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                // follow the old pdpt entry to its page directory
                let local_pd = raw::abracadabra_static_ref_mut::<PageTable>(
                    pdpt_entries[pdpt_idx].align_canon_default(), false,
                );
                let local_pd_entries = unsafe { local_pd.entries.as_mut().unwrap() };

                // if they're not 2MB page entries, then we need to
                // de-allocate every page table under this page directory
                for i in 0..PAGE_TABLE_MAX_ENTRIES {
                    if !ubit::is_bit_set(local_pd_entries[i].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                        iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().dealloc_frame(
                            local_pd_entries[i].align_canon_default(),
                            Owner::Memory,
                        );

                        self.unmap_page(
                            local_pd_entries[i].align_canon_default().as_usize().as_virt(),
                            Owner::Memory,
                            MEMORY_DEFAULT_PAGE_SIZE_ENUM,
                        );
                    }
                }

                // now de-allocate the page directory itself
                iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().dealloc_frame(
                    pdpt_entries[pdpt_idx].align_canon_default(),
                    Owner::Kernel,
                );

                self.unmap_page(
                    pdpt_entries[pdpt_idx].align_canon_default().as_usize().as_virt(),
                    owner,
                    MEMORY_DEFAULT_PAGE_SIZE_ENUM,
                );
            }

            // Clear our owner information
            fn_unmap_page_info(pdpt_entries[pdpt_idx].align_1g(), page_size);

            // Unmap our huge page
            pdpt_entries[pdpt_idx] = ZERO_USIZE.as_phys();
            x86_invalidate_page(v.as_usize());
        } // PageSize::Huge

        if pdpt_entries[pdpt_idx] == ZERO_USIZE.as_phys() {
            // if the entry is already 0, then there's nothing to do
            return true;
        }

        // create a reference to our pd
        pd = raw::abracadabra_static_ref_mut::<PageTable>(pdpt_entries[pdpt_idx].align_canon_default(), false);
        let pd_entries = unsafe { pd.entries.as_mut().unwrap() };

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
                    if !ubit::is_bit_set(pd_entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
                        // de-allocate every page under this page table
                        iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().dealloc_frame(
                            pd_entries[i].align_canon_default(),
                            Owner::Memory,
                        );

                        self.unmap_page(
                            pd_entries[i].align_canon_default().as_usize().as_virt(),
                            Owner::Memory,
                            MEMORY_DEFAULT_PAGE_SIZE_ENUM,
                        );
                    }
                }

                // now de-allocate the page directory itself
                iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().dealloc_frame(
                    pdpt_entries[pdpt_idx].align_canon_default(),
                    Owner::Memory,
                );

                self.unmap_page(
                    pdpt_entries[pdpt_idx].align_canon_default().as_usize().as_virt(),
                    Owner::Memory,
                    MEMORY_DEFAULT_PAGE_SIZE_ENUM,
                );
            }

            // Clear our owner information
            fn_unmap_page_info(pd_entries[pd_idx].align_canon_2m(), page_size);

            // unmap our medium page
            pd_entries[pd_idx] = ZERO_USIZE.as_phys();
            x86_invalidate_page(v.as_usize());
        } // PageSize::Medium

        if pd_entries[pd_idx] == ZERO_USIZE.as_phys() {
            // if the entry is already 0, then there's nothing to do
            return true;
        }

        // This is a 4KB page

        // create a reference to our pt
        pt = raw::abracadabra_static_ref_mut::<PageTable>(pd_entries[pd_idx].align_canon_default(), false);
        let pt_entries = unsafe { pt.entries.as_mut().unwrap() };

        // Clear our owner information
        fn_unmap_page_info(pt_entries[pt_idx].align_canon_default(), page_size);

        // Unmap our small page
        // no page frame flag for 4KB pages
        pt_entries[pt_idx] = ZERO_USIZE.as_phys();
        x86_invalidate_page(v.as_usize());
        true
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

        let my_entries = unsafe { self.entries.as_mut().unwrap() };

        // check our entry in the pml4 table, which maps 512GB chunks
        if my_entries[pml4_idx] == ZERO_USIZE.as_phys() {
            // if the entry is 0, then there's nothing to do
            return ZERO_USIZE.as_phys();
        }

        // check our entry in the pdpt table, which maps 1GB chunks
        // create a reference to our pdpt
        pdpt = raw::abracadabra_static_ref_mut::<PageTable>(my_entries[pml4_idx].align_canon_default(), false);
        let pdpt_entries = unsafe { pdpt.entries.as_mut().unwrap() };

        // see if we're getting a 1GB page. if so, return it
        if ubit::is_bit_set(pdpt_entries[pdpt_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            return pdpt_entries[pdpt_idx].align_canon_1g();
        } // PageSize::Huge

        if pdpt_entries[pdpt_idx] == ZERO_USIZE.as_phys() {
            // if the entry is 0, then there's nothing to do
            return ZERO_USIZE.as_phys();
        }

        // create a reference to our pd
        pd = raw::abracadabra_static_ref_mut::<PageTable>(pdpt_entries[pdpt_idx].align_canon_default(), false);
        let pd_entries = unsafe { pd.entries.as_mut().unwrap() };

        // see if we're getting a 2MB page. if so, return it
        if ubit::is_bit_set(pd_entries[pd_idx].as_usize(), PAGING_IS_PAGE_FRAME_BIT) {
            return pd_entries[pd_idx].align_2m();
        } // PageSize::Medium

        if pd_entries[pd_idx] == ZERO_USIZE.as_phys() {
            // if the entry is 0, then there's nothing to do
            return ZERO_USIZE.as_phys();
        }

        // This is a 4KB page

        // create a reference to our pt
        pt = raw::abracadabra_static_ref_mut::<PageTable>(pd_entries[pd_idx].align_canon_default(), false);
        let pt_entries = unsafe { pt.entries.as_mut().unwrap() };

        // return our small page
        return pt_entries[pt_idx].align_canon_default();
    }

    fn dealloc_page(&mut self, v: VirtAddr, owner: Owner, page_size: PageSize) {
        self.unmap_page(v, owner, page_size);
        iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().dealloc_frame(
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
        let page_count = pages::bytes_to_pages(size, MEMORY_DEFAULT_PAGE_SIZE_ENUM);
        let mut cv: VirtAddr = v;

        for _i in 0..page_count {
            self.unmap_page(cv, owner, page_size);
            iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().dealloc_frame(
                self.virt_to_phys(cv),
                Owner::Nobody,
            );
            cv.inner_inc_by_page_size(page_size);
        }
    }

    fn identity_map_page(&mut self, p: PhysAddr, page_size: PageSize, flags: usize) {
        self.map_page(
            p,
            p.align_canon_default().as_usize().as_virt(),
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
        bit_pattern: BytePattern,
    ) -> VirtAddr {
        let new_page_frame_base = 
            iron().unwrap().frame_alloc_internal_04
                .lock_rw_spin().as_mut().unwrap().as_mut().unwrap()
                .alloc_frame(MEMORY_DEFAULT_PAGE_USIZE, page_size, owner);

        // Looks like we were not able to obtain a page
        if new_page_frame_base.is_none() {
            return ZERO_USIZE.as_virt();
        }

        // Map the new page frame to where it was requested
        self.map_page(new_page_frame_base.unwrap(), v, page_size, flags);

        // fill the allocated memory with the bit pattern
        raw::memset_aligned(v.as_usize().as_phys(), page_size.as_usize(), bit_pattern.as_usize_pattern());

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
        bit_pattern: BytePattern,
    ) -> Option<VirtAddr> {

        let size_in_pages = pages::bytes_to_pages(size, MEMORY_DEFAULT_PAGE_SIZE_ENUM);
        let size_in_bytes = pages::pages_to_bytes(size_in_pages, page_size);
        let mut allocated_pages: usize = 0;

        let mut va = v.clone();

        for _i in 0..size_in_pages {
            let page_base = 
                iron().unwrap().frame_alloc_internal_04
                    .lock_rw_spin().as_mut().unwrap().as_mut().unwrap()
                    .alloc_frame(page_size.as_usize(), page_size, owner);

            if page_base.is_some() {
                allocated_pages += 1;

                self.map_page(
                    page_base.unwrap(),
                    va.clone(),
                    page_size,
                    flags,
                );

                va.inner_inc_by_page_size(page_size);

            } else {
                // we need to deallocate and unmap any pages that were allocated and return failure
                let mut cv: VirtAddr = v.clone();

                for _j in 0..allocated_pages {
                    self.unmap_page(cv, owner, page_size);
                    iron().unwrap().frame_alloc_internal_04.lock_rw_spin().as_mut().unwrap().as_mut().unwrap().dealloc_frame(
                        self.virt_to_phys(cv),
                        owner,
                    );
                    cv.inner_inc_by_page_size(MEMORY_DEFAULT_PAGE_SIZE_ENUM);
                }

                return None;
            }
        }

        // fill the allocated memory with the bit pattern
        raw::memset_aligned(v.as_usize().as_phys(), size_in_bytes, bit_pattern.as_usize_pattern());
        Some(v)
    }

    fn alloc_pages_fixed_virtual(
        &mut self,
        size: usize,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
        bit_pattern: BytePattern,
    ) -> Option<VirtAddr> {
        
        let size_in_pages = pages::bytes_to_pages(size, page_size);
        let page_base = 
            iron().unwrap().frame_alloc_internal_04
                .lock_rw_spin()
                .as_mut().unwrap()
                .as_mut().unwrap()
                .alloc_frame(size, page_size, owner);
            
        if page_base.is_some() {
            for i in 0..size_in_pages {
                self.map_page(
                    page_base.unwrap(),
                    VirtAddr(v.as_usize() + (i * page_size.as_usize())),
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
                raw::abracadabra_ptr_mut::<u8, VirtAddr>(v, false),
                bit_pattern.into_bits(),
                size,
            );
        }

        Some(v)
    }
}