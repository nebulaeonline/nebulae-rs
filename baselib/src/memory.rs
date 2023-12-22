use crate::common::base::*;
pub use crate::frame_alloc::*;

use core::slice;
use core::convert::{From, Into};
use core::mem;
use core::fmt;

pub use crate::vmem::{BasePageTable, PageSize, Vas};

// CONSTANTS
pub const MEMORY_TYPE_BOOT_FRAMER: u32 = 0x80015225;
pub const MEMORY_TYPE_UEFI_MEM_MAP: u32 = 0x80025225;
pub const MEMORY_TYPE_MEMORY_SUBSYSTEM: u32 = 0x80035225;
pub const MEMORY_TYPE_BOOT_FRAMER_BITMAP: u32 = 0x80045225;

pub const MEMORY_NATURAL_ALIGN: u64 = 2 * MACHINE_BYTES;
pub const MEMORY_NATURAL_UALIGN: usize = MEMORY_NATURAL_ALIGN as usize;

// THE REST
pub trait MemAddr: AsUsize {
    fn inner(&self) -> usize;

    fn new() -> Self;
    fn new_from(item: Self) -> Self;
    fn new_from_usize(item: usize) -> Self;

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

pub trait AsPhys {
    fn as_phys(&self) -> PhysAddr;
}

pub trait AsVirt {
    fn as_virt(&self) -> VirtAddr;
}

impl Bitmask for PhysAddr {
    fn bitmask(&self, mask: usize) -> Self {
        ubit::apply_mask(self.0, mask).as_phys()
    }
}

impl AsUsize for PhysAddr {
    fn as_usize(&self) -> usize {
        self.0
    }
}

pub trait Align: From<usize> + Bitmask + Sized + PartialEq + AsUsize {
    #[inline(always)]
    fn align_4k(&self) -> Self {
        self.bitmask(ALIGN_MASK_4K)
    }

