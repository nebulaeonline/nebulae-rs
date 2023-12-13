use core::mem;
use core::marker::PhantomData;
use core::ops::*;

use crate::common::*;

#[cfg(target_pointer_width = "64")]
pub struct Bit<T: From<u64>>{
    pub b: [usize; 64],
    _pt: PhantomData<T>,
}
#[cfg(target_pointer_width = "64")]
impl<T: From<u64>> Bit<T> {
    const fn new() -> Bit<T> {
        Bit {
            b: [ 1 << 0, 1 << 1, 1 << 2, 1 << 3,
                1 << 4, 1 << 5, 1 << 6, 1 << 7,
                1 << 8, 1 << 9, 1 << 10, 1 << 11,
                1 << 12, 1 << 13, 1 << 14, 1 << 15,
                1 << 16, 1 << 17, 1 << 18, 1 << 19,
                1 << 20, 1 << 21, 1 << 22, 1 << 23,
                1 << 24, 1 << 25, 1 << 26, 1 << 27,
                1 << 28, 1 << 29, 1 << 30, 1 << 31,
                1 << 32, 1 << 33, 1 << 34, 1 << 35,
                1 << 36, 1 << 37, 1 << 38, 1 << 39,
                1 << 40, 1 << 41, 1 << 42, 1 << 43,
                1 << 44, 1 << 45, 1 << 46, 1 << 47,
                1 << 48, 1 << 49, 1 << 50, 1 << 51,
                1 << 52, 1 << 53, 1 << 54, 1 << 55,
                1 << 56, 1 << 57, 1 << 58, 1 << 59,
                1 << 60, 1 << 61, 1 << 62, 1 << 63,
            ],
            _pt: PhantomData,
        }
    }
}

#[cfg(target_pointer_width = "32")]
pub struct Bit<T: From<u32>>{
    pub b: [usize; 32],
    _pt: PhantomData<T>,
}
#[cfg(target_pointer_width = "32")]
impl<T: From<u32>> Bit<T> {
    const fn new() -> Bit<T> {
        Bit {
            b: [ 1 << 0, 1 << 1, 1 << 2, 1 << 3,
                1 << 4, 1 << 5, 1 << 6, 1 << 7,
                1 << 8, 1 << 9, 1 << 10, 1 << 11,
                1 << 12, 1 << 13, 1 << 14, 1 << 15,
                1 << 16, 1 << 17, 1 << 18, 1 << 19,
                1 << 20, 1 << 21, 1 << 22, 1 << 23,
                1 << 24, 1 << 25, 1 << 26, 1 << 27,
                1 << 28, 1 << 29, 1 << 30, 1 << 31,
            ],
            _pt: PhantomData,
        }
    }
}

#[cfg(target_pointer_width = "64")]
impl<T: From<u64>> Bit<T> {
    pub fn get_as_u64(&self, bit: usize) -> u64 {
        debug_assert!(bit <= (mem::size_of::<T>() << FACTOR_OF_8));
        self.b[bit] as u64
    }

    pub fn get_into(&self, bit: usize) -> T {
        debug_assert!(bit <= (mem::size_of::<T>() << FACTOR_OF_8));
        self.get_as_u64(bit).into()
    }

    pub fn get_mask_first_n_bits_into(&self, n_bits: usize) -> T {
        debug_assert!(n_bits <= (mem::size_of::<T>() << FACTOR_OF_8));

        if n_bits == 0 {
            0.into()
        } else if n_bits == 64 {
            u64::MAX.into()
        } else {
            (self.get_as_u64(n_bits) - 1).into()
        }
    }

    pub fn get_as_usize(&self, bit: usize) -> usize {
        self.b[bit]
    }
}

#[cfg(target_pointer_width = "32")]
impl<T: From<u32>> Bit<T> {
    pub fn get_as_u32(&self, bit: usize) -> u32 {
        debug_assert!(bit <= (mem::size_of::<T>() << UFACTOR_OF_8));
        self.b[bit] as u32
    }

    pub fn get_into(&self, bit: usize) -> T {
        debug_assert!(bit <= (mem::size_of::<T>() << UFACTOR_OF_8));
        self.get_as_u32(bit).into()
    }

    pub fn get_mask_first_n_bits_into(&self, n_bits: usize) -> T {
        debug_assert!(n_bits <= (mem::size_of::<T>() << UFACTOR_OF_8));

        if n_bits == 0 {
            0.into()
        } else if n_bits == 32 {
            u32::MAX.into()
        } else {
            (self.get_as_u32(n_bits) - 1).into()
        }
    }

