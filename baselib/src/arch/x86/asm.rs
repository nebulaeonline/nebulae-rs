use core::arch::asm;
#[cfg(target_arch = "x86")]
use core::arch::x86::_rdtsc;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::_rdtsc;
use core::mem;

#[repr(C, packed)]
pub struct DescriptorTablePtr {
    pub limit: u16,
    pub base: usize,
}

//==========================================================
// VOID x86_noop()
//==========================================================
#[inline(always)]
pub fn x86_noop() {
    unsafe {
        asm!(
            "nop",
            options(nostack, nomem),
        );
    }
}

//==========================================================
// VOID x86_outport8(UINT16 Port, UINT8  Value)
//==========================================================
#[inline(always)]
pub fn x86_outport8(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nostack, nomem),
        );
    }
}

//==========================================================
// VOID x86_outport16(UINT16 Port, UINT16 Value)
//==========================================================
#[inline(always)]
pub fn x86_outport16(port: u16, value: u16) {
    unsafe {
        asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") value,
            options(nostack, nomem),
        );
    }
}

//==========================================================
// VOID x86_outport32(UINT16 Port, UINT32 Value)
//==========================================================
#[inline(always)]
pub fn x86_outport32(port: u16, value: u32) {
    unsafe {
        asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
            options(nostack, nomem),
        );
    }
}

//==========================================================
// UINT8 x86_inport8(UINT16 Port)
//==========================================================
#[inline(always)]
pub fn x86_inport8(port: u16) -> u8 {
    let mut inval: u8;

    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            lateout("al") inval,
            options(nostack, nomem),
        );
    }
    inval
}

//==========================================================
// UINT16 x86_inport16(UINT16 Port)
//==========================================================
#[inline(always)]
pub fn x86_inport16(port: u16) -> u16 {
    let mut inval: u16;

    unsafe {
        asm!(
            "in ax, dx",
            in("dx") port,
            lateout("ax") inval,
            options(nostack, nomem),
        );
    }
    inval
}

//==========================================================
// UINT32 x86_inport32(UINT16 Port)
//==========================================================
#[inline(always)]
pub fn x86_inport32(port: u16) -> u32 {
    let mut inval: u32;

    unsafe {
        asm!(
            "in eax, dx",
            in("dx") port,
            lateout("eax") inval,
            options(nostack, nomem),
        );
    }
    inval
}

//==========================================================
// VOID x86_enable_interrupts()
//==========================================================
#[inline(always)]
pub fn x86_enable_interrupts() {
    unsafe {
        asm!("cli", options(nostack, nomem),);
    }
}

//==========================================================
// VOID x86_disable_interrupts()
//==========================================================
#[inline(always)]
pub fn x86_disable_interrupts() {
    unsafe {
        asm!("sti", options(nostack, nomem),);
    }
}

//==========================================================
// UINT64 x86_read_tsc()
//==========================================================
#[inline(always)]
pub fn x86_read_tsc() -> u64 {
    unsafe { _rdtsc() as u64 }
}

//==========================================================
// UINT64 x86_read_tscp()
// from x86 crate
//==========================================================
pub fn x86_rdtscp() -> (u64, u32) {
    let eax: u32;
    let ecx: u32;
    let edx: u32;

    unsafe {
        asm!(
        "rdtscp",
        lateout("eax") eax,
        lateout("ecx") ecx,
        lateout("edx") edx,
        options(nomem, nostack)
        );
    }

    let counter: u64 = (edx as u64) << 32 | eax as u64;
    (counter, ecx)
}

//==========================================================
// usize x86_read_raw_cr3()
//==========================================================
#[inline(always)]
pub fn x86_read_raw_cr3() -> usize {
    let mut cr3: usize;

    unsafe {
        asm!(
            "mov {0}, cr3",
            lateout(reg) cr3,
            options(nostack, nomem),
        );
    }
    cr3
}

//==========================================================
// usize x86_write_cr3 (usize new_cr3)
//==========================================================
#[inline(always)]
pub fn x86_write_cr3(cr3: usize) -> usize {
    let mut new_cr3: usize = cr3;

    unsafe {
        asm!(
            "mov cr3, {0}",
            inlateout(reg) new_cr3,
            options(nostack, nomem),
        );
    }
    new_cr3
}

