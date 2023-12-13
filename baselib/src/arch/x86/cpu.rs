#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(dead_code)] // TODO: Remove when in use

use crate::bit::*;
use crate::constants::*;
use crate::arch::x86::cache_descriptor::*;
use crate::arch::x86::asm::*;

use bitfield_struct::*;

#[inline(always)]
pub fn wait_forever() -> ! {
    loop {
        x86_halt();
    }
}

#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct CpuFeatures {
    #[bits(1)]
    pub feat_hypervisor_present: bool,
    #[bits(1)]
    pub feat_rdrand: bool,
    #[bits(1)]
    pub feat_f16c: bool,
    #[bits(1)]
    pub feat_avx: bool,
    #[bits(1)]
    pub feat_osxsave: bool,
    #[bits(1)]
    pub feat_xsave: bool,
    #[bits(1)]
    pub feat_aes: bool,
    #[bits(1)]
    pub feat_tsc_deadline: bool,
    #[bits(1)]
    pub feat_popcnt: bool,
    #[bits(1)]
    pub feat_movbe: bool,
    #[bits(1)]
    pub feat_x2apic: bool,
    #[bits(1)]
    pub feat_sse42: bool,
    #[bits(1)]
    pub feat_sse41: bool,
    #[bits(1)]
    pub feat_dca: bool,
    #[bits(1)]
    pub feat_pcid: bool,
    #[bits(1)]
    pub feat_pdcm: bool,
    #[bits(1)]
    pub feat_etprd: bool,
    #[bits(1)]
    pub feat_cx16: bool,
    #[bits(1)]
    pub feat_fma: bool,
    #[bits(1)]
    pub feat_sdbg: bool,
    #[bits(1)]
    pub feat_cid: bool,
    #[bits(1)]
    pub feat_ssse3: bool,
    #[bits(1)]
    pub feat_tm2: bool,
    #[bits(1)]
    pub feat_est: bool,
    #[bits(1)]
    pub feat_smx: bool,
    #[bits(1)]
    pub feat_vmx: bool,
    #[bits(1)]
    pub feat_dscpl: bool,
    #[bits(1)]
    pub feat_monitor: bool,
    #[bits(1)]
    pub feat_dtes64: bool,
    #[bits(1)]
    pub feat_pclmul: bool,
    #[bits(1)]
    pub feat_sse3: bool,
    #[bits(1)]
    pub feat_pbe: bool,
    #[bits(1)]
    pub feat_tm1: bool,
    #[bits(1)]
    pub feat_hyperthreading: bool,
    #[bits(1)]
    pub feat_selfsnoop: bool,
    #[bits(1)]
    pub feat_sse2: bool,
    #[bits(1)]
    pub feat_sse: bool,
    #[bits(1)]
    pub feat_fxsr: bool,
    #[bits(1)]
    pub feat_mmx: bool,
    #[bits(1)]
    pub feat_acpi_therm: bool,
    #[bits(1)]
    pub feat_dtes: bool,
    #[bits(1)]
    pub feat_clfl: bool,
    #[bits(1)]
    pub feat_psn: bool,
    #[bits(1)]
    pub feat_pse36: bool,
    #[bits(1)]
    pub feat_pat: bool,
    #[bits(1)]
    pub feat_cmov: bool,
    #[bits(1)]
    pub feat_mca: bool,
    #[bits(1)]
    pub feat_pge: bool,
    #[bits(1)]
    pub feat_mtrr: bool,
    #[bits(1)]
    pub feat_sysenter: bool,
    #[bits(1)]
    pub feat_apic: bool,
    #[bits(1)]
    pub feat_cx8: bool,
    #[bits(1)]
    pub feat_mce: bool,
    #[bits(1)]
    pub feat_pae: bool,
    #[bits(1)]
    pub feat_msr: bool,
    #[bits(1)]
    pub feat_tsc: bool,
    #[bits(1)]
    pub feat_pse: bool,
    #[bits(1)]
    pub feat_de: bool,
    #[bits(1)]
    pub feat_vme: bool,
    #[bits(1)]
    pub feat_fpu: bool,
    #[bits(4)]
    reserved: u8,
}