    pub fn get_as_usize(&self, bit: usize) -> usize {
        self.b[bit]
    }
}

#[cfg(target_pointer_width = "64")]
impl const Default for Bit<u64> {
    
    fn default() -> Bit<u64> {
        Bit {
            b: [ 1 << 0, 1 << 1, 1 << 2, 1 << 3,
                1 << 4, 1 << 5, 1 << 6, 1 << 7,
                1 << 8, 1 << 9, 1 << 10, 1 << 11,
                1 << 12, 1 << 13, 1 << 14, 1 << 15,
                1 << 16, 1 << 17, 1 << 18, 1 << 19,
                1 << 20, 1 << 21, 1 << 22, 1 << 23,
                1 << 24, 1 << 25, 1 << 26, 1 << 27,
                1 << 28, 1 << 29, 1 << 30, 1 << 31,
                1 << 32, 1 << 33, 1 << 34, 1 << 35,
                1 << 36, 1 << 37, 1 << 38, 1 << 39,
                1 << 40, 1 << 41, 1 << 42, 1 << 43,
                1 << 44, 1 << 45, 1 << 46, 1 << 47,
                1 << 48, 1 << 49, 1 << 50, 1 << 51,
                1 << 52, 1 << 53, 1 << 54, 1 << 55,
                1 << 56, 1 << 57, 1 << 58, 1 << 59,
                1 << 60, 1 << 61, 1 << 62, 1 << 63,
            ],
            _pt: PhantomData,
        }
    }
}

#[cfg(target_pointer_width = "32")]
impl const Default for Bit<u32> {
    
    fn default() -> Bit<u32> {
        Bit {
            b: [ 1 << 0, 1 << 1, 1 << 2, 1 << 3,
                1 << 4, 1 << 5, 1 << 6, 1 << 7,
                1 << 8, 1 << 9, 1 << 10, 1 << 11,
                1 << 12, 1 << 13, 1 << 14, 1 << 15,
                1 << 16, 1 << 17, 1 << 18, 1 << 19,
                1 << 20, 1 << 21, 1 << 22, 1 << 23,
                1 << 24, 1 << 25, 1 << 26, 1 << 27,
                1 << 28, 1 << 29, 1 << 30, 1 << 31,
            ],
            _pt: PhantomData,
        }
    }
}

#[cfg(target_pointer_width = "64")]
pub const  BIT: Bit<u64> = Bit::new();

#[cfg(target_pointer_width = "32")]
pub const  BIT: Bit<u32> = Bit::new();

pub fn bit<T: From<usize> + Copy>(bit: usize) -> T {
    debug_assert!(bit < mem::size_of::<T>() * 8);
    BIT.b[bit].into()
}

pub fn set_bit<T: BitOr<Output = T> + From<usize> + From<usize> + Copy>(value: T, bit: usize) -> T {
    debug_assert!(bit < mem::size_of::<T>() * 8);
    T::from(BIT.b[bit]) | value
}

pub fn clear_bit<T: BitAnd<Output = T> + From<usize> + From<usize> + Copy>(value: T, bit: usize) -> T {
    debug_assert!(bit < mem::size_of::<T>() * 8);
    T::from(!BIT.b[bit]) & value
}

pub fn get_bit<T: BitAnd<Output = T> + From<usize> + From<usize> + PartialEq + Copy>(value: T, bit: usize) -> bool {
    debug_assert!(bit < mem::size_of::<T>() * 8);
    T::from(BIT.b[bit]) & value == value
}

pub trait Bitmask {
    fn bitmask(&self, mask: usize) -> Self;
}

impl Bitmask for usize {
    fn bitmask(&self, mask: usize) -> Self {
        self & mask
    }
}

impl Bitmask for u64 {
    fn bitmask(&self, mask: usize) -> Self {
        self & mask as u64
    }
}

impl Bitmask for u32 {
    fn bitmask(&self, mask: usize) -> Self {
        self & mask as u32
    }
}

pub const fn usizebit(bit: usize) -> usize {
    debug_assert!(bit < mem::size_of::<usize>() * 8);
    BIT.b[bit]
}

pub const fn u64bit(bit: usize) -> u64 {
    debug_assert!(bit < 64);
    BIT.b[bit] as u64
}

pub const fn u32bit(bit: usize) -> u32 {
    debug_assert!(bit < 32);
    BIT.b[bit] as u32
}

pub const fn u16bit(bit: usize) -> u16 {
    debug_assert!(bit < 16);
    BIT.b[bit] as u16
}

pub const fn u8bit(bit: usize) -> u8 {
    debug_assert!(bit < 8);
    BIT.b[bit] as u8
}

