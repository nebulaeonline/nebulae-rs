// Rust
use core::fmt;
// Internal
use crate::common::base::*;

// all things memory address related

// phys and virt are kept as distinct types to prevent accidental mixing of the two;
// my preference would be to use traits to cut down on the amount of code duplication,
// but I want to preserve the "similar, but separate" distinction between the two
pub trait MemAddr: AsUsize {
    fn inner(&self) -> usize;

    fn new() -> Self;
    fn new_from(item: Self) -> Self;
    fn new_from_usize(item: usize) -> Self;

    #[cfg(target_arch = "x86")]
    fn get_page_table_indexes(&self) -> (usize, usize) {
        let pd_idx = (self.as_usize() & 0xFFC0_0000) >> 22;
        let pt_idx = (self.as_usize() & 0x0000_03FF) >> 12;

        (pd_idx, pt_idx)
    }

    #[cfg(target_arch = "x86_64")]
    fn get_page_table_indexes(&self) -> (usize, usize, usize, usize) {
        let pml4_idx = (self.as_usize() & 0x7F80_0000_0000) >> 39;
        let pdpt_idx = (self.as_usize() & 0x007F_C000_0000) >> 30;
        let pd_idx   = (self.as_usize() & 0x0000_3FE0_0000) >> 21;
        let pt_idx   = (self.as_usize() & 0x0000_001F_F000) >> 12;

        (pml4_idx, pdpt_idx, pd_idx, pt_idx)
    }
}

