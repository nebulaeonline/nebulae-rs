use core::arch::asm;

//==========================================================
// VOID aarch_nop()
//==========================================================
#[inline(always)]
pub fn aarch_noop() {
    unsafe {
        asm!("nop", 
        options(nostack, nomem));
    }
}

//==========================================================
// UINTN aarch_popcount(usize value)
//==========================================================
#[inline(always)]
pub fn aarch_popcount(value: usize) -> usize {
    let mut popcount: usize;

    unsafe {
        asm!("cnt {0:w}, {1:x}", 
        in(reg) value,
        lateout(reg) popcount,
        options(nostack, nomem));
    }
    popcount
}