#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct CpuFeaturesExt {
    // Double-super-extended features
    #[bits(1)]
    pub feat_avx512vl: bool,
    #[bits(1)]
    pub feat_avx512bw: bool,
    #[bits(1)]
    pub feat_sha: bool,
    #[bits(1)]
    pub feat_avx512cd: bool,
    #[bits(1)]
    pub feat_avx512er: bool,
    #[bits(1)]
    pub feat_avx512pf: bool,
    #[bits(1)]
    pub feat_processor_trace: bool,
    #[bits(1)]
    pub feat_clwb: bool,
    #[bits(1)]
    pub feat_clflushopt: bool,
    #[bits(1)]
    pub feat_pcommit: bool,
    #[bits(1)]
    pub feat_avx512ifma: bool,
    #[bits(1)]
    pub feat_smap: bool,
    #[bits(1)]
    pub feat_adx: bool,
    #[bits(1)]
    pub feat_rdseed: bool,
    #[bits(1)]
    pub feat_avx512dq: bool,
    #[bits(1)]
    pub feat_avx512f: bool,
    #[bits(1)]
    pub feat_pqe: bool,
    #[bits(1)]
    pub feat_mpx: bool,
    #[bits(1)]
    pub feat_fpcsds: bool,
    #[bits(1)]
    pub feat_pqm: bool,
    #[bits(1)]
    pub feat_rtm: bool,
    #[bits(1)]
    pub feat_invpcid: bool,
    #[bits(1)]
    pub feat_erms: bool,
    #[bits(1)]
    pub feat_bmi2: bool,
    #[bits(1)]
    pub feat_smep: bool,
    #[bits(1)]
    pub feat_fpdp: bool,
    #[bits(1)]
    pub feat_avx2: bool,
    #[bits(1)]
    pub feat_hle: bool,
    #[bits(1)]
    pub feat_bmi1: bool,
    #[bits(1)]
    pub feat_sgx: bool,
    #[bits(1)]
    pub feat_tsc_adjust: bool,
    #[bits(1)]
    pub feat_fsgsbase: bool,
    #[bits(1)]
    pub feat_sgx_lc: bool,
    #[bits(1)]
    pub feat_rdpid: bool,
    #[bits(1)]
    pub feat_va57: bool,
    #[bits(1)]
    pub feat_avx512vp_dq: bool,
    #[bits(1)]
    pub feat_tme: bool,
    #[bits(1)]
    pub feat_avx512bitalg: bool,
    #[bits(1)]
    pub feat_avx512vnni: bool,
    #[bits(1)]
    pub feat_vpcl: bool,
    #[bits(1)]
    pub feat_vaes: bool,
    #[bits(1)]
    pub feat_gfni: bool,
    #[bits(1)]
    pub feat_cet: bool,
    #[bits(1)]
    pub feat_avx512vbmi2: bool,
    #[bits(1)]
    pub feat_ospke: bool,
    #[bits(1)]
    pub feat_pku: bool,
    #[bits(1)]
    pub feat_umip: bool,
    #[bits(1)]
    pub feat_avx512vbmi: bool,
    #[bits(1)]
    pub feat_prefetchwt1: bool,
    #[bits(1)]
    pub feat_arch_cap_msr: bool,
    #[bits(1)]
    pub feat_stibp: bool,
    #[bits(1)]
    pub feat_ibrs_mbpb: bool,
    #[bits(1)]
    pub feat_pconfig: bool,
    #[bits(1)]
    pub feat_avx512qfma: bool,
    #[bits(1)]
    pub feat_avx512qvnniw: bool,
    #[bits(9)]
    reserved: u16,
}

