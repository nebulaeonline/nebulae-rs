pub use crate::frame_alloc::*;

use core::convert::{From, Into};
use core::mem;

use crate::common::*;

#[cfg(target_arch = "x86")]
pub use crate::arch::x86::vmem32::*;
#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86::vmem64::*;

// CONSTANTS
pub const MEMORY_TYPE_BOOT_FRAMER: u32 = 0x80015225;
pub const MEMORY_TYPE_UEFI_MEM_MAP: u32 = 0x80025225;

// THE REST
pub trait MemAddr: HasAsUsize {
    fn inner(&self) -> usize;

    fn new() -> Self;
    fn new_from(item: Self) -> Self;
    
    fn new_from_uintn(item: Uintn) -> Self;
    fn new_from_usize(item: usize) -> Self;

    fn as_uintn(&self) -> Uintn {
        self.inner() as Uintn
    }

    #[cfg(target_arch = "x86")]
    fn get_page_table_indexes(&self) -> (usize, usize) {
        let pd_idx = (self.as_usize() & 0xFFC00000) >> 22;
        let pt_idx = (self.as_usize() & 0x3FF) >> 12;

        (pd_idx, pt_idx)
    }

    #[cfg(target_arch = "x86_64")]
    fn get_page_table_indexes(&self) -> (usize, usize, usize, usize) {
        let pml4_idx = (self.as_usize() & 0x7F8000000000) >> 39;
        let pdpt_idx = (self.as_usize() & 0x7FC0000000) >> 30;
        let pd_idx = (self.as_usize() & 0x3FE00000) >> 21;
        let pt_idx = (self.as_usize() & 0x1FF000) >> 12;

        (pml4_idx, pdpt_idx, pd_idx, pt_idx)
    }
}

impl Bitmask for PhysAddr {
    fn bitmask(&self, mask: usize) -> Self {
        PhysAddr(self.0 & mask)
    }
}

pub trait HasAsUsize {
    fn as_usize(&self) -> usize;
}

impl HasAsUsize for Uintn {
    fn as_usize(&self) -> usize {
        *self as usize
    }
}

impl HasAsUsize for usize {
    fn as_usize(&self) -> usize {
        *self
    }
}

impl HasAsUsize for PhysAddr {
    fn as_usize(&self) -> usize {
        self.0
    }
}

pub trait Align: Bitmask + Sized + PartialEq + HasAsUsize {
    fn align_4k(&self) -> Self {
        self.bitmask(ALIGN_MASK_4K)
    }

    fn is_aligned_4k(&self) -> bool {
        let x = self.bitmask(ALIGN_MASK_4K).as_usize();
        let y = self.as_usize();
        x == y
    }

    #[cfg(target_arch = "x86_64")]
    fn align_2m(&self) -> Self {
        self.bitmask(ALIGN_MASK_2M)
    }

    #[cfg(target_arch = "x86_64")]
    fn is_aligned_2m(&self) -> bool {
        let x = self.bitmask(ALIGN_MASK_2M).as_usize();
        let y = self.as_usize();
        x == y
    }

    #[cfg(target_arch = "x86")]
    fn align_4m(&self) -> Self {
        self.bitmask(ALIGN_MASK_4M)
    }

    #[cfg(target_arch = "x86")]
    fn is_aligned_4m(&self) -> bool {
        let x = self.bitmask(ALIGN_MASK_4M).as_usize();
        let y = self.as_usize();
        x == y
    }

    #[cfg(target_arch = "x86_64")]
    fn align_1g(&self) -> Self {
        self.bitmask(ALIGN_MASK_1G)
    }

    #[cfg(target_arch = "x86_64")]
    fn is_aligned_1g(&self) -> bool {
        let x = self.bitmask(ALIGN_MASK_1G).as_usize();
        let y = self.as_usize();
        x == y
    }
}

impl Align for usize { }
impl Align for Uintn { }

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct PhysAddr(pub usize);
impl PhysAddr {
    pub fn inner_and(&mut self, mask: usize) {
        self.0 &= mask;
    }

    pub fn inner_or(&mut self, mask: usize) {
        self.0 |= mask;
    }

    pub fn inner_xor(&mut self, mask: usize) {
        self.0 ^= mask;
    }

    pub fn inner_bitnot(&mut self) {
        self.0 = !self.0;
    }

    pub fn inner_add(&mut self, x: usize) {
        self.0 += x;
    }

    pub fn inner_sub(&mut self, x: usize) {
        self.0 -= x;
    }

    pub fn inner_inc<T>(&mut self, n: usize) {
        self.0 += mem::size_of::<T>() * n;
    }

    pub fn inner_dec<T>(&mut self, n: usize) {
        self.0 -= mem::size_of::<T>() * n;
    }

    pub fn inner_inc_by_default_page_size(&mut self) {
        self.0 += MEMORY_DEFAULT_PAGE_USIZE;
    }

    pub fn inner_dec_by_default_page_size(&mut self) {
        self.0 -= MEMORY_DEFAULT_PAGE_USIZE;
    }

    pub fn inner_inc_by_page_size(&mut self, page_size: PageSize) {
        self.0 += page_size.into_bits();
    }

    pub fn inner_dec_by_page_size(&mut self, page_size: PageSize) {
        self.0 -= page_size.into_bits();
    }
}

