use core::arch::asm;

//==========================================================
// UINT32 aarch_popcount(usize value)
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
