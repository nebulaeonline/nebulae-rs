#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use crate::arch::x86::cpu::*;

#[cfg(target_arch = "aarch64")]
pub use crate::arch::aa64::cpu::*;