    #[inline(always)]
    fn is_aligned_4k(&self) -> bool {
        let x = self.align_4k().as_usize();
        let y = self.as_usize();
        x == y
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn align_16k(&self) -> Self {
        self.bitmask(ALIGN_MASK_16K)
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn is_aligned_16k(&self) -> bool {
        let x = self.align_16k().as_usize();
        let y = self.as_usize();
        x == y
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn align_2m(&self) -> Self {
        self.bitmask(ALIGN_MASK_2M)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn is_aligned_2m(&self) -> bool {
        let x = self.align_2m().as_usize();
        let y = self.as_usize();
        x == y
    }

    #[inline(always)]
    #[cfg(target_arch = "x86")]
    fn align_4m(&self) -> Self {
        self.bitmask(ALIGN_MASK_4M)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86")]
    fn is_aligned_4m(&self) -> bool {
        let x = self.align_4m().as_usize();
        let y = self.as_usize();
        x == y
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn align_64k(&self) -> Self {
        self.bitmask(ALIGN_MASK_64K)
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn is_aligned_64k(&self) -> bool {
        let x = self.align_64k().as_usize();
        let y = self.as_usize();
        x == y
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn align_1g(&self) -> Self {
        self.bitmask(ALIGN_MASK_1G)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn is_aligned_1g(&self) -> bool {
        let x = self.align_1g().as_usize();
        let y = self.as_usize();
        x == y
    }

    #[inline(always)]
    fn is_default_page_aligned(&self) -> bool {
        if self.as_usize() % MEMORY_DEFAULT_PAGE_USIZE == 0 {
            true
        } else {
            false
        }
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn is_page_aligned_greater_than_default(&self) -> bool {
        self.as_usize().is_aligned_2m() || self.as_usize().is_aligned_1g()
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn is_page_aligned_greater_than_default(&self) -> bool {
        self.as_usize().is_aligned_16k() || self.as_usize().is_aligned_64k()
    }

    #[inline(always)]
    fn align_up(&self, alignment: usize) -> Self {
        align_up(self.as_usize(), alignment).into()
    }

    #[inline(always)]
    fn align_down(&self, alignment: usize) -> Self {
        align_down(self.as_usize(), alignment).into()
    }

    #[inline(always)]
    fn is_aligned(&self, alignment: usize) -> bool {
        self.as_usize() % alignment == 0
    }

    #[inline(always)]
    fn is_default_page_size_aligned(&self) -> bool {
        self.as_usize() % MEMORY_DEFAULT_PAGE_USIZE == 0
    }
}

impl Align for usize {}

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

    pub fn inner_inc_by_type<T>(&mut self, n: usize) {
        self.0 += mem::size_of::<T>() * n;
    }

    pub fn inner_dec_by_type<T>(&mut self, n: usize) {
        self.0 -= mem::size_of::<T>() * n;
    }

    pub fn inner_inc(&mut self, n: usize) {
        self.0 += n;
    }

    pub fn inner_dec(&mut self, n: usize) {
        self.0 -= n;
    }

    pub fn inner_inc_by_page_size(&mut self, page_size: PageSize) {
        self.0 += page_size.as_usize();
    }

    pub fn inner_dec_by_page_size(&mut self, page_size: PageSize) {
        self.0 -= page_size.as_usize();
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

    fn new_from_usize(item: usize) -> Self {
        Self(item as usize)
    }
}

impl fmt::LowerHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

impl const Align for PhysAddr {}

#[cfg(target_pointer_width = "32")]
impl const From<u32> for PhysAddr {
    fn from(item: u32) -> Self {
        Self(item as usize)
    }
}

impl const From<u64> for PhysAddr {
    fn from(item: u64) -> Self {
        Self(item as usize)
    }
}

impl const From<usize> for PhysAddr {
    fn from(item: usize) -> Self {
        Self(item)
    }
}

impl const From<PhysAddr> for usize {
    fn from(item: PhysAddr) -> Self {
        item.inner()
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

    pub fn inner_inc_by_type<T>(&mut self, n: usize) {
        self.0 += mem::size_of::<T>() * n;
    }

    pub fn inner_dec_by_type<T>(&mut self, n: usize) {
        self.0 -= mem::size_of::<T>() * n;
    }

    pub fn inner_inc(&mut self, n: usize) {
        self.0 += n;
    }

    pub fn inner_dec(&mut self, n: usize) {
        self.0 -= n;
    }

    pub fn inner_inc_by_page_size(&mut self, page_size: PageSize) {
        self.0 += page_size.as_usize();
    }

    pub fn inner_dec_by_page_size(&mut self, page_size: PageSize) {
        self.0 -= page_size.as_usize();
    }
}

impl Bitmask for VirtAddr {
    fn bitmask(&self, mask: usize) -> Self {
        VirtAddr(self.0 & mask)
    }
}

impl AsUsize for VirtAddr {
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

    fn new_from_usize(item: usize) -> Self {
        Self(item as usize)
    }
}

impl fmt::LowerHex for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

impl const Align for VirtAddr {}

#[cfg(target_pointer_width = "32")]
impl const From<u32> for VirtAddr {
    fn from(item: u32) -> Self {
        Self(item as usize)
    }
}

impl const From<u64> for VirtAddr {
    fn from(item: u64) -> Self {
        Self(item as usize)
    }
}

impl const From<usize> for VirtAddr {
    fn from(item: usize) -> Self {
        Self(item as usize)
    }
}

impl const From<PhysAddr> for VirtAddr {
    fn from(item: PhysAddr) -> Self {
        Self(item.0)
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

impl AsPhys for usize {
    fn as_phys(&self) -> PhysAddr {
        PhysAddr(*self)
    }
}

impl AsVirt for usize {
    fn as_virt(&self) -> VirtAddr {
        VirtAddr(*self)
    }
}

#[cfg(target_pointer_width = "32")]
impl AsPhys for u32 {
    fn as_phys(&self) -> PhysAddr {
        PhysAddr(*self as usize)
    }
}

#[cfg(target_pointer_width = "32")]
impl AsVirt for u32 {
    fn as_virt(&self) -> VirtAddr {
        VirtAddr(*self as usize)
    }
}

impl AsPhys for u64 {
    fn as_phys(&self) -> PhysAddr {
        PhysAddr(*self as usize)
    }
}

impl AsVirt for u64 {
    fn as_virt(&self) -> VirtAddr {
        VirtAddr(*self as usize)
    }
}
// Traits for the virtual memory subsystem
pub trait PageDir {
    fn new_base() -> PhysAddr;
    fn virt_to_phys(&self, vaddr: VirtAddr) -> PhysAddr;
    fn map_page(
        &mut self,
        paddr: PhysAddr,
        vaddr: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
    ) -> Option<VirtAddr>;
    fn unmap_page(&mut self, vaddr: VirtAddr, owner: Owner, page_size: PageSize);
    fn dealloc_page(&mut self, vaddr: VirtAddr, owner: Owner, page_size: PageSize);
    fn dealloc_pages_contiguous(
        &mut self,
        v: VirtAddr,
        size: usize,
        owner: Owner,
        page_size: PageSize,
    );
    fn identity_map_page(
        &mut self,
        paddr: PhysAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
    );
    fn alloc_page_fixed(
        &mut self,
        v: VirtAddr,
        owner: Owner,
        size: PageSize,
        flags: usize,
        bit_pattern: BitPattern,
    ) -> VirtAddr;
    fn alloc_pages_fixed(
        &mut self,
        size_in_pages: usize,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
        bit_pattern: BitPattern,
    ) -> Option<VirtAddr>;
    fn alloc_pages_fixed_virtual(
        &mut self,
        size_in_pages: usize,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
        bit_pattern: BitPattern,
    ) -> Option<VirtAddr>;
}

// Common for tracking allocations / deallocations
#[derive(Debug)]
pub struct MemoryUnit<T: MemAddr + Align + Copy> {
    pub base: T,
    pub size: usize,
}
impl<T: MemAddr + Align + Copy> MemoryUnit<T> {
    pub fn new() -> Self {
        Self {
            base: T::new(),
            size: 0,
        }
    }

    pub fn new_from(item: Self) -> Self {
        Self {
            base: item.base,
            size: item.size,
        }
    }

    pub fn new_with(base: impl MemAddr, size: usize) -> Self {
        Self {
            base: T::new_from_usize(base.as_usize()),
            size: size,
        }
    }

    pub fn is_addr_page_aligned(&self, page_size: PageSize) -> bool {
        match page_size {
            PageSize::Small => self.base.is_aligned_4k(),
            #[cfg(target_arch = "aarch64")]
            PageSize::Medium => self.base.is_aligned_16k(),
            #[cfg(target_arch = "x86")]
            PageSize::Medium => self.base.is_aligned_4m(),
            #[cfg(target_arch = "x86_64")]
            PageSize::Medium => self.base.is_aligned_2m(),
            #[cfg(target_arch = "aarch64")]
            PageSize::Large => self.base.is_aligned_64k(),
            #[cfg(target_arch = "x86_64")]
            PageSize::Huge => self.base.is_aligned_1g(),
        }
    }

    pub fn page_count(&self, page_size: PageSize) -> usize {
        if self.is_addr_page_aligned(page_size) {
            self.size / page_size.as_usize()
        } else {
            self.size / page_size.as_usize() + 1
        }
    }

    pub fn max_addr(&self) -> usize {
        self.base.as_usize() + self.size - 1
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BitPattern {
    ZeroZero = 0u8,
    Custom(u8),
    FF = u8::MAX,
}
impl BitPattern {
    pub const fn into_bits(self) -> u8 {
        match self {
            BitPattern::ZeroZero => 0,
            BitPattern::FF => u8::MAX,
            BitPattern::Custom(x) => x,
        }
    }

    pub const fn from_bits(value: u8) -> Self {
        match value {
            0 => BitPattern::ZeroZero,
            u8::MAX => BitPattern::FF,
            _ => BitPattern::Custom(value),
        }
    }
}

pub trait AddrSpace {
    fn new() -> Self;
    fn switch_to(&mut self);
    fn identity_map_critical(&mut self);
}

pub trait FrameAllocator {
    fn new() -> Self;
    fn init(&mut self);
    fn alloc_frame(&mut self, size: usize, page_size: PageSize, owner: Owner) -> Option<PhysAddr>;
    fn alloc_frame_fixed(&mut self, phys_addr: PhysAddr, size: usize, page_size: PageSize, owner: Owner) -> Option<PhysAddr>;
    fn dealloc_frame(&mut self, page_base: PhysAddr, owner: Owner);
    fn free_page_count(&mut self) -> usize;
    fn free_mem_count(&mut self) -> usize;
    fn page_count(&self) -> usize;
    fn mem_count(&self) -> usize;
    fn is_memory_frame_free(&self, page_base: PhysAddr) -> bool;
    fn is_frame_index_free(&self, page_idx: usize) -> bool;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(usize)]
pub enum Owner {
    Nobody = 0,
    System = 1,
    Memory = 2,
    Uefi = 3,
    User(usize),
    Verboten = usize::MAX,
}

// the genesis
// These are the naughty functions that need to be
// used during scaffolding from the inside, but that
// should be eliminated long-term
pub mod raw {
    use super::*;
    use core::mem;

    // our raw memset
    #[inline(always)]
    pub fn memset_size(start_addr: PhysAddr, size: usize, value: u8) {
        unsafe {
            let base = core::slice::from_raw_parts_mut(start_addr.as_usize() as *mut u8, size);

            for i in 0..size {
                base[i] = value;
            }
        }
    }

    #[inline(always)]
    pub fn memset_size_aligned(start_addr: PhysAddr, size: usize, value: usize) {
        let usize_units = size / MACHINE_UBYTES;

        unsafe {
            let base =
                core::slice::from_raw_parts_mut(start_addr.as_usize() as *mut usize, usize_units);

            for i in 0..usize_units {
                base[i] = value;
            }
        }
    }

    #[inline(always)]
    pub fn memset_range_inclusive(start_addr: PhysAddr, end_addr: PhysAddr, value: u8) {
        let region_size = end_addr.as_usize() - start_addr.as_usize() + 1;
        unsafe {
            let base =
                core::slice::from_raw_parts_mut(start_addr.as_usize() as *mut u8, region_size);

            for i in 0..region_size {
                base[i] = value;
            }
        }
    }

    #[inline(always)]
    pub fn memset_range_exclusive(start_addr: PhysAddr, end_addr: PhysAddr, value: u8) {
        let region_size = end_addr.as_usize() - start_addr.as_usize();
        unsafe {
            let base =
                core::slice::from_raw_parts_mut(start_addr.as_usize() as *mut u8, region_size);

            for i in 0..region_size {
                base[i] = value;
            }
        }
    }

    // our raw memcpys
    #[inline(always)]
    pub fn memcpy(src_addr: PhysAddr, dest_addr: PhysAddr, size: usize) {
        unsafe {
            let src = core::slice::from_raw_parts(src_addr.as_usize() as *const u8, size);
            let dest = core::slice::from_raw_parts_mut(dest_addr.as_usize() as *mut u8, size);

            for i in 0..size {
                dest[i] = src[i];
            }
        }
    }

    // our raw memcpy_aligned
    #[inline(always)]
    pub fn memcpy_usize_aligned(src_addr: PhysAddr, dest_addr: PhysAddr, size_in_bytes: usize) {
        debug_assert!(src_addr.as_usize() % MACHINE_UBYTES == 0);
        debug_assert!(dest_addr.as_usize() % MACHINE_UBYTES == 0);

        let size_in_usize = size_in_bytes / MACHINE_UBYTES;

        unsafe {
            let src =
                core::slice::from_raw_parts(src_addr.as_usize() as *const usize, size_in_usize);
            let dest =
                core::slice::from_raw_parts_mut(dest_addr.as_usize() as *mut usize, size_in_usize);

            for i in 0..size_in_usize {
                dest[i] = src[i];
            }
        }
    }

    // our raw memmove
    #[inline(always)]
    pub fn memmove(src_addr: PhysAddr, dest_addr: PhysAddr, size: usize) {
        unsafe {
            let src = core::slice::from_raw_parts_mut(src_addr.as_usize() as *mut u8, size);
            let dest = core::slice::from_raw_parts_mut(dest_addr.as_usize() as *mut u8, size);

            for i in 0..size {
                dest[i] = src[i];
                src[i] = 0;
            }
        }
    }

    // our raw memmove_aligned
    #[inline(always)]
    pub fn memmove_usize_aligned(src_addr: PhysAddr, dest_addr: PhysAddr, size_in_bytes: usize) {
        debug_assert!(src_addr.as_usize() % MACHINE_UBYTES == 0);
        debug_assert!(dest_addr.as_usize() % MACHINE_UBYTES == 0);

        let size_in_usize = size_in_bytes / MACHINE_UBYTES;
        
        unsafe {
            let src =
                core::slice::from_raw_parts_mut(src_addr.as_usize() as *mut usize, size_in_usize);
            let dest =
                core::slice::from_raw_parts_mut(dest_addr.as_usize() as *mut usize, size_in_usize);

            for i in 0..size_in_bytes {
                dest[i] = src[i];
                src[i] = 0;
            }
        }
    }

    // our function to make an object appear at a specific address with 
    // completely zeroed memory
    #[inline(always)]
    pub fn abracadabra<T>(addr: impl MemAddr + AsUsize) -> *mut T {
        let addr = addr.as_usize();
        let ptr = addr as *mut T;
        unsafe {
            core::ptr::write_volatile(ptr, core::mem::zeroed());
        }
        ptr
    }

    // our function to make an array appear at a specific address with 
    // completely zeroed memory
    #[inline(always)]
    pub fn abracadabra_array<T>(addr: impl MemAddr + AsUsize, count: usize) -> *mut [T] {
        let addr = addr.as_usize();
        let ptr = unsafe { slice::from_raw_parts_mut::<T>(addr as *mut T, count) };
        
        // zero the slice
        raw::memset_size(addr.as_phys(), count * mem::size_of::<T>(), 0);
        ptr
    }

    // address to ref
    #[inline(always)]
    pub fn abracadabra_static_ref<T> (addr: impl MemAddr + From<usize> + AsUsize) -> &'static T {
        unsafe { mem::transmute::<usize, &'static T>(addr.as_usize()) }
    }

    // address to mutable ref - type args are ref type and address type being passed
    #[inline(always)]
    pub fn abracadabra_static_ref_mut<T>(addr: impl MemAddr + From<usize> + AsUsize) -> &'static mut T {
        unsafe { mem::transmute::<usize, &'static mut T>(addr.as_usize()) }
    }

    // address to const pointer
    #[inline(always)]
    pub fn abracadabra_ptr_const<T, AT: MemAddr + From<usize> + AsUsize>(addr: AT) -> *const T {
        unsafe { mem::transmute::<usize, *const T>(addr.as_usize()) }
    }

    // address to mutable pointer
    #[inline(always)]
    pub fn abracadabra_ptr_mut<T, AT: MemAddr + From<usize> + AsUsize>(addr: AT) -> *mut T {
        unsafe { mem::transmute::<usize, *mut T>(addr.as_usize()) }
    }

    // mut ptr to a usize address
    #[inline(always)]
    pub fn ptr_mut_to_usize<T>(reff: *mut T) -> usize {
        unsafe { mem::transmute::<*mut T, usize>(reff) }
    }

    // ptr to a phys address
    #[inline(always)]
    pub fn ptr_to_phys<T>(reff: *const T) -> PhysAddr {
        let addr = unsafe { mem::transmute::<*const T, usize>(reff) };
        PhysAddr::from(addr)
    }

    // ptr to a virt address
    #[inline(always)]
    pub fn ptr_to_virt<T>(reff: *const T) -> VirtAddr {
        let addr = unsafe { mem::transmute::<*const T, usize>(reff) };
        VirtAddr::from(addr)
    }

    // ptr to a usize address
    #[inline(always)]
    pub fn ptr_to_usize<T>(reff: *const T) -> usize {
        unsafe { mem::transmute::<*const T, usize>(reff) }
    }

    // ref to an address
    #[inline(always)]
    pub fn ptr_to_raw<T, AT: MemAddr + From<usize> + AsUsize>(reff: *const T) -> AT {
        let addr = unsafe { mem::transmute::<*const T, usize>(reff) };
        AT::from(addr)
    }

    // ref to an address
    #[inline(always)]
    pub fn ptr_mut_to_raw<T, AT: MemAddr + From<usize> + AsUsize>(reff: *mut T) -> AT {
        let addr = unsafe { mem::transmute::<*mut T, usize>(reff) };
        AT::from(addr)
    }

    // ref to an address
    #[inline(always)]
    pub fn ref_to_raw<T, AT: MemAddr + From<usize> + AsUsize>(reff: &T) -> AT {
        let addr = unsafe { mem::transmute::<&T, usize>(reff) };
        AT::from(addr)
    }

    // ref to an address
    #[inline(always)]
    pub fn ref_mut_to_raw<T, AT: MemAddr + From<usize> + AsUsize>(reff: &mut T) -> AT {
        let addr = unsafe { mem::transmute::<&T, usize>(reff) };
        AT::from(addr)
    }    
}