#![cfg(target_arch = "aarch64")]

use aarch64_cpu::asm;

pub use asm::nop;

/// Pause execution on the core.
#[inline(always)]
pub fn wait_forever() -> ! {
    loop {
        asm::wfe()
    }
}