//==========================================================
// VOID x86_read_gdtr (OUT SegmentSelector *gdtr)
//==========================================================
#[inline(always)]
pub fn x86_read_gdtr(gdtr_out: &mut DescriptorTablePtr) {
    unsafe {
        asm!(
            "sgdt [{0}]",
            in(reg) gdtr_out,
            options(nostack),
        );
    }
}

//==========================================================
// VOID x86_write_gdtr (IN CONST x64_seg_sel *gdtr);
//==========================================================
#[inline(always)]
pub fn x86_write_gdtr(new_gdt: &DescriptorTablePtr) {
    unsafe {
        asm!(
            "lgdt [{0}]",
            in(reg) new_gdt,
            options(nostack),
        );
    }
}

//==========================================================
// VOID x86_read_idtr (OUT x64_seg_sel *idtr)
//==========================================================
#[inline(always)]
pub fn x86_read_idtr(idtr_out: &mut DescriptorTablePtr) {
    unsafe {
        asm!(
            "sidt [{0}]",
            in(reg) idtr_out,
            options(nostack),
        );
    }
}

//==========================================================
// VOID x86_write_idtr (IN CONST x64_seg_sel *idtr)
//==========================================================
#[inline(always)]
pub fn x86_write_idtr(new_idt: &DescriptorTablePtr) {
    unsafe {
        asm!(
            "lidt [{0}]",
            in(reg) new_idt,
            options(nostack),
        );
    }
}

//==========================================================
// UINT64 x86_read_cr2 ()
//==========================================================
#[inline(always)]
pub fn x86_read_cr2() -> usize {
    let mut out: usize;

    unsafe {
        asm!(
            "mov {0}, cr2",
            lateout(reg) out,
            options(nostack, nomem),
        );
    }
    out
}

//==========================================================
// UINT16 x86_read_cs()
//==========================================================
#[inline(always)]
pub fn x86_read_cs() -> u16 {
    let mut out: u16;

    unsafe {
        asm!(
            "mov {0:x}, cs",
            lateout(reg_abcd) out,
            options(nostack, nomem),
        );
    }
    out
}

//==========================================================
// UINT16 x86_read_ds()
//==========================================================
#[inline(always)]
pub fn x86_read_ds() -> u16 {
    let mut out: u16;

    unsafe {
        asm!(
            "mov {0:x}, ds",
            lateout(reg_abcd) out,
            options(nostack, nomem),
        );
    }
    out
}

//==========================================================
// UINT16 x86_read_ss()
//==========================================================
#[inline(always)]
pub fn x86_read_ss() -> u16 {
    let mut out: u16;

    unsafe {
        asm!(
            "mov {0:x}, ss",
            lateout(reg_abcd) out,
            options(nostack, nomem),
        );
    }
    out
}

//==========================================================
// UINT16 x86_read_es()
//==========================================================
#[inline(always)]
pub fn x86_read_es() -> u16 {
    let mut out: u16;

    unsafe {
        asm!(
            "mov {0:x}, es",
            lateout(reg_abcd) out,
            options(nostack, nomem),
        );
    }
    out
}

//==========================================================
// UINT16 x86_read_fs()
//==========================================================
#[inline(always)]
pub fn x86_read_fs() -> u16 {
    let mut out: u16;

    unsafe {
        asm!(
            "mov {0:x}, fs",
            lateout(reg_abcd) out,
            options(nostack, nomem),
        );
    }
    out
}

//==========================================================
// UINT16 x86_read_gs()
//==========================================================
#[inline(always)]
pub fn x86_read_gs() -> u16 {
    let mut out: u16;

    unsafe {
        asm!(
            "mov {0:x}, gs",
            lateout(reg_abcd) out,
            options(nostack, nomem),
        );
    }
    out
}

//==========================================================
// VOID x86_load_tr (UINT16 tr_gdt_descr_index)
//==========================================================
#[inline(always)]
pub fn x86_load_tr(gdt_descr_idx: u16) {
    unsafe {
        asm!(
            "ltr {0:x}",
            in(reg_abcd) gdt_descr_idx,
            options(nostack, nomem),
        );
    }
}

