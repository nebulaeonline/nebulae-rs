// Submodule(s)
pub mod address;
// Rust
use core::slice;
use core::convert::From;
// Internal
use crate::common::base::*;
// Re-exports
pub use crate::memory::address::*;
pub use crate::frame_alloc::*;
pub use crate::vmem::*;

// CONSTANTS
pub const MEMORY_TYPE_UEFI_MEM_MAP: u32 = 0x80805225;

pub const MEMORY_NATURAL_ALIGN: u64 = 2 * MACHINE_BYTES;
pub const MEMORY_NATURAL_UALIGN: usize = MEMORY_NATURAL_ALIGN as usize;

#[derive(Debug)]
pub struct MemBlock<T: MemAddr + Align + Copy + From<usize>> {
    pub base_addr: T,
    pub size: usize,
}
impl<T: MemAddr + Align + Copy + From<usize>> MemBlock<T> {
    pub fn new() -> Self {
        Self {
            base_addr: T::new(),
            size: ZERO_USIZE,
        }
    }

    pub fn new_from(item: Self) -> Self {
        Self {
            base_addr: item.base_addr,
            size: item.size,
        }
    }

    pub fn new_with(base: impl MemAddr, size: usize) -> Self {
        Self {
            base_addr: T::new_from_usize(base.as_usize()),
            size: size,
        }
    }

    pub fn contains_addr(&self, addr: impl MemAddr) -> bool {
        let addr = addr.as_usize();
        let base = self.base_addr.as_usize();
        let size = self.size;

        addr >= base && addr < base + size
    }
}

// Traits (interfaces) for the memory subsystem components
pub trait AddrSpace {
    fn new() -> Self;
    fn init(&mut self, base_page_table_addr: PhysAddr);
    fn switch_to(&mut self);
    fn base_table(&self) -> Option<&BasePageTable>;
    fn base_table_mut(&mut self) -> Option<&mut BasePageTable>;
    fn as_mut(&mut self) -> &mut Self;
    fn as_ref(&self) -> &Self;
}

pub trait FrameAllocator {
    fn new(mem_nodes_base_addr: PhysAddr, node_count: usize) -> Self;
    fn init(&mut self);
    fn alloc_frame(&mut self, size: usize, page_size: PageSize, owner: Owner) -> Option<PhysAddr>;
    fn alloc_frame_fixed(&mut self, phys_addr: PhysAddr, size: usize, page_size: PageSize, owner: Owner) -> Option<PhysAddr>;
    fn dealloc_frame(&mut self, page_base: PhysAddr, owner: Owner) -> bool;
    fn free_page_count(&mut self) -> usize;
    fn free_mem_count(&mut self) -> usize;
    fn total_page_count(&self) -> usize;
    fn total_mem_count(&self) -> usize;
    fn is_memory_frame_free(&self, page_base: PhysAddr) -> bool;
    fn is_frame_index_free(&self, page_idx: usize) -> bool;
}

pub trait PageDir {
    fn new_addr_space(owner: Owner) -> Option<PhysAddr>;
    fn virt_to_phys(&self, vaddr: VirtAddr) -> PhysAddr;
    fn map_page(
        &mut self,
        paddr: PhysAddr,
        vaddr: VirtAddr,
        page_size: PageSize,
        flags: usize,
    ) -> Option<VirtAddr>;
    fn unmap_page(&mut self, vaddr: VirtAddr, owner: Owner, page_size: PageSize) -> bool;
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
        page_size: PageSize,
        flags: usize,
    );
    fn alloc_page_fixed(
        &mut self,
        v: VirtAddr,
        owner: Owner,
        size: PageSize,
        flags: usize,
        bit_pattern: BytePattern,
    ) -> VirtAddr;
    fn alloc_pages_fixed(
        &mut self,
        size_in_pages: usize,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
        bit_pattern: BytePattern,
    ) -> Option<VirtAddr>;
    fn alloc_pages_fixed_virtual(
        &mut self,
        size_in_pages: usize,
        v: VirtAddr,
        owner: Owner,
        page_size: PageSize,
        flags: usize,
        bit_pattern: BytePattern,
    ) -> Option<VirtAddr>;
}

// These are the naughty functions that need to be
// used during scaffolding, but whose use should be 
// reduced / minimized / eliminated in the long-term
pub mod raw {
    use super::*;
    use core::mem;

    // our raw memsets

    #[inline(always)]
    pub fn memset(start_addr: PhysAddr, size: usize, value: u8) {
        unsafe {
            let base = core::slice::from_raw_parts_mut(start_addr.as_usize() as *mut u8, size);

            for i in 0..size {
                base[i] = value;
            }
        }
    }