pub trait Align: From<usize> + Bitmask + Sized + PartialEq + AsUsize {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
    #[inline(always)]
    fn align_canon_default(&self) -> Self {
        self.bitmask(ALIGN_CANON_4K)
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
    #[inline(always)]
    fn is_aligned_canon_default(&self) -> bool {
        *self == self.bitmask(ALIGN_CANON_4K)
    }
    
    #[inline(always)]
    fn align_canon_4k(&self) -> Self {
        self.bitmask(ALIGN_CANON_4K)
    }

    #[inline(always)]
    fn is_aligned_canon_4k(&self) -> bool {
        *self == self.bitmask(ALIGN_CANON_4K)
    }

    #[inline(always)]
    fn align_4k(&self) -> Self {
        self.bitmask(ALIGN_MASK_4K)
    }

    #[inline(always)]
    fn is_aligned_4k(&self) -> bool {
        self.as_usize() % USIZE_4K == 0
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn align_canon_16k(&self) -> Self {
        self.bitmask(ALIGN_CANON_16K)
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn is_aligned_canon_16k(&self) -> bool {
        *self == self.bitmask(ALIGN_CANON_16K)
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn align_16k(&self) -> Self {
        self.bitmask(ALIGN_MASK_16K)
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn is_aligned_16k(&self) -> bool {
        self.as_usize() % USIZE_16K == 0
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn align_canon_2m(&self) -> Self {
        self.bitmask(ALIGN_CANON_2M)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn is_aligned_canon_2m(&self) -> bool {
        *self == self.bitmask(ALIGN_CANON_2M)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn align_2m(&self) -> Self {
        self.bitmask(ALIGN_MASK_2M)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn is_aligned_2m(&self) -> bool {
        self.as_usize() % USIZE_2M == 0
    }

    #[inline(always)]
    #[cfg(target_arch = "x86")]
    fn align_canon_4m(&self) -> Self {
        self.bitmask(ALIGN_CANON_4M)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86")]
    fn is_aligned_canon_4m(&self) -> bool {
        *self == self.bitmask(ALIGN_CANON_4M)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86")]
    fn align_4m(&self) -> Self {
        self.bitmask(ALIGN_MASK_4M)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86")]
    fn is_aligned_4m(&self) -> bool {
        self.as_usize() % USIZE_4M == 0
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn align_canon_64k(&self) -> Self {
        self.bitmask(ALIGN_CANON_64K)
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn is_aligned_canon_64k(&self) -> bool {
        *self == self.bitmask(ALIGN_CANON_64K)
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn align_64k(&self) -> Self {
        self.bitmask(ALIGN_MASK_64K)
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn is_aligned_64k(&self) -> bool {
        self.as_usize() % USIZE_64K == 0
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn align_canon_1g(&self) -> Self {
        self.bitmask(ALIGN_CANON_1G)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn is_aligned_canon_1g(&self) -> bool {
        *self == self.bitmask(ALIGN_CANON_1G)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn align_1g(&self) -> Self {
        self.bitmask(ALIGN_MASK_1G)
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn is_aligned_1g(&self) -> bool {
        self.as_usize() % USIZE_1G == 0
    }

    #[inline(always)]
    fn is_default_page_aligned(&self) -> bool {
        self.as_usize() % MEMORY_DEFAULT_PAGE_USIZE == 0
    }

    #[inline(always)]
    #[cfg(target_arch = "x86")]
    fn is_page_aligned_greater_than_default(&self) -> bool {
        self.as_usize().is_aligned_4m()
    }

    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    fn is_page_aligned_greater_than_default(&self) -> bool {
        self.as_usize().is_aligned_2m() // if it's 2m aligned, it's 1g aligned
    }

    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    fn is_page_aligned_greater_than_default(&self) -> bool {
        self.as_usize().is_aligned_16k() // if it's 16k aligned, it's 64k aligned
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
}

impl Align for usize {}

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

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct PhysAddr(pub usize);
impl PhysAddr {
    pub const fn is_null(&self) -> bool {
        self.0 == 0 || self.0 == NEBULAE_TEST_PATTERN
    }

    pub const fn as_const_usize(&self) -> usize {
        self.0
    }
    
    pub fn inner_and(&mut self, mask: usize) -> PhysAddr{
        self.0 &= mask;
       *self
    }

    pub fn inner_or(&mut self, mask: usize) -> PhysAddr{
        self.0 |= mask;
        *self
    }

    pub fn inner_xor(&mut self, mask: usize) -> PhysAddr {
        self.0 ^= mask;
        *self
    }

    pub fn inner_bitnot(&mut self) -> PhysAddr {
        self.0 = !self.0;
        *self
    }

    pub fn inner_add(&mut self, x: usize) -> PhysAddr {
        self.0 += x;
        *self
    }

    pub fn inner_sub(&mut self, x: usize) -> PhysAddr{
        self.0 -= x;
        *self
    }

    pub fn inner_inc_by_type<T>(&mut self, n: usize) -> PhysAddr {
        self.0 += core::mem::size_of::<T>() * n;
        *self
    }

    pub fn inner_dec_by_type<T>(&mut self, n: usize) -> PhysAddr {
        self.0 -= core::mem::size_of::<T>() * n;
        *self
    }

    pub fn inner_inc(&mut self, n: usize) -> PhysAddr {
        self.0 += n;
        *self
    }

    pub fn inner_dec(&mut self, n: usize) -> PhysAddr {
        self.0 -= n;
        *self
    }

    pub fn inner_inc_by_page_size(&mut self, page_size: PageSize) -> PhysAddr {
        self.0 += page_size.as_usize();
        *self
    }

    pub fn inner_dec_by_page_size(&mut self, page_size: PageSize) -> PhysAddr {
        self.0 -= page_size.as_usize();
        *self
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
    pub const fn is_null(&self) -> bool {
        self.0 == 0 || self.0 == NEBULAE_TEST_PATTERN
    }

    pub const fn as_const_usize(&self) -> usize {
        self.0
    }
    
    pub fn inner_and(&mut self, mask: usize) -> VirtAddr{
        self.0 &= mask;
       *self
    }

    pub fn inner_or(&mut self, mask: usize) -> VirtAddr{
        self.0 |= mask;
        *self
    }

    pub fn inner_xor(&mut self, mask: usize) -> VirtAddr {
        self.0 ^= mask;
        *self
    }

    pub fn inner_bitnot(&mut self) -> VirtAddr {
        self.0 = !self.0;
        *self
    }

    pub fn inner_add(&mut self, x: usize) -> VirtAddr {
        self.0 += x;
        *self
    }

    pub fn inner_sub(&mut self, x: usize) -> VirtAddr{
        self.0 -= x;
        *self
    }

    pub fn inner_inc_by_type<T>(&mut self, n: usize) -> VirtAddr {
        self.0 += core::mem::size_of::<T>() * n;
        *self
    }

    pub fn inner_dec_by_type<T>(&mut self, n: usize) -> VirtAddr {
        self.0 -= core::mem::size_of::<T>() * n;
        *self
    }

    pub fn inner_inc(&mut self, n: usize) -> VirtAddr {
        self.0 += n;
        *self
    }

    pub fn inner_dec(&mut self, n: usize) -> VirtAddr {
        self.0 -= n;
        *self
    }

    pub fn inner_inc_by_page_size(&mut self, page_size: PageSize) -> VirtAddr {
        self.0 += page_size.as_usize();
        *self
    }

    pub fn inner_dec_by_page_size(&mut self, page_size: PageSize) -> VirtAddr {
        self.0 -= page_size.as_usize();
        *self
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

// -------------------------------------------------------------------

// NOTES: probably not a problem that needed solving
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BytePattern {
    ZeroZero = ZERO_U8,
    Custom(u8),
    FF = u8::MAX,
}
impl BytePattern {
    pub const fn into_bits(self) -> u8 {
        match self {
            BytePattern::ZeroZero => ZERO_U8,
            BytePattern::Custom(x) => x,
            BytePattern::FF => u8::MAX,
        }
    }

    pub const fn from_bits(value: u8) -> Self {
        match value {
            0 => BytePattern::ZeroZero,
            u8::MAX => BytePattern::FF,
            _ => BytePattern::Custom(value),
        }
    }

    pub const fn as_usize_pattern(&self) -> usize {
        let base = self.into_bits() as usize;
        let mut ret = ZERO_USIZE;

        ret |= base;
        ret |= base << 8;
        ret |= base << 16;
        ret |= base << 24;

        if cfg!(target_pointer_width = "64") {
            ret |= base << 32;
            ret |= base << 40;
            ret |= base << 48;
            ret |= base << 56;        
        }
        ret
    }
}
impl const AsUsize for BytePattern {
    fn as_usize(&self) -> usize {
        self.into_bits() as usize
    }
}