//==========================================================
// VOID x86_invalidate_page(EFI_PHYSICAL_ADDRESS *addr)
//==========================================================
#[inline(always)]
pub fn x86_invalidate_page(mem_addr: usize) {
    unsafe {
        asm!(
            "invlpg [{0}]",
            in(reg) mem_addr,
            options(nostack),
        );
    }
}

//==========================================================
// VOID x86_atomic_or(usize *value, usize mask)
//==========================================================
#[inline(always)]
pub fn x86_atomic_or(value: &mut usize, mask: usize) {
    unsafe {
        if cfg!(target_arch = "x86") {
            asm!(
                "lock or dword [{0}], {1}",
                in(reg) value,
                in(reg) mask,
                options(nostack),
            );
        } else {
            asm!(
                "lock or qword [{0}], {1}",
                in(reg) value,
                in(reg) mask,
                options(nostack),
            );
        }
    }
}

//==========================================================
// VOID x86_atomic_and(usize *value, usize mask);
//==========================================================
#[inline(always)]
pub fn x86_atomic_and(value: &mut usize, mask: usize) {
    unsafe {
        if cfg!(target_arch = "x86") {
            asm!(
                "lock and dword [{0}], {1}",
                in(reg) value,
                in(reg) mask,
                options(nostack),
            );
        } else {
            asm!(
                "lock and qword [{0}], {1}",
                in(reg) value,
                in(reg) mask,
                options(nostack),
            );
        }
    }
}

//==========================================================
// VOID x86_atomic_xor(usize *value, usize mask);
//==========================================================
#[inline(always)]
pub fn x86_atomic_xor(value: &mut usize, mask: usize) {
    unsafe {
        if cfg!(target_arch = "x86") {
            asm!(
                "lock xor dword [{0}], {1}",
                in(reg) value,
                in(reg) mask,
                options(nostack),
            );
        } else {
            asm!(
                "lock xor qword [{0}], {1}",
                in(reg) value,
                in(reg) mask,
                options(nostack),
            );
        }
    }
}

//==========================================================
// VOID x86_atomic_add(usize *value, usize addend);
//==========================================================
#[inline(always)]
pub fn x86_atomic_add(value: &mut usize, addend: usize) {
    unsafe {
        if cfg!(target_arch = "x86") {
            asm!(
                "lock add dword [{0}], {1}",
                in(reg) value,
                in(reg) addend,
                options(nostack),
            );
        } else {
            asm!(
                "lock add qword [{0}], {1}",
                in(reg) value,
                in(reg) addend,
                options(nostack),
            );
        }
    }
}

//==========================================================
// VOID x86_atomic_sub(usize *value, usize subtrahend);
//==========================================================
#[inline(always)]
pub fn x86_atomic_sub(value: &mut usize, subtrahend: usize) {
    unsafe {
        if cfg!(target_arch = "x86") {
            asm!(
                "lock sub dword [{0}], {1}",
                in(reg) value,
                in(reg) subtrahend,
                options(nostack),
            );
        } else {
            asm!(
                "lock sub qword [{0}], {1}",
                in(reg) value,
                in(reg) subtrahend,
                options(nostack),
            );
        }
    }
}

//==========================================================
// VOID x86_atomic_inc(usize *value)
//==========================================================
#[inline(always)]
pub fn x86_atomic_inc(value: &mut usize) {
    unsafe {
        if cfg!(target_arch = "x86") {
            asm!(
                "lock inc dword [{0}]",
                in(reg) value,
                options(nostack),
            );
        } else {
            asm!(
                "lock inc qword [{0}]",
                in(reg) value,
                options(nostack),
            );
        }
    }
}

//==========================================================
// VOID x86_atomic_dec(usize *value)
//==========================================================
#[inline(always)]
pub fn x86_atomic_dec(value: &mut usize) {
    unsafe {
        if cfg!(target_arch = "x86") {
            asm!(
                "lock dec dword [{0}]",
                in(reg) value,
                options(nostack),
            );
        } else {
            asm!(
                "lock dec qword [{0}]",
                in(reg) value,
                options(nostack),
            );
        }
    }
}

