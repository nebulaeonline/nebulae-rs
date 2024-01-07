use crate::common::base::*;

pub const MAX_SYS_ID: u64 = u32::MAX as u64 - 3;
pub const MIN_USER_ID: u64 = u32::MAX as u64 + 1;

const VERBOTEN_OWNER_ID: u64 = u32::MAX as u64;
const UEFI_OWNER_ID: u64 = u32::MAX as u64 - 1;
const FW_OWNER_ID: u64 = u32::MAX as u64 - 2;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u64)]
pub enum Owner {
    Nobody = ZERO_U64,
    Reserved = 1,
    Kernel = 2,
    Memory = 3,
    System(u64),
    Firmware = FW_OWNER_ID,
    Uefi = UEFI_OWNER_ID,
    Verboten = VERBOTEN_OWNER_ID,
    User(u64),
}
impl Owner {
    pub const fn is_system(self) -> bool {
        self.into_bits() < MIN_USER_ID
    }

    pub const fn is_user(self) -> bool {
        self.into_bits() >= MIN_USER_ID
    }
    
    pub const fn into_bits(self) -> u64 {
        match self {
            Owner::Nobody => ZERO_U64,
            Owner::Reserved => 1,
            Owner::Kernel => 2,
            Owner::Memory => 3,
            Owner::System(x) => x,
            Owner::Firmware => FW_OWNER_ID,
            Owner::Uefi => UEFI_OWNER_ID,
            Owner::Verboten => VERBOTEN_OWNER_ID,
            Owner::User(x) => x,            
        }
    }

    pub const fn from_bits(value: u64) -> Self {
        match value {
            ZERO_U64 => Owner::Nobody,
            1 => Owner::Reserved,
            2 => Owner::Kernel,
            3 => Owner::Memory,
            4..=MAX_SYS_ID => Owner::System(value),
            UEFI_OWNER_ID => Owner::Uefi,
            FW_OWNER_ID => Owner::Firmware,
            VERBOTEN_OWNER_ID => Owner::Verboten,
            MIN_USER_ID..=u64::MAX => Owner::User(value),
        }
    }
}