impl const MemAddr for PhysAddr {

    fn inner(&self) -> usize {
        self.0
    }

    fn new() -> Self {
        Self(0usize)
    }

    fn new_from(item: Self) -> Self {
        Self(item.as_usize())
    }

    fn new_from_uintn(item: Uintn) -> Self {
        Self(item as usize)
    }

    fn new_from_usize(item: usize) -> Self {
        Self(item)
    }
}

impl const Align for PhysAddr { }

impl const From<Uintn> for PhysAddr {
    fn from(item: Uintn) -> Self {
        Self(item as usize) 
    }
}

impl const From<usize> for PhysAddr {
    fn from(item: usize) -> Self {
        Self(item)
    }
}

impl const From<PhysAddr> for Uintn {
    fn from(item: PhysAddr) -> Self {
        item.into()
    }
}

impl const From<PhysAddr> for usize {
    fn from(item: PhysAddr) -> Self {
        item.into()
    }
}

impl core::fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[phys {:#0x}]", self.0)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct VirtAddr(pub usize);
impl VirtAddr {
    pub fn inner_and(&mut self, mask: usize) {
        self.0 &= mask;
    }

    pub fn inner_or(&mut self, mask: usize) {
        self.0 |= mask;
    }

    pub fn inner_xor(&mut self, mask: usize) {
        self.0 ^= mask;
    }

    pub fn inner_bitnot(&mut self) {
        self.0 = !self.0;
    }

    pub fn inner_add(&mut self, x: usize) {
        self.0 += x;
    }

    pub fn inner_sub(&mut self, x: usize) {
        self.0 -= x;
    }

    pub fn inner_inc<T>(&mut self, n: usize) {
        self.0 += mem::size_of::<T>() * n;
    }

    pub fn inner_dec<T>(&mut self, n: usize) {
        self.0 -= mem::size_of::<T>() * n;
    }

    pub fn inner_inc_by_default_page_size(&mut self) {
        self.0 += MEMORY_DEFAULT_PAGE_USIZE;
    }

    pub fn inner_dec_by_default_page_size(&mut self) {
        self.0 -= MEMORY_DEFAULT_PAGE_USIZE;
    }
}

impl Bitmask for VirtAddr {
    fn bitmask(&self, mask: usize) -> Self {
        VirtAddr(self.0 & mask)
    }
}

impl HasAsUsize for VirtAddr {
    fn as_usize(&self) -> usize {
        self.0
    }
}

impl const MemAddr for VirtAddr {

    fn inner(&self) -> usize {
        self.0
    }
    
    fn new() -> Self {
        Self(0usize)
    }

    fn new_from(item: Self) -> Self {
        Self(item.as_usize())
    }

    fn new_from_uintn(item: Uintn) -> Self {
        Self(item as usize)
    }

    fn new_from_usize(item: usize) -> Self {
        Self(item)
    }
}

impl const Align for VirtAddr { }

impl const From<Uintn> for VirtAddr {
    fn from(item: Uintn) -> Self {
        Self(item as usize) 
    }
}

impl const From<usize> for VirtAddr {
    fn from(item: usize) -> Self {
        Self(item)
    }
}

impl const From<PhysAddr> for VirtAddr {
    fn from(item: PhysAddr) -> Self {
        Self(item.0)
    }
}

impl const From<VirtAddr> for Uintn {
    fn from(item: VirtAddr) -> Self {
        item.into()
    }
}

impl const From<VirtAddr> for usize {
    fn from(item: VirtAddr) -> Self {
        item.into()
    }
}

impl core::fmt::Debug for VirtAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[virt {:#0x}]", self.0)
    }
}

// Traits for the virtual memory subsystem
pub trait PageDir {
    fn new_base() -> PhysAddr;
    fn virt_to_phys(&self, vaddr: VirtAddr) -> PhysAddr;
    fn map_page(&mut self, paddr: PhysAddr, vaddr: VirtAddr, page_size: PageSize, flags: usize) -> Option<VirtAddr>;
    fn unmap_page(&mut self, vaddr: VirtAddr, page_size: PageSize);
    fn identity_map_page(&mut self, paddr: PhysAddr, page_size: PageSize, flags: usize);
    fn alloc_page(&mut self, v: VirtAddr, size: PageSize, flags: usize) -> VirtAddr;
    fn alloc_pages(&mut self, size_in_pages: usize, v: VirtAddr, page_size: PageSize, flags: usize) -> Option<VirtAddr>;
    fn alloc_pages_contiguous(&mut self, size_in_pages: usize, v: VirtAddr, page_size: PageSize, flags: usize) -> Option<VirtAddr>;
}

// Common for tracking allocations / deallocations
#[derive(Debug)]
pub struct MemoryAllocationUnit {
    pub base: VirtAddr,
    pub size: usize,
}

pub const MEMORY_ALLOCATIONS_TRACKED_PER_PAGE: usize = MEMORY_DEFAULT_PAGE_USIZE / mem::size_of::<MemoryAllocationUnit>();

pub const fn calc_pages_reqd(size: usize) -> usize {
    (size + MEMORY_DEFAULT_PAGE_USIZE - 1) / MEMORY_DEFAULT_PAGE_USIZE
}