//==========================================================
// UINT8 x86_atomic_dec_and_test_zero(usize *value)
//==========================================================
#[inline(always)]
pub fn x86_atomic_dec_and_test_zero(value: &mut usize) -> bool {
    let mut result: u8;

    if cfg!(target_arch = "x86") {
        unsafe {
            asm!(
                "lock dec dword [{0}]",
                "setz {1}",
                in(reg) value,
                lateout(reg_byte) result,
                options(nostack),
            );
        }
    } else {
        unsafe {
            asm!(
                "lock dec qword [{0}]",
                "setz {1}",
                in(reg) value,
                lateout(reg_byte) result,
                options(nostack),
            );
        }
    }

    if result > 0 {
        true
    } else {
        false
    }
}

//==========================================================
// VOID x86_atomic_mov(usize *dest, usize value)
//==========================================================
#[inline(always)]
pub fn x86_atomic_mov(dest: &mut usize, value: usize) {
    unsafe {
        if cfg!(target_arch = "x86") {
            asm!(
                "lock mov dword [{0}], {1}",
                in(reg) dest,
                in(reg) value,
                options(nostack),
            );
        } else {
            asm!(
                "lock mov qword [{0}], {1}",
                in(reg) dest,
                in(reg) value,
                options(nostack),
            );
        }
    }
}

//==========================================================
// VOID x86_spinlock_acquire(usize *spinlock)
//==========================================================
#[inline(always)]
pub fn x86_spinlock_acquire(spinlock: &mut usize) {
    unsafe {
        if cfg!(target_arch = "x86") {
            asm!(
                "2:",
                "cmp dword [{0}], 0",
                "je 3f",
                "pause",
                "jmp 2b",
                "3:",
                "mov dword {1}, 1",
                "xchg [{0}], {1}",
                "cmp dword {1}, 0",
                "jne 2b",
                in(reg) spinlock,
                out(reg) _,
                options(nostack),
            );
        } else {
            asm!(
                "2:",
                "cmp qword [{0}], 0",
                "je 3f",
                "pause",
                "jmp 2b",
                "3:",
                "mov qword {1}, 1",
                "xchg [{0}], {1}",
                "cmp qword {1}, 0",
                "jne 2b",
                in(reg) spinlock,
                out(reg) _,
                options(nostack),
            );
        }
    }
}

//==========================================================
// VOID x86_spinlock_release(usize *spinlock)
//==========================================================
#[inline(always)]
pub fn x86_spinlock_release(spinlock: &mut usize) {
    unsafe {
        if cfg!(target_arch = "x86") {
            asm!(
                "mov dword {1}, 0",
                "xchg [{0}], {1}",
                in(reg) spinlock,
                out(reg) _,
                options(nostack),
            );
        } else {
            asm!(
                "mov qword {1}, 0",
                "xchg [{0}], {1}",
                in(reg) spinlock,
                out(reg) _,
                options(nostack),
            );
        }
    }
}

//==========================================================
// cpuid functions
//==========================================================
pub struct CpuidRegs {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}
impl CpuidRegs {
    pub fn new() -> CpuidRegs {
        CpuidRegs {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
        }
    }
}