#[derive(Debug)]
pub struct CpuInfo {
    pub vendor_id: [u8; 12],
    pub max_cpuid_level: u32,
    pub extended_model: u8,
    pub extended_family: u8,
    pub processor_type: u8,
    pub processor_family: u8,
    pub processor_model: u8,
    pub processor_stepping: u8,
    pub processor_brandid: u8,
    pub clflush_chunk_count: u8,
    pub cpu_count: u16,
    pub default_apic_id: u8,
    pub cache_descriptors: [CacheDescriptor; 15],
    pub cache_descriptor_count: u8,
    pub mode4_cache_info: bool,
    pub mode4_tlb_info: bool,
    pub features: CpuFeatures,
    pub features_ext: CpuFeaturesExt,
}

#[allow(dead_code)] // TODO: Remove when in use
#[derive(Debug)]
pub struct Cpu {
    pub id: u64,
    pub cohort_id: u64,
    pub physical: bool,
    pub info: CpuInfo,    
}

#[allow(dead_code)] // TODO: Remove when in use
impl Cpu {
    pub fn new() -> Cpu {
        let info = CpuInfo { 
            vendor_id: [0;12], 
            max_cpuid_level: 0,
            extended_model: 0,
            extended_family: 0,
            processor_type: 0,
            processor_family: 0,
            processor_model: 0,
            processor_stepping: 0,
            processor_brandid: 0,
            clflush_chunk_count: 0,
            cpu_count: 0,
            default_apic_id: 0,
            cache_descriptors: [CacheDescriptor {
                level: CacheLevel::Unknown,
                type_of_cache: CacheType::Unknown,
                size: CacheSize::Unknown,
                associativity: CacheAssociativity::Unknown,
                layout: CacheLayoutType::Unknown,
                count: 0,
                ecc: false,
                sectored: CacheSectored::No,
            }; 15],
            cache_descriptor_count: 0,
            mode4_cache_info: false,
            mode4_tlb_info: false,
            features: CpuFeatures::new() 
                .with_feat_hypervisor_present(false)
                .with_feat_rdrand(false)
                .with_feat_f16c(false)
                .with_feat_avx(false)
                .with_feat_osxsave(false)
                .with_feat_xsave(false)
                .with_feat_aes(false)
                .with_feat_tsc_deadline(false)
                .with_feat_popcnt(false)
                .with_feat_movbe(false)
                .with_feat_x2apic(false)
                .with_feat_sse42(false)
                .with_feat_sse41(false)
                .with_feat_dca(false)
                .with_feat_pcid(false)
                .with_feat_pdcm(false)
                .with_feat_etprd(false)
                .with_feat_cx16(false)
                .with_feat_fma(false)
                .with_feat_sdbg(false)
                .with_feat_cid(false)
                .with_feat_ssse3(false)
                .with_feat_tm2(false)
                .with_feat_est(false)
                .with_feat_smx(false)
                .with_feat_vmx(false)
                .with_feat_dscpl(false)
                .with_feat_monitor(false)
                .with_feat_dtes64(false)
                .with_feat_pclmul(false)
                .with_feat_sse3(false)
                .with_feat_pbe(false)
                .with_feat_tm1(false)
                .with_feat_hyperthreading(false)
                .with_feat_selfsnoop(false)
                .with_feat_sse2(false)
                .with_feat_sse(false)
                .with_feat_fxsr(false)
                .with_feat_mmx(false)
                .with_feat_acpi_therm(false)
                .with_feat_dtes(false)
                .with_feat_clfl(false)
                .with_feat_psn(false)
                .with_feat_pse36(false)
                .with_feat_pat(false)
                .with_feat_cmov(false)
                .with_feat_mca(false)
                .with_feat_pge(false)
                .with_feat_mtrr(false)
                .with_feat_sysenter(false)
                .with_feat_apic(false)
                .with_feat_cx8(false)
                .with_feat_mce(false)
                .with_feat_pae(false)
                .with_feat_msr(false)
                .with_feat_tsc(false)
                .with_feat_pse(false)
                .with_feat_de(false)
                .with_feat_vme(false)
                .with_feat_fpu(false),
            features_ext: CpuFeaturesExt::new()
                .with_feat_avx512vl(false)
                .with_feat_avx512bw(false)
                .with_feat_sha(false)
                .with_feat_avx512cd(false)
                .with_feat_avx512er(false)
                .with_feat_avx512pf(false)
                .with_feat_processor_trace(false)
                .with_feat_clwb(false)
                .with_feat_clflushopt(false)
                .with_feat_pcommit(false)
                .with_feat_avx512ifma(false)
                .with_feat_smap(false)
                .with_feat_adx(false)
                .with_feat_rdseed(false)
                .with_feat_avx512dq(false)
                .with_feat_avx512f(false)
                .with_feat_pqe(false)
                .with_feat_mpx(false)
                .with_feat_fpcsds(false)
                .with_feat_pqm(false)
                .with_feat_rtm(false)
                .with_feat_invpcid(false)
                .with_feat_erms(false)
                .with_feat_bmi2(false)
                .with_feat_smep(false)
                .with_feat_fpdp(false)
                .with_feat_avx2(false)
                .with_feat_hle(false)
                .with_feat_bmi1(false)
                .with_feat_sgx(false)
                .with_feat_tsc_adjust(false)
                .with_feat_fsgsbase(false)
                .with_feat_sgx_lc(false)
                .with_feat_rdpid(false)
                .with_feat_va57(false)
                .with_feat_avx512vp_dq(false)
                .with_feat_tme(false)
                .with_feat_avx512bitalg(false)
                .with_feat_avx512vnni(false)
                .with_feat_vpcl(false)
                .with_feat_vaes(false)
                .with_feat_gfni(false)
                .with_feat_cet(false)
                .with_feat_avx512vbmi2(false)
                .with_feat_ospke(false)
                .with_feat_pku(false)
                .with_feat_umip(false)
                .with_feat_avx512vbmi(false)
                .with_feat_prefetchwt1(false)
                .with_feat_arch_cap_msr(false)
                .with_feat_stibp(false)
                .with_feat_ibrs_mbpb(false)
                .with_feat_pconfig(false)
                .with_feat_avx512qfma(false)
                .with_feat_avx512qvnniw(false),
        };

        let mut cpu = Cpu { 
            id: 0, 
            cohort_id: 0, 
            physical: true,
            info: info,
        };

        // Identify this cpu
        let regs = x86_cpuid(0);

        // Figure out how far we can go
        cpu.info.max_cpuid_level = regs.eax;

        // Go
        cpu.info.vendor_id[0] = ((regs.ebx & BYTE3_U32) >> 24) as u8;
        cpu.info.vendor_id[1] = ((regs.ebx & BYTE2_U32) >> 16) as u8;
        cpu.info.vendor_id[2] = ((regs.ebx & BYTE1_U32) >> 8) as u8;
        cpu.info.vendor_id[3] = (regs.ebx & BYTE0_U32) as u8;
        cpu.info.vendor_id[4] = ((regs.edx & BYTE3_U32) >> 24) as u8;
        cpu.info.vendor_id[5] = ((regs.edx & BYTE2_U32) >> 16) as u8;
        cpu.info.vendor_id[6] = ((regs.edx & BYTE1_U32) >> 8) as u8;
        cpu.info.vendor_id[7] = (regs.edx & BYTE0_U32) as u8;
        cpu.info.vendor_id[8] = ((regs.ecx & BYTE3_U32) >> 24) as u8;
        cpu.info.vendor_id[9] = ((regs.ecx & BYTE2_U32) >> 16) as u8;
        cpu.info.vendor_id[10] = ((regs.ecx & BYTE1_U32) >> 8) as u8;
        cpu.info.vendor_id[11] = (regs.ecx & BYTE0_U32) as u8;

        // CPUID / EAX == 1
        if cpu.info.max_cpuid_level >= 1 {
            let regs = x86_cpuid(1);

            cpu.info.extended_family = ((regs.eax & BYTE3_U32) >> 20) as u8;
            cpu.info.extended_model = ((regs.eax & BYTE2_U32) >> 16) as u8;
            cpu.info.processor_type = ((regs.eax & 0x3000) >> 12) as u8;
            cpu.info.processor_family = ((regs.eax & 0xF00) >> 8) as u8;
            cpu.info.processor_model = ((regs.eax & 0xF0) >> 4) as u8;
            cpu.info.processor_stepping = (regs.eax & 0x0F) as u8;
            cpu.info.processor_brandid = (regs.ebx & BYTE0_U32) as u8;
            cpu.info.clflush_chunk_count = ((regs.ebx & BYTE1_U32) >> 8) as u8;
            cpu.info.cpu_count = ((regs.ebx & BYTE2_U32) >> 16) as u16;
            cpu.info.default_apic_id = ((regs.ebx & BYTE3_U32) >> 24) as u8;

            // Individual feature flags (ECX)
            cpu.info.features.set_feat_hypervisor_present(is_bit_set32(regs.ecx, 31));
            cpu.info.features.set_feat_rdrand(is_bit_set32(regs.ecx, 30));
            cpu.info.features.set_feat_f16c(is_bit_set32(regs.ecx, 29));
            cpu.info.features.set_feat_avx(is_bit_set32(regs.ecx, 28));
            cpu.info.features.set_feat_osxsave(is_bit_set32(regs.ecx, 27));
            cpu.info.features.set_feat_xsave(is_bit_set32(regs.ecx, 26));
            cpu.info.features.set_feat_aes(is_bit_set32(regs.ecx, 25));
            cpu.info.features.set_feat_tsc_deadline(is_bit_set32(regs.ecx, 24));
            cpu.info.features.set_feat_popcnt(is_bit_set32(regs.ecx, 23));
            cpu.info.features.set_feat_movbe(is_bit_set32(regs.ecx, 22));
            cpu.info.features.set_feat_x2apic(is_bit_set32(regs.ecx, 21));
            cpu.info.features.set_feat_sse42(is_bit_set32(regs.ecx, 20));
            cpu.info.features.set_feat_sse41(is_bit_set32(regs.ecx, 19));
            cpu.info.features.set_feat_dca(is_bit_set32(regs.ecx, 18));
            cpu.info.features.set_feat_pcid(is_bit_set32(regs.ecx, 17));
            cpu.info.features.set_feat_pdcm(is_bit_set32(regs.ecx, 15));
            cpu.info.features.set_feat_etprd(is_bit_set32(regs.ecx, 14));
            cpu.info.features.set_feat_cx16(is_bit_set32(regs.ecx, 13));
            cpu.info.features.set_feat_fma(is_bit_set32(regs.ecx, 12));
            cpu.info.features.set_feat_sdbg(is_bit_set32(regs.ecx, 11));
            cpu.info.features.set_feat_cid(is_bit_set32(regs.ecx, 10));
            cpu.info.features.set_feat_ssse3(is_bit_set32(regs.ecx, 9));
            cpu.info.features.set_feat_tm2(is_bit_set32(regs.ecx, 8));
            cpu.info.features.set_feat_est(is_bit_set32(regs.ecx, 7));
            cpu.info.features.set_feat_smx(is_bit_set32(regs.ecx, 6));
            cpu.info.features.set_feat_vmx(is_bit_set32(regs.ecx, 5));
            cpu.info.features.set_feat_dscpl(is_bit_set32(regs.ecx, 4));
            cpu.info.features.set_feat_monitor(is_bit_set32(regs.ecx, 3));
            cpu.info.features.set_feat_dtes64(is_bit_set32(regs.ecx, 2));
            cpu.info.features.set_feat_pclmul(is_bit_set32(regs.ecx, 1));
            cpu.info.features.set_feat_sse3(is_bit_set32(regs.ecx, 0));

            // Individual feature flags (EDX)
            cpu.info.features.set_feat_pbe(is_bit_set32(regs.edx, 31));
            cpu.info.features.set_feat_tm1(is_bit_set32(regs.edx, 29));
            cpu.info.features.set_feat_hyperthreading(is_bit_set32(regs.edx, 28));
            cpu.info.features.set_feat_selfsnoop(is_bit_set32(regs.edx, 27));
            cpu.info.features.set_feat_sse2(is_bit_set32(regs.edx, 26));
            cpu.info.features.set_feat_sse(is_bit_set32(regs.edx, 25));
            cpu.info.features.set_feat_fxsr(is_bit_set32(regs.edx, 24));
            cpu.info.features.set_feat_mmx(is_bit_set32(regs.edx, 23));
            cpu.info.features.set_feat_acpi_therm(is_bit_set32(regs.edx, 22));
            cpu.info.features.set_feat_dtes(is_bit_set32(regs.edx, 21));
            cpu.info.features.set_feat_clfl(is_bit_set32(regs.edx, 19));
            cpu.info.features.set_feat_psn(is_bit_set32(regs.edx, 18));
            cpu.info.features.set_feat_pse36(is_bit_set32(regs.edx, 17));
            cpu.info.features.set_feat_pat(is_bit_set32(regs.edx, 16));
            cpu.info.features.set_feat_cmov(is_bit_set32(regs.edx, 15));
            cpu.info.features.set_feat_mca(is_bit_set32(regs.edx, 14));
            cpu.info.features.set_feat_pge(is_bit_set32(regs.edx, 13));
            cpu.info.features.set_feat_mtrr(is_bit_set32(regs.edx, 12));
            cpu.info.features.set_feat_sysenter(is_bit_set32(regs.edx, 11));
            cpu.info.features.set_feat_apic(is_bit_set32(regs.edx, 9));
            cpu.info.features.set_feat_cx8(is_bit_set32(regs.edx, 8));
            cpu.info.features.set_feat_mce(is_bit_set32(regs.edx, 7));
            cpu.info.features.set_feat_pae(is_bit_set32(regs.edx, 6));
            cpu.info.features.set_feat_msr(is_bit_set32(regs.edx, 5));
            cpu.info.features.set_feat_tsc(is_bit_set32(regs.edx, 4));
            cpu.info.features.set_feat_pse(is_bit_set32(regs.edx, 3));
            cpu.info.features.set_feat_de(is_bit_set32(regs.edx, 2));
            cpu.info.features.set_feat_vme(is_bit_set32(regs.edx, 1));
            cpu.info.features.set_feat_fpu(is_bit_set32(regs.edx, 0));
        }

        // CPUID / EAX == 2
        if cpu.info.max_cpuid_level >= 2 {
            let regs = x86_cpuid(2);
            let mut i = 0;

            debug_assert!(regs.eax & BYTE0_U32 == 0x01);
            
            // clear bit 31 indicates this register has
            // valid descriptors
            // EAX has bytes 1-3 (byte 0 is always 01H)
            if is_bit_clear32(regs.eax, 31) {
                if regs.eax & BYTE1_U32 != 0 {
                    let val = (regs.eax & BYTE1_U32 >> 8) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
                if regs.eax & BYTE2_U32 != 0 {
                    let val = (regs.eax & BYTE2_U32 >> 16) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
                if regs.eax & BYTE3_U32 != 0 {
                    let val = (regs.eax & BYTE3_U32 >> 24) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
            }

            // EBX, ECX & EDX all have all 4 bytes if bit 31 is clear
            // some may contain null entries however
            if is_bit_clear32(regs.ebx, 31) {
                if regs.ebx & BYTE0_U32 != 0 {
                    let val = (regs.ebx & BYTE0_U32) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
                if regs.ebx & BYTE1_U32 != 0 {
                    let val = (regs.ebx & BYTE1_U32 >> 8) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
                if regs.ebx & BYTE2_U32 != 0 {
                    let val = (regs.ebx & BYTE2_U32 >> 16) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
                if regs.ebx & BYTE3_U32 != 0 {
                    let val = (regs.ebx & BYTE3_U32 >> 24) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
            }

            if is_bit_clear32(regs.ecx, 31) {
                if regs.ecx & BYTE0_U32 != 0 {
                    let val = (regs.ecx & BYTE0_U32) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
                if regs.ecx & BYTE1_U32 != 0 {
                    let val = (regs.ecx & BYTE1_U32 >> 8) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
                if regs.ecx & BYTE2_U32 != 0 {
                    let val = (regs.ecx & BYTE2_U32 >> 16) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
                if regs.ecx & BYTE3_U32 != 0 {
                    let val = (regs.ecx & BYTE3_U32 >> 24) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
            }

            if is_bit_clear32(regs.edx, 31) {
                if regs.edx & BYTE0_U32 != 0 {
                    let val = (regs.edx & BYTE0_U32) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
                if regs.edx & BYTE1_U32 != 0 {
                    let val = (regs.edx & BYTE1_U32 >> 8) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
                if regs.edx & BYTE2_U32 != 0 {
                    let val = (regs.edx & BYTE2_U32 >> 16) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
                if regs.edx & BYTE3_U32 != 0 {
                    let val = (regs.edx & BYTE3_U32 >> 24) as usize;
                    cpu.info.cache_descriptors[i] = CACHE_CONFIGS[val];
                    i += 1;

                    if val == 0xFE { cpu.info.mode4_tlb_info = true; }
                    if val == 0xFF { cpu.info.mode4_cache_info = true; }
                }
            }
            cpu.info.cache_descriptor_count = i as u8;
        }

        // CPUID / EAX == 7
        if cpu.info.max_cpuid_level >= 7 {
            let regs = x86_cpuid_ext(7, 0);

            //let max_sublevel = regs.eax;

            cpu.info.features_ext.set_feat_avx512vl(is_bit_set32(regs.ebx, 31));
            cpu.info.features_ext.set_feat_avx512bw(is_bit_set32(regs.ebx, 30));
            cpu.info.features_ext.set_feat_sha(is_bit_set32(regs.ebx, 29));
            cpu.info.features_ext.set_feat_avx512cd(is_bit_set32(regs.ebx, 28));
            cpu.info.features_ext.set_feat_avx512er(is_bit_set32(regs.ebx, 27));
            cpu.info.features_ext.set_feat_avx512pf(is_bit_set32(regs.ebx, 26));
            cpu.info.features_ext.set_feat_processor_trace(is_bit_set32(regs.ebx, 25));
            cpu.info.features_ext.set_feat_clwb(is_bit_set32(regs.ebx, 24));
            cpu.info.features_ext.set_feat_clflushopt(is_bit_set32(regs.ebx, 23));
            cpu.info.features_ext.set_feat_pcommit(is_bit_set32(regs.ebx, 22));
            cpu.info.features_ext.set_feat_avx512ifma(is_bit_set32(regs.ebx, 21));
            cpu.info.features_ext.set_feat_smap(is_bit_set32(regs.ebx, 20));
            cpu.info.features_ext.set_feat_adx(is_bit_set32(regs.ebx, 19));
            cpu.info.features_ext.set_feat_rdseed(is_bit_set32(regs.ebx, 18));
            cpu.info.features_ext.set_feat_avx512dq(is_bit_set32(regs.ebx, 17));
            cpu.info.features_ext.set_feat_avx512f(is_bit_set32(regs.ebx, 16));
            cpu.info.features_ext.set_feat_pqe(is_bit_set32(regs.ebx, 15));
            cpu.info.features_ext.set_feat_mpx(is_bit_set32(regs.ebx, 14));
            cpu.info.features_ext.set_feat_fpcsds(is_bit_set32(regs.ebx, 13));
            cpu.info.features_ext.set_feat_pqm(is_bit_set32(regs.ebx, 12));
            cpu.info.features_ext.set_feat_rtm(is_bit_set32(regs.ebx, 11));
            cpu.info.features_ext.set_feat_invpcid(is_bit_set32(regs.ebx, 10));
            cpu.info.features_ext.set_feat_erms(is_bit_set32(regs.ebx, 9));
            cpu.info.features_ext.set_feat_bmi2(is_bit_set32(regs.ebx, 8));
            cpu.info.features_ext.set_feat_smep(is_bit_set32(regs.ebx, 7));
            cpu.info.features_ext.set_feat_fpdp(is_bit_set32(regs.ebx, 6));
            cpu.info.features_ext.set_feat_avx2(is_bit_set32(regs.ebx, 5));
            cpu.info.features_ext.set_feat_hle(is_bit_set32(regs.ebx, 4));
            cpu.info.features_ext.set_feat_bmi1(is_bit_set32(regs.ebx, 3));
            cpu.info.features_ext.set_feat_sgx(is_bit_set32(regs.ebx, 2));
            cpu.info.features_ext.set_feat_tsc_adjust(is_bit_set32(regs.ebx, 1));
            cpu.info.features_ext.set_feat_fsgsbase(is_bit_set32(regs.ebx, 0));
            cpu.info.features_ext.set_feat_sgx_lc(is_bit_set32(regs.ecx, 30));
            cpu.info.features_ext.set_feat_rdpid(is_bit_set32(regs.ecx, 22));
            cpu.info.features_ext.set_feat_va57(is_bit_set32(regs.ecx, 16));
            cpu.info.features_ext.set_feat_avx512vp_dq(is_bit_set32(regs.ecx, 14));
            cpu.info.features_ext.set_feat_tme(is_bit_set32(regs.ecx, 13));
            cpu.info.features_ext.set_feat_avx512bitalg(is_bit_set32(regs.ecx, 12));
            cpu.info.features_ext.set_feat_avx512vnni(is_bit_set32(regs.ecx, 11));
            cpu.info.features_ext.set_feat_vpcl(is_bit_set32(regs.ecx, 10));
            cpu.info.features_ext.set_feat_vaes(is_bit_set32(regs.ecx, 9));
            cpu.info.features_ext.set_feat_gfni(is_bit_set32(regs.ecx, 8));
            cpu.info.features_ext.set_feat_cet(is_bit_set32(regs.ecx, 7));
            cpu.info.features_ext.set_feat_avx512vbmi2(is_bit_set32(regs.ecx, 6));
            cpu.info.features_ext.set_feat_ospke(is_bit_set32(regs.ecx, 4));
            cpu.info.features_ext.set_feat_pku(is_bit_set32(regs.ecx, 3));
            cpu.info.features_ext.set_feat_umip(is_bit_set32(regs.ecx, 2));
            cpu.info.features_ext.set_feat_avx512vbmi(is_bit_set32(regs.ecx, 1));
            cpu.info.features_ext.set_feat_prefetchwt1(is_bit_set32(regs.ecx, 0));
            cpu.info.features_ext.set_feat_arch_cap_msr(is_bit_set32(regs.ecx, 29));
            cpu.info.features_ext.set_feat_stibp(is_bit_set32(regs.ecx, 27));
            cpu.info.features_ext.set_feat_ibrs_mbpb(is_bit_set32(regs.ecx, 26));
            cpu.info.features_ext.set_feat_pconfig(is_bit_set32(regs.ecx, 18));
            cpu.info.features_ext.set_feat_avx512qfma(is_bit_set32(regs.ecx, 3));
            cpu.info.features_ext.set_feat_avx512qvnniw(is_bit_set32(regs.ecx, 2));
        }

        cpu
    }    
}