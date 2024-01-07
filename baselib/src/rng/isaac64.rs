use crate::common::base::*;

// size constants
const ISAAC64_WORD_SZ: usize = 8;
const ISAAC64_SZ_64: usize = 1 << ISAAC64_WORD_SZ;
const ISAAC64_SZ_8: usize  = ISAAC64_SZ_64 << 2;
const IND_MASK: u64 = ((ISAAC64_SZ_64 as u64) - 1) << 3;
const ISAAC64_BUF_SZ: usize = ISAAC64_WORD_SZ * ISAAC64_SZ_64;

const MIX_SHIFT: [usize; 8] = [ 9, 9, 23, 15, 14, 20, 17, 14 ];

macro_rules! rng_step {
    ($ctx_parent:ident, $state_idx1:ident, $state_idx2:ident, $rng_idx:ident, $a:ident, $b:ident, $x:ident, $y:ident) => {
        $x = $ctx_parent.rng_state_buf[$state_idx1];

        $a = match $state_idx1 % 4 {
            0 => !($a ^ ($a << 21)),
            1 => ($a ^ ($a >> 5)),
            2 => ($a ^ ($a << 12)),
            3 => ($a ^ ($a >> 33)),
            _ => unreachable!(),
        };

        $a = $a.wrapping_add($ctx_parent.rng_state_buf[$state_idx2]);

        $state_idx2 += 1;

        $y = $ctx_parent.ind($x).wrapping_add($a).wrapping_add($b);
        $ctx_parent.rng_state_buf[$state_idx1] = $y;
        $state_idx1 += 1;
        $b = $ctx_parent.ind($y >> ISAAC64_WORD_SZ).wrapping_add($x);
        $ctx_parent.rng_buf[$rng_idx] = $b;
        $rng_idx += 1;
    };
}

#[repr(C)]
#[derive(Debug)]
pub struct Isaac64RngState {
    base_addr: PhysAddr,
    aa: u64,
    bb: u64,
    cc: u64,
    rng_buf_cur_idx: usize,

    banked8: [u8; 8],
    banked16: [u16; 4],
    banked32: [u32; 2],
}