    #[inline(always)]
    pub fn memset_aligned(start_addr: PhysAddr, size: usize, value: usize) {
        let mut size_in_usize = size / MACHINE_UBYTES;
        if size % MACHINE_UBYTES != 0 {
            size_in_usize += 1;
        }

        unsafe {
            let base =
                core::slice::from_raw_parts_mut(start_addr.as_usize() as *mut usize, size_in_usize);

            for i in 0..size_in_usize {
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

    #[inline(always)]
    pub fn memcpy_aligned(src_addr: PhysAddr, dest_addr: PhysAddr, size: usize) {
        debug_assert!(src_addr.as_usize() % MACHINE_UBYTES == 0);
        debug_assert!(dest_addr.as_usize() % MACHINE_UBYTES == 0);

        let size_in_usize = size / MACHINE_UBYTES;

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

    // our raw memmoves

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

    #[inline(always)]
    pub fn memmove_aligned(src_addr: PhysAddr, dest_addr: PhysAddr, size_in_bytes: usize) {
        debug_assert!(src_addr.as_usize() % MACHINE_UBYTES == 0);
        debug_assert!(dest_addr.as_usize() % MACHINE_UBYTES == 0);

        let mut size_in_usize = size_in_bytes / MACHINE_UBYTES;
        if size_in_bytes % MACHINE_UBYTES != 0 {
            size_in_usize += 1;
        }
        
        unsafe {
            let src =
                core::slice::from_raw_parts_mut(src_addr.as_usize() as *mut usize, size_in_usize);
            let dest =
                core::slice::from_raw_parts_mut(dest_addr.as_usize() as *mut usize, size_in_usize);

            for i in 0..size_in_usize {
                dest[i] = src[i];
                src[i] = 0;
            }
        }
    }

    // our function to make a new object appear at a specific address with 
    // (optional) zeroed memory
    #[inline(always)]
    pub fn abracadabra<T>(addr: impl MemAddr + AsUsize + Align, zero_mem: bool) -> *mut T {
        let ptr = addr.as_usize() as *mut T;
        
        if zero_mem {
            let type_size = mem::size_of::<T>();

            if addr.is_aligned(MACHINE_UBYTES) && type_size.is_aligned(MACHINE_UBYTES) {
                raw::memset_aligned(addr.as_usize().as_phys(), type_size, 0usize);
            } else {
                raw::memset(addr.as_usize().as_phys(), type_size, 0);
            }
        }
        
        ptr
    }

    // our function to make an array appear at a specific address
    #[inline(always)]
    pub fn abracadabra_array<T>(addr: impl MemAddr + AsUsize + Align, count: usize) -> *const [T] {
        let addr = addr.as_usize();
        let ptr = unsafe { slice::from_raw_parts::<T>(addr as *const T, count) };
        ptr
    }

    // our function to make a mutable array appear at a specific address with
    // (optional) zeroed memory
    #[inline(always)]
    pub fn abracadabra_array_mut<T>(addr: impl MemAddr + AsUsize + Align, count: usize, zero_mem: bool) -> *mut [T] {
        let uaddr = addr.as_usize();
        let ptr = unsafe { slice::from_raw_parts_mut::<T>(uaddr as *mut T, count) };
        
        // zero the slice, if requested
        if zero_mem {
            let type_size = mem::size_of::<T>();

            if uaddr.is_aligned(MACHINE_UBYTES) && type_size.is_aligned(MACHINE_UBYTES) {
                raw::memset_aligned(uaddr.as_usize().as_phys(), type_size, 0usize);
            } else {
                raw::memset(uaddr.as_usize().as_phys(), type_size, 0);
            }
        }
        ptr
    }

    // address to ref, optionally zeroing memory
    #[inline(always)]
    pub fn abracadabra_static_ref<T> (addr: impl MemAddr + AsUsize + Align, zero_mem: bool) -> &'static T {
        
        let rref = unsafe { mem::transmute::<usize, &'static T>(addr.as_usize()) };

        if zero_mem {
            let type_size = mem::size_of::<T>();

            if addr.is_aligned(MACHINE_UBYTES) && type_size.is_aligned(MACHINE_UBYTES) {
                raw::memset_aligned(addr.as_usize().as_phys(), type_size, 0usize);
            } else {
                raw::memset(addr.as_usize().as_phys(), type_size, 0);
            }
        }
        rref
    }

    // address to mutable ref - type args are the ref type and the address type being passed
    // optionally zero memory
    #[inline(always)]
    pub fn abracadabra_static_ref_mut<T>(addr: impl MemAddr + AsUsize + Align, zero_mem: bool) -> &'static mut T {
        let mutref = unsafe { mem::transmute::<usize, &'static mut T>(addr.as_usize()) };

        if zero_mem {
            let type_size = mem::size_of::<T>();

            if addr.is_aligned(MACHINE_UBYTES) && type_size.is_aligned(MACHINE_UBYTES) {
                raw::memset_aligned(addr.as_usize().as_phys(), type_size, 0usize);
            } else {
                raw::memset(addr.as_usize().as_phys(), type_size, 0);
            }
        }
        mutref
    }

    // address to const pointer, optionally zeroing memory
    #[inline(always)]
    pub fn abracadabra_ptr_const<T, AT: MemAddr + AsUsize + Align>(addr: AT, zero_mem: bool) -> *const T {
        let ptr = unsafe { mem::transmute::<usize, *const T>(addr.as_usize()) };

        if zero_mem {
            let type_size = mem::size_of::<T>();

            if addr.is_aligned(MACHINE_UBYTES) && type_size.is_aligned(MACHINE_UBYTES) {
                raw::memset_aligned(addr.as_usize().as_phys(), type_size, 0usize);
            } else {
                raw::memset(addr.as_usize().as_phys(), type_size, 0);
            }
        }
        ptr
    }

    // address to mutable pointer
    #[inline(always)]
    pub fn abracadabra_ptr_mut<T, AT: MemAddr + AsUsize + Align>(addr: AT, zero_mem: bool) -> *mut T {
        let mut_ptr = unsafe { mem::transmute::<usize, *mut T>(addr.as_usize()) };

        if zero_mem {
            let type_size = mem::size_of::<T>();

            if addr.is_aligned(MACHINE_UBYTES) && type_size.is_aligned(MACHINE_UBYTES) {
                raw::memset_aligned(addr.as_usize().as_phys(), type_size, 0usize);
            } else {
                raw::memset(addr.as_usize().as_phys(), type_size, 0);
            }
        }
        mut_ptr
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