use core::mem;

use crate::common::*;

// These are the naughty functions that need to be
// used during scaffolding from the inside, but that
// should be eliminated long-term


// ref to an address
#[inline(always)]
pub fn ref_type_to_addr<T, AT: MemAddr + From<usize>>(reff: &T) -> AT {
    let addr = unsafe { mem::transmute::<&T, usize>(reff) };
    AT::from(addr)
}

// ref to a usize address
#[inline(always)]
pub fn ref_type_to_addr_usize<T>(reff: &T) -> usize {
    unsafe { mem::transmute::<&T, usize>(reff) }
}

// ref to a Uptr address
#[inline(always)]
pub fn ref_type_to_addr_uintn<T>(reff: &T) -> Uintn {
    unsafe { mem::transmute::<&T, Uintn>(reff) }
}

// mut ptr to an address
#[inline(always)]
pub fn ptr_mut_to_addr<T, AT: MemAddr + From<usize>>(reff: *mut T) -> AT {
    let addr = unsafe { mem::transmute::<*mut T, usize>(reff) };
    AT::from(addr)
}

// mut ptr to a usize address
#[inline(always)]
pub fn ptr_mut_to_addr_usize<T>(reff: *mut T) -> usize {
    unsafe { mem::transmute::<*mut T, usize>(reff) }
}

// mut ptr to a Uptr address
#[inline(always)]
pub fn ptr_mut_to_addr_uintn<T>(reff: *mut T) -> Uintn {
    unsafe { mem::transmute::<*mut T, Uintn>(reff) }
}

// ptr to an address
#[inline(always)]
pub fn ptr_to_addr<T, AT: MemAddr + From<usize>>(reff: *const T) -> AT {
    let addr = unsafe { mem::transmute::<*const T, usize>(reff) };
    AT::from(addr)
}

// ptr to a usize address
#[inline(always)]
pub fn ptr_to_addr_usize<T>(reff: *const T) -> usize {
    unsafe { mem::transmute::<*const T, usize>(reff) }
}

// ptr to a Uptr address
#[inline(always)]
pub fn ptr_to_addr_uintn<T>(reff: *const T) -> Uintn {
    unsafe { mem::transmute::<*const T, Uintn>(reff) }
}

// address to ref
#[inline(always)]
pub fn addr_to_ref_type<T, AT: MemAddr + From<usize>>(addr: AT) -> &'static T {
    unsafe { mem::transmute::<usize, &'static T>(addr.as_usize()) }
}

// address to mutable ref
#[inline(always)]
pub fn addr_to_ref_type_mut<T, AT: MemAddr>(addr: AT) -> &'static mut T {
    unsafe { mem::transmute::<usize, &'static mut T>(addr.as_usize()) }
}

// address to const pointer
#[inline(always)]
pub fn addr_to_ptr<T, AT: MemAddr>(addr: AT) -> *const T {
    unsafe { mem::transmute::<usize, *const T>(addr.as_usize()) }
}

// address to mutable pointer
#[inline(always)]
pub fn addr_to_ptr_mut<T, AT: MemAddr>(addr: AT) -> *mut T {
    unsafe { mem::transmute::<usize, *mut T>(addr.as_usize()) }
}

