pub enum CpuAvailability {
    Single,
    Multi,
}
pub enum MemSubsystemAvailability {
    Static,
    StaticRaw,
    StaticOneWayBump,
    BitmapArray,
    FrameAlloc,
    VirtualPool,
    Full,
}
pub enum KernelServiceStatus {
    Uninit,
    SysOnly,
    SysAndUser,
    Full,
}
pub enum RuntimeMode {
    VirtUser,
    User,
    Sup,
    VirtSup,
}
pub enum KernelVerbosity {
    None,
    Errors,
    Warnings,
    Info,
    Debug,
    Trace,
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
pub enum FiberStatus {
    Created,
    Loaded,
    Linked,
    Ready,
    Running,
    ReadingMsgRing,
    WritingMsgRing,
    Exception,
    FPUException,
    Yielded,
    WaitingOnInterrupt,
    WaitingOnMsgRing,
    WaitingOnBaseCpu,
    WaitingOnCpu,
    Preempted,
}
// realms go most trusted to least trusted
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
pub enum SecurityRealm {
    PreBase = 0,
    KernelRootAddrSpaceBaseCpuNoSMTNoExtIntNoDMA = 99,
    KernelRootAddrSpaceBaseCpuNoSMTNoExtInt = 100,
    KernelRootAddrSpaceBaseCpuNoSMT = 200,
    KernelRootAddrSpaceBaseCpu = 300,
    KernelRootAddrSpace = 400,
    KernelIsolatedAddrSpace = 500,
    UserIsolatedAddrSpace = 600,
    UserSharedAddrSpace = 700,
    VirtIsolatedAddrSpace = 800,
    VirtSharedAddrSpace = 900,
    GlobalAddrSpace = 1000,
}
pub struct KernelStatus {
    pub security_realm: SecurityRealm,
    pub cpu_exec_mode: CpuAvailability,
    pub mem_subsystem: MemSubsystemAvailability,
    pub internal_exceptions: KernelServiceStatus,
    pub external_interrupts: KernelServiceStatus,
    pub virtual_mem: KernelServiceStatus,
    pub virtualization: KernelServiceStatus,
    pub runtime_mode: RuntimeMode,
}
impl KernelStatus {
    pub fn new() -> Self {
        Self {
            security_realm: SecurityRealm::PreBase,
            cpu_exec_mode: CpuAvailability::Single,
            mem_subsystem: MemSubsystemAvailability::Static,
            internal_exceptions: KernelServiceStatus::Uninit,
            external_interrupts: KernelServiceStatus::Uninit,
            virtual_mem: KernelServiceStatus::Uninit,
            virtualization: KernelServiceStatus::Uninit,
            runtime_mode: RuntimeMode::Sup,
        }
    }
}