pub struct Isaac64Rng<'n> {
    rng_buf: &'n mut [u64; ISAAC64_SZ_64],
    rng_state_buf: &'n mut [u64; ISAAC64_SZ_64],
    aa: u64,
    bb: u64,
    cc: u64,
    rng_buf_cur_idx: usize,

    banked8: [u8; 8],
    banked16: [u16; 4],
    banked32: [u32; 2],
}
impl<'n> Isaac64Rng<'n> {
    // base_addr must be page aligned and have 4KB of runway
    // -> moves the old rng buffer & state to the new base address
    pub fn rebase_buffer(&mut self, base_addr: impl MemAddr + Align + AsUsize) -> bool {
        if !base_addr.is_default_page_aligned() {
            return false;
        }

        // prepare to copy the rng_buf and rng_state_buf to the new base address
        let old_rng_buf = self.rng_buf.as_mut();
        let old_rng_state_buf = self.rng_state_buf.as_mut();
        
        // set the new rng_buf and rng_state_buf pointers
        self.rng_buf = unsafe { core::mem::transmute::<usize, &'n mut [u64; ISAAC64_SZ_64]>(base_addr.as_usize()) };
        self.rng_state_buf = unsafe { core::mem::transmute::<usize, &'n mut [u64; ISAAC64_SZ_64]>(base_addr.as_usize() + ISAAC64_BUF_SZ) };

        // copy the rng_buf and rng_state_buf to the new base address
        self.rng_buf.copy_from_slice(old_rng_buf);
        self.rng_state_buf.copy_from_slice(old_rng_state_buf);

        true
    }

    // base_addr must be page aligned and have at least 4KB of runway
    pub fn new_with_fixed_buf(base_addr: impl MemAddr + Align + AsUsize) -> Option<Self> {
        
        if !base_addr.is_default_page_aligned() {
            return None;
        }

        Some(Self {
            rng_buf: unsafe { core::mem::transmute::<usize, &'n mut [u64; ISAAC64_SZ_64]>(base_addr.as_usize()) },
            rng_state_buf: unsafe { core::mem::transmute::<usize, &'n mut [u64; ISAAC64_SZ_64]>(base_addr.as_usize() + ISAAC64_BUF_SZ) },
            aa: 0,
            bb: 0,
            cc: 0,
            rng_buf_cur_idx: 0,

            banked8: [ZERO_U8; 8],
            banked16: [ZERO_U16; 4],
            banked32: [ZERO_U32; 2],
        })
    }

    // new from an existing rng state (useful when moving things around during early boot)
    pub fn new_from_rng_state(rng_state: Isaac64RngState) -> Self {
        let rng_buf = unsafe { core::mem::transmute::<usize, &'n mut [u64; ISAAC64_SZ_64]>(rng_state.base_addr.as_usize()) };
        let rng_state_buf = unsafe { core::mem::transmute::<usize, &'n mut [u64; ISAAC64_SZ_64]>(rng_state.base_addr.as_usize() + ISAAC64_BUF_SZ) };

        Self {
            rng_buf,
            rng_state_buf,
            aa: rng_state.aa,
            bb: rng_state.bb,
            cc: rng_state.cc,
            rng_buf_cur_idx: rng_state.rng_buf_cur_idx,

            banked8: rng_state.banked8,
            banked16: rng_state.banked16,
            banked32: rng_state.banked32,
        }
    }

    // export rng state
    pub fn export_rng_state(&self) -> Isaac64RngState {
        Isaac64RngState {
            base_addr: PhysAddr(self.rng_buf.as_ptr() as usize),
            aa: self.aa,
            bb: self.bb,
            cc: self.cc,
            rng_buf_cur_idx: self.rng_buf_cur_idx,
        
            banked8: self.banked8,
            banked16: self.banked16,
            banked32: self.banked32,
        }
    }

    pub fn reseed_via_u64_val(&mut self, seed: u64) {
        self.clear_state();
        self.rng_buf[ZERO_USIZE] = seed;
        self.init(if seed == ZERO_U64 { true } else { false });
    }

    pub fn reseed_via_u64_array(&mut self, seed: &[u64; ISAAC64_SZ_64]) {
        self.clear_state();
        self.rng_buf.copy_from_slice(seed);
        self.init(false);
    }

    pub fn reseed_via_u32_array(&mut self, seed: &[u32; ISAAC64_SZ_64 * 2]) {
        self.clear_state();
        self.rng_buf.copy_from_slice(unsafe { core::mem::transmute::<&[u32; ISAAC64_SZ_64 * 2], &[u64; ISAAC64_SZ_64]>(seed) });
        self.init(false);
    }

    pub fn reseed_via_u8_array(&mut self, seed: &[u8; ISAAC64_SZ_8]) {
        self.clear_state();

        for i in ZERO_USIZE..ISAAC64_SZ_8 {
            if i % 8 == 0 {
                self.rng_buf[i >> UFACTOR_OF_8] = ZERO_U64;
            }

            self.rng_buf[i >> UFACTOR_OF_8] |= (seed[i] as u64) << ((i % 8) << UFACTOR_OF_8);
        }

        self.init(false);
    }

    pub fn shuffle(&mut self) {
        self.isaac64();
        self.reset_cur_idx();
    }

    pub fn rand64(&mut self, max: u64) -> u64 {
        let maxval = if max == u64::MAX {
            0
        } else {
            max
        };

        // see if we have enough bytes in the buffer
        self.dec_cur_idx();

        // get the next random value
        let rval = self.rng_buf[self.rng_buf_cur_idx];

        // return the random value
        if maxval == 0 { rval } else { rval % (maxval + 1) }
    }

    pub fn ranged_rand64(&mut self, min: u64, max: u64) -> u64 {
        // if there's no interval, return the value they
        // apparently want
        if min == max {
            return min;
        }

        // swap the bounds if necessary
        let (minval, maxval) = if max < min {
            (max, min)
        } else {
            (min, max)
        };

        // calculate our max random value
        let max_rval = maxval - minval;

        // obtain a random value
        let rval = self.rand64(max_rval);

        // return the random value adjusted for the range
        rval + minval
    }

    pub fn ranged_rand64_signed(&mut self, min: i64, max: i64) -> i64 {
        // if there's no interval, return the value they
        // apparently want
        if min == max {
            return min;
        }

        // no need to swap the bounds -> ranged_rand64() handles it if necessary
        
        // obtain a random value in our new unsigned range
        let rval = self.ranged_rand64(min as u64, max as u64);

        // return the random value casted to signed
        rval as i64
    }

    pub fn rand32(&mut self, max: u32) -> u32 {
        let maxval = if max == u32::MAX {
            0
        } else {
            max
        };

        // see if we have any banked 32-bit values
        if self.banked32[0] > 0 {
            // get the next random value
            let rval = self.banked32[self.banked32[0] as usize];
            self.banked32[self.banked32[0] as usize] = ZERO_U32;
            self.banked32[0] -= 1;

            return if maxval == 0 { rval } else { rval % (maxval + 1) };
        }

        // we didn't have any u32 banked, so make sure we have enough bytes in the buffer
        self.dec_cur_idx();

        // get the next random 64-bit value
        let nextval = self.rng_buf[self.rng_buf_cur_idx];

        // we are going to push the high 32-bits into the bank
        self.banked32[0] = 1;
        self.banked32[1] = ((nextval & DWORD1_U64) >> 32) as u32;

        // return the low 32-bits of the random value as the requested rand32
        if maxval == 0 { (nextval & DWORD0_U64) as u32 } else { ((nextval & DWORD0_U64) as u32) % (maxval + 1) }
    }

    pub fn ranged_rand32(&mut self, min: u32, max: u32) -> u32 {
        // if there's no interval, return the value they
        // apparently want
        if min == max {
            return min;
        }

        // swap the bounds if necessary
        let (minval, maxval) = if max < min {
            (max, min)
        } else {
            (min, max)
        };

        // calculate our max random value
        let max_rval = maxval - minval;

        // obtain a random value
        let rval = self.rand32(max_rval);

        // return the random value adjusted for the range
        rval + minval
    }

    pub fn ranged_rand32_signed(&mut self, min: i32, max: i32) -> i32 {
        // if there's no interval, return the value they
        // apparently want
        if min == max {
            return min;
        }

        // no need to swap the bounds -> ranged_rand32() handles it if necessary
        
        // obtain a random value in our new unsigned range
        let rval = self.ranged_rand32(min as u32, max as u32);

        // return the random value casted to signed
        rval as i32
    }

    pub fn rand_usize(&mut self, max: usize) -> usize {
        if cfg!(target_pointer_width = "32") {
            self.rand32(max as u32) as usize
        } else {
            self.rand64(max as u64) as usize
        }
    }

    pub fn ranged_rand_usize(&mut self, min: usize, max: usize) -> usize {
        if cfg!(target_pointer_width = "32") {
            self.ranged_rand32(min as u32, max as u32) as usize
        } else {
            self.ranged_rand64(min as u64, max as u64) as usize
        }
    }

    pub fn ranged_rand_usize_signed(&mut self, min: isize, max: isize) -> isize {
        if cfg!(target_pointer_width = "32") {
            self.ranged_rand32_signed(min as i32, max as i32) as isize
        } else {
            self.ranged_rand64_signed(min as i64, max as i64) as isize
        }
    }

    pub fn rand16(&mut self, max: u16) -> u16 {
        let maxval = if max == u16::MAX {
            0
        } else {
            max
        };

        // see if we have any banked 16-bit values
        if self.banked16[0] > 0 {
            // get the next random value
            let rval = self.banked16[self.banked16[0] as usize];
            self.banked16[self.banked16[0] as usize] = ZERO_U16;
            self.banked16[0] -= 1;

            return if maxval == 0 { rval } else { rval % (maxval + 1) };
        }

        // we didn't have any u16 banked, so make sure we have enough bytes in the buffer
        self.dec_cur_idx();

        // get the next random 64-bit value
        let nextval = self.rng_buf[self.rng_buf_cur_idx];

        // we are going to push the 3 highest 16-bit values into the bank
        self.banked16[0] = 3;
        self.banked16[1] = ((nextval & WORD1_U64) >> 16) as u16;
        self.banked16[2] = ((nextval & WORD2_U64) >> 32) as u16;
        self.banked16[3] = ((nextval & WORD3_U64) >> 48) as u16;

        // return the lowest 16-bits of the random value as the requested rand16
        if maxval == 0 { (nextval & WORD0_U64) as u16 } else { ((nextval & WORD0_U64) as u16) % (maxval + 1) }
    }

    pub fn ranged_rand16(&mut self, min: u16, max: u16) -> u16 {
        // if there's no interval, return the value they
        // apparently want
        if min == max {
            return min;
        }

        // swap the bounds if necessary
        let (minval, maxval) = if max < min {
            (max, min)
        } else {
            (min, max)
        };

        // calculate our max random value
        let max_rval = maxval - minval;

        // obtain a random value
        let rval = self.rand16(max_rval);

        // return the random value adjusted for the range
        rval + minval
    }

    pub fn ranged_rand16_signed(&mut self, min: i16, max: i16) -> i16 {
        // if there's no interval, return the value they
        // apparently want
        if min == max {
            return min;
        }

        // no need to swap the bounds -> ranged_rand16() handles it if necessary
        
        // obtain a random value in our new unsigned range
        let rval = self.ranged_rand16(min as u16, max as u16);

        // return the random value casted to signed
        rval as i16
    }

    pub fn rand8(&mut self, max: u8) -> u8 {
        let maxval = if max == u8::MAX {
            0
        } else {
            max
        };

        // see if we have any banked 8-bit values
        if self.banked8[0] > 0 {
            // get the next random value
            let rval = self.banked8[self.banked8[0] as usize];
            self.banked8[self.banked8[0] as usize] = ZERO_U8;
            self.banked8[0] -= 1;

            return if maxval == 0 { rval } else { rval % (maxval + 1) };
        }

        // we didn't have any u8 banked, so make sure we have enough bytes in the buffer
        self.dec_cur_idx();

        // get the next random 64-bit value
        let nextval = self.rng_buf[self.rng_buf_cur_idx];

        // we are going to push the 7 highest 8-bit values into the bank
        self.banked8[0] = 7;
        self.banked8[1] = ((nextval & BYTE1_U64) >> 8) as u8;
        self.banked8[2] = ((nextval & BYTE2_U64) >> 16) as u8;
        self.banked8[3] = ((nextval & BYTE3_U64) >> 24) as u8;
        self.banked8[4] = ((nextval & BYTE4_U64) >> 32) as u8;
        self.banked8[5] = ((nextval & BYTE5_U64) >> 40) as u8;
        self.banked8[6] = ((nextval & BYTE6_U64) >> 48) as u8;
        self.banked8[7] = ((nextval & BYTE7_U64) >> 56) as u8;

        // return the lowest 8-bits of the random value as the requested rand8
        if maxval == 0 { (nextval & BYTE0_U64) as u8 } else { ((nextval & BYTE0_U64) as u8) % (maxval + 1) }
    }

    pub fn ranged_rand8(&mut self, min: u8, max: u8) -> u8 {
        // if there's no interval, return the value they
        // apparently want
        if min == max {
            return min;
        }

        // swap the bounds if necessary
        let (minval, maxval) = if max < min {
            (max, min)
        } else {
            (min, max)
        };

        // calculate our max random value
        let max_rval = maxval - minval;

        // obtain a random value
        let rval = self.rand8(max_rval);

        // return the random value adjusted for the range
        rval + minval
    }

    pub fn ranged_rand8_signed(&mut self, min: i8, max: i8) -> i8 {
        // if there's no interval, return the value they
        // apparently want
        if min == max {
            return min;
        }

        // no need to swap the bounds -> ranged_rand8() handles it if necessary
        
        // obtain a random value in our new unsigned range
        let rval = self.ranged_rand8(min as u8, max as u8);

        // return the random value casted to signed
        rval as i8
    }

    #[inline(always)]
    fn ind(&self, x: u64) -> u64 {
        let idx = (x & IND_MASK) >> 3;
        self.rng_state_buf[idx as usize]
    }

    fn mix(&self, x: &mut [u64; 8]) {
        for mut i in (ZERO_USIZE..8).step_by(2) {
            x[i] = x[i].wrapping_sub(x[(i + 4) & 7]);
            x[(i + 5) & 7] ^= x[(i + 7) & 7] >> MIX_SHIFT[i];
            x[(i + 7) & 7] = x[(i + 7) & 7].wrapping_add(x[i]);
            i += 1;
            x[i] = x[i].wrapping_sub(x[(i + 4) & 7]);
            x[(i + 5) & 7] ^= x[(i + 7) & 7] << MIX_SHIFT[i];
            x[(i + 7) & 7] = x[(i + 7) & 7].wrapping_add(x[i]);
        }
    }

    fn isaac64(&mut self) {
        let mut a: u64;
        let mut b: u64;
        let mut x: u64;
        let mut y: u64;

        let mut state_idx1: usize = ZERO_USIZE;
        let mut state_idx2: usize = ISAAC64_SZ_64 / 2;
        let mut rng_idx: usize = ZERO_USIZE;
        let end_idx: usize = state_idx2;

        a = self.aa;
        self.cc += 1;
        b = self.bb + self.cc;

        while state_idx1 < end_idx {
            for _i in ZERO_USIZE..4 {
                rng_step!(self, state_idx1, state_idx2, rng_idx, a, b, x, y);
            }
        }

        state_idx2 = ZERO_USIZE;

        while state_idx2 < end_idx {
            for _i in ZERO_USIZE..4 {
                rng_step!(self, state_idx1, state_idx2, rng_idx, a, b, x, y);
            }
        }

        self.bb = b;
        self.aa = a;
    }

    fn init(&mut self, init_zero_state: bool) {
        // save 4 rounds of mixing
        // const ISAAC64_MAGIC: u64 = 0x9e3779b97f4a7c13;
        let mut x: [u64; 8] = [ 
            0x647c4677a2884b7c, 0xb9f8b322c73ac862,
            0x8c0ea5053d4712a0, 0xb29b2e824a595524,
            0x82f053db8355e0ce, 0x48fe4a0fa5a09315,
            0xae985bf2cbfc89ed, 0x98f5704f6c44c0ab 
        ];

        self.aa = ZERO_U64;
        self.bb = ZERO_U64;
        self.cc = ZERO_U64;

        for i in (ZERO_USIZE..ISAAC64_SZ_64).step_by(8) {
            if !init_zero_state {
                for j in ZERO_USIZE..8 {
                    x[j] = x[j].wrapping_add(self.rng_buf[i + j]);
                }
            }

            self.mix(&mut x);

            for j in ZERO_USIZE..8 {
                self.rng_state_buf[i + j] = x[j];
            }
        }

        if !init_zero_state {
            for i in (ZERO_USIZE..ISAAC64_SZ_64).step_by(8) {
                for j in ZERO_USIZE..8 {
                    x[j] = x[j].wrapping_add(self.rng_state_buf[i + j]);
                }

                self.mix(&mut x);

                for j in ZERO_USIZE..8 {
                    self.rng_state_buf[i + j] = x[j];
                }
            }
        }

        self.isaac64();
        self.reset_cur_idx();
    }

    fn clear_state(&mut self) {
        raw::memset_aligned((self.rng_state_buf.as_mut_ptr() as usize).as_phys(), ISAAC64_BUF_SZ, ZERO_USIZE);        
    }

    fn reset_cur_idx(&mut self) {
        self.rng_buf_cur_idx = ISAAC64_SZ_64;
    }

    fn dec_cur_idx(&mut self) {
        if self.rng_buf_cur_idx == 0 {
            self.shuffle();
        }

        self.rng_buf_cur_idx -= 1;
    }
}
