#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86::interrupts::*;

#[cfg(target_arch = "aarch64")]
pub use crate::arch::aa64::exception::*;
