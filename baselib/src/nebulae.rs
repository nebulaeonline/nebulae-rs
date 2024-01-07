// Essentials
use crate::serial_println;
// Core Rust Items
// External Items
// Internal Items
use crate::common::base::*;
use crate::rng::isaac64::Isaac64Rng;
use crate::frame_alloc::*;
use crate::vmem::*;

#[repr(C)]
pub struct Nebulae<'n> {
    internal_id: usize,
    orig_mem_map_addr: Option<PhysAddr>,
    pub status: HybridLock<KernelStatus>,
    
    conv_pages: usize,
    total_pages: usize,
    phys_mem_boundary: PhysAddr,
    
    pub page_info_structs_01: HybridLock<Option<&'n mut [pages::PageInfoStruct]>>,
    pub krng_03: HybridLock<Option<Isaac64Rng<'n>>>,
    pub frame_alloc_internal_04: HybridLock<Option<TreeAllocator<'n>>>,
    pub frame_alloc_05: HybridLock<bool>,
    pub base_vas_internal_06: HybridLock<bool>,
    pub base_vas_07: HybridLock<Option<Vas>>,
}
impl<'n> Nebulae<'n> {
    
    pub fn new_at_phys_fixed(new_neb_base: PhysAddr, neb_fn: fn(usize) -> usize, neb_fn_seed: usize, conv_pages: usize, total_pages: usize, phys_mem_boundary: PhysAddr, orig_mem_map_addr: PhysAddr) {
        
        let neb = raw::abracadabra_static_ref_mut::<Nebulae>(new_neb_base, false);
        
        #[cfg(all(feature = "serialdbg", debug_assertions))]
        serial_println!("nebulae::new_at() called with new_neb_base = 0x{:08x}, neb = 0x{:08x}, neb_fn = 0x{:p}, conv_pages = {}, total_pages = {}", new_neb_base, neb as *const Nebulae as usize, neb_fn, conv_pages, total_pages);

        #[cfg(all(feature = "serialdbg", debug_assertions))]
        serial_println!("nebulae::new_at() -> phys_mem_boundary = {:#x}, orig_mem_map_addr = {:#x}", phys_mem_boundary, orig_mem_map_addr);

        neb.internal_id = neb_fn(neb_fn_seed);
        neb.status = HybridLock::new(LockType::ExclusiveReadWrite, KernelStatus::new());
        neb.conv_pages = conv_pages;
        neb.total_pages = total_pages;
        neb.phys_mem_boundary = phys_mem_boundary;
        neb.orig_mem_map_addr = Some(orig_mem_map_addr);
        neb.page_info_structs_01 = HybridLock::new(LockType::ExclusiveReadWrite, None);
        neb.krng_03 = HybridLock::new(LockType::ExclusiveReadWrite, None);
        neb.frame_alloc_internal_04 = HybridLock::new(LockType::ExclusiveReadWrite, None);
        neb.frame_alloc_05 = HybridLock::new(LockType::ExclusiveReadWrite, false);
        neb.base_vas_internal_06 = HybridLock::new(LockType::ExclusiveReadWrite, false);
        neb.base_vas_07 = HybridLock::new(LockType::ExclusiveReadWrite, None);
    }

    pub fn get_internal_id(&self) -> usize {
        self.internal_id
    }

    pub fn get_orig_mem_map_addr(&self) -> Option<PhysAddr> {
        self.orig_mem_map_addr
    }

    pub fn get_conv_pages(&self) -> usize {
        self.conv_pages
    }

    pub fn get_total_pages(&self) -> usize {
        self.total_pages
    }

    pub fn get_phys_mem_boundary(&self) -> PhysAddr {
        self.phys_mem_boundary
    }
}

// This is our genesis block function. For children,
// you may use any function that obeys the signature.
pub fn nebulae(let_the_magic_in: usize) -> usize {
    let kernel = let_the_magic_in;
    kernel
}

// Here. Goes. Nothing.
pub fn kernel_main() {
    serial_println!("kernel_main() called with new stack");
}