// Executes the cpuid instruction, passing mode in eax
#[cfg(target_arch = "x86")]
#[inline(always)]
pub fn x86_cpuid(mode: u32) -> CpuidRegs {
    let mut regs = CpuidRegs::new();

    unsafe {
        asm!(
            "push ebx",
            "cpuid",
            "pop ebx",
            inout("eax") mode => regs.eax,
            lateout("ecx") regs.ecx,
            lateout("edx") regs.edx,
            options(nomem),
        );
        asm!(
            "push ebx",
            "cpuid",
            "mov ecx, ebx",
            "pop ebx",
            in("eax") regs.eax,
            lateout("ecx") regs.ebx,
            options(nomem),
        );
    }
    regs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn x86_cpuid(mode: u32) -> CpuidRegs {
    let mut regs = CpuidRegs::new();

    unsafe {
        asm!(
            "push rbx",
            "cpuid",
            "mov r9, rbx",
            "pop rbx",
            inout("eax") mode => regs.eax,
            lateout("r9") regs.ebx,
            lateout("ecx") regs.ecx,
            lateout("edx") regs.edx,
            options(nomem),
        );
    }
    regs
}

// Executes the cpuid instruction, passing mode in eax, and
// an additional value in ecx for extended instructions
#[cfg(target_arch = "x86")]
#[inline(always)]
pub fn x86_cpuid_ext(mode: u32, param: u32) -> CpuidRegs {
    let mut regs = CpuidRegs {
        eax: 0,
        ebx: 0,
        ecx: 0,
        edx: 0,
    };

    unsafe {
        asm!(
            "push ebx",
            "cpuid",
            "pop ebx",
            inout("eax") mode => regs.eax,
            inout("ecx") param => regs.ecx,
            lateout("edx") regs.edx,
            options(nomem),
        );
        asm!(
            "push ebx",
            "cpuid",
            "mov ecx, ebx",
            "pop ebx",
            in("eax") mode,
            inout("ecx") param => regs.ebx,
            options(nomem),
        );
    }
    regs
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn x86_cpuid_ext(mode: u32, param: u32) -> CpuidRegs {
    let mut regs = CpuidRegs {
        eax: 0,
        ebx: 0,
        ecx: 0,
        edx: 0,
    };

    unsafe {
        asm!(
            "push rbx",
            "cpuid",
            "mov r9, rbx",
            "pop rbx",
            inout("eax") mode => regs.eax,
            inout("ecx") param => regs.ecx,
            lateout("r9") regs.ebx,
            lateout("edx") regs.edx,
            options(nomem),
        );
    }
    regs
}

//==========================================================
// VOID x86_mfence()
//==========================================================
#[inline(always)]
pub fn x86_mfence() {
    unsafe {
        asm!("mfence", options(nostack, nomem));
    }
}

//==========================================================
// VOID x86_sfence()
//==========================================================
#[inline(always)]
pub fn x86_sfence() {
    unsafe {
        asm!("sfence", options(nostack, nomem));
    }
}

//==========================================================
// VOID x86_lfence()
//==========================================================
#[inline(always)]
pub fn x86_lfence() {
    unsafe {
        asm!("lfence", options(nostack, nomem));
    }
}

//==========================================================
// VOID x86_popcount(usize value)
//==========================================================
#[inline(always)]
pub fn x86_popcount(value: usize) -> usize {
    let mut popcount: usize;

    unsafe {
        asm!("popcnt eax, ecx", 
        in("ecx") value,
        lateout("eax") popcount,
        options(nostack, nomem));
    }
    popcount
}

//==========================================================
// Bit Scan Forward
// usize x86_bsf(usize value)
// taken from https://docs.oracle.com/cd/E19455-01/806-3773/instructionset-89/index.html
//==========================================================
#[inline(always)]
pub fn x86_bsf(input: usize) -> usize {
    debug_assert!(input > 0);
    let mut pos: usize;
    unsafe {
        asm! {
            "bsf {pos}, {input}",
            input = in(reg) input,
            pos = out(reg) pos,
            options(nomem, nostack),
        };
    };
    debug_assert!(pos < mem::size_of::<usize>());
    pos
}

//==========================================================
// Bit Scan Reverse
// usize x86_bsr(usize value)
// taken from https://docs.oracle.com/cd/E19620-01/805-4693/instructionset-90/index.html
//==========================================================
#[inline(always)]
pub fn x86_bsr(input: usize) -> usize {
    debug_assert!(input > 0);
    let mut pos: usize;
    unsafe {
        asm! {
            "bsr {pos}, {input}",
            input = in(reg) input,
            pos = out(reg) pos,
            options(nomem, nostack),
        };
    };
    debug_assert!(pos < mem::size_of::<usize>());
    pos
}

//==========================================================
// VOID x86_halt(VOID)
//==========================================================
#[inline(always)]
pub fn x86_halt() {
    unsafe {
        asm! {
            "hlt",
            options(nomem, nostack),
        }
    }
}