#[inline(always)]
pub const fn set_bit8(num: u8, bit: usize) -> u8 {
    debug_assert!(bit < 8);
    num | (BIT.b[bit] as u8)
}

#[inline(always)]
pub const fn set_bit16(num: u16, bit: usize) -> u16 {
    debug_assert!(bit < 16);
    num | (BIT.b[bit] as u16)
}

#[inline(always)]
pub const fn set_bit32(num: u32, bit: usize) -> u32 {
    debug_assert!(bit < 32);
    num | (BIT.b[bit] as u32)
}

#[inline(always)]
pub const fn set_bit64(num: u64, bit: usize) -> u64 {
    debug_assert!(bit < 64);
    num | BIT.b[bit] as u64
}

#[inline(always)]
pub const fn set_bit_usize(num: usize, bit: usize) -> usize {
    debug_assert!(bit < mem::size_of::<usize>() * 8);
    num | BIT.b[bit]
}

#[inline(always)]
pub const fn clear_bit8(num: u8, bit: usize) -> u8 {
    debug_assert!(bit < 8);
    num & (!BIT.b[bit] as u8)
}

#[inline(always)]
pub const fn clear_bit16(num: u16, bit: usize) -> u16 {
    debug_assert!(bit < 16);
    num & (!BIT.b[bit] as u16)
}

#[inline(always)]
pub const fn clear_bit32(num: u32, bit: usize) -> u32 {
    debug_assert!(bit < 32);
    num & (!BIT.b[bit] as u32)
}

#[inline(always)]
pub const fn clear_bit64(num: u64, bit: usize) -> u64 {
    debug_assert!(bit < 64);
    num & !BIT.b[bit] as u64
}

#[inline(always)]
pub const fn clear_bit_usize(num: usize, bit: usize) -> usize {
    debug_assert!(bit < mem::size_of::<usize>() * 8);
    num & !BIT.b[bit]
}

#[inline(always)]
pub const fn is_bit_set8(num: u8, bit: usize) -> bool {
    debug_assert!(bit < 8);
    num & (BIT.b[bit] as u8) == BIT.b[bit] as u8
}

#[inline(always)]
pub const fn is_bit_set16(num: u16, bit: usize) -> bool {
    debug_assert!(bit < 16);
    (num & BIT.b[bit] as u16) == BIT.b[bit] as u16
}

#[inline(always)]
pub const fn is_bit_set32(num: u32, bit: usize) -> bool {
    debug_assert!(bit < 32);
    (num & BIT.b[bit] as u32) == BIT.b[bit] as u32
}

#[inline(always)]
pub const fn is_bit_set64(num: u64, bit: usize) -> bool {
    debug_assert!(bit < 64);
    (num & BIT.b[bit] as u64) == BIT.b[bit] as u64
}

#[inline(always)]
pub const fn is_bit_set_uintn(num: Uintn, bit: usize) -> bool {
    debug_assert!(bit < MACHINE_UBITS);
    (num & BIT.b[bit] as Uintn) == BIT.b[bit] as Uintn
}

#[inline(always)]
pub const fn is_bit_set_usize(num: usize, bit: usize) -> bool {
    debug_assert!(bit < MACHINE_UBITS);
    (num & BIT.b[bit]) == BIT.b[bit]
}

#[inline(always)]
pub const fn is_bit_clear8(num: u8, bit: usize) -> bool {
    debug_assert!(bit < 8);
    num & (BIT.b[bit] as u8) == 0
}

#[inline(always)]
pub const fn is_bit_clear16(num: u16, bit: usize) -> bool {
    debug_assert!(bit < 16);
    (num & BIT.b[bit] as u16) == 0
}

#[inline(always)]
pub const fn is_bit_clear32(num: u32, bit: usize) -> bool {
    debug_assert!(bit < 32);
    (num & BIT.b[bit] as u32) == 0
}

#[inline(always)]
pub const fn is_bit_clear64(num: u64, bit: usize) -> bool {
    debug_assert!(bit < 64);
    (num & BIT.b[bit] as u64) == 0
}

#[inline(always)]
pub const fn is_bit_clear_uintn(num: Uintn, bit: usize) -> bool {
    debug_assert!(bit < MACHINE_UBITS);
    (num & BIT.b[bit] as Uintn) == 0
}

#[inline(always)]
pub const fn is_bit_clear_usize(num: usize, bit: usize) -> bool {
    debug_assert!(bit < mem::size_of::<usize>() * 8);
    (num & BIT.b[bit]) == 0
}