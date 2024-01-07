use core::sync::atomic::{AtomicUsize, Ordering};
use core::ops::{Deref, DerefMut};
use core::cell::UnsafeCell;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use crate::arch::x86::asm::x86_noop as noop;
#[cfg(any(target_arch = "aarch64"))]
use crate::arch::a64::asm::aarch_noop as noop;

use crate::common::base::*;

// the lock guard is a wrapper around the data protected
// by the lock. we basically Indiana Jones the data out
// of the lock, and then do the same when we're done.
pub struct LockGuard<'n, T> {
    parent_lock: &'n HybridLock<T>,
    lockee: Option<T>,
    readonly: bool,
    spent: bool,
}
impl<'n, T> LockGuard<'n, T> {
    pub fn new(parent: &'n HybridLock<T>, read_only: bool) -> Self {
        Self {
            parent_lock: parent,
            lockee: unsafe { parent.data.get().as_mut().unwrap().take() },
            readonly: read_only,
            spent: false,
        }
    }

    pub fn as_ref(&self) -> &T {
        if self.spent { panic!("UB detected -> attempt to reference a spent lock guard"); }

        self.lockee.as_ref().unwrap()
    }

    pub fn as_mut(&mut self) -> Option<&mut T> {
        if self.spent { panic!("UB detected -> attempt to reference a spent lock guard"); }

        if self.readonly { return None; }
        
        Some(self.lockee.as_mut().unwrap())
    }

    pub fn force_unlock(&mut self) {
        if self.spent { panic!("UB detected -> attempt to reference a spent lock guard"); }

        unsafe { self.parent_lock.data.get().as_mut().unwrap().replace(self.lockee.take().unwrap()) };
        self.spent = true;

        self.parent_lock._unlock();
    }
}
impl<'n, T> const Deref for LockGuard<'n, T> {

    type Target = T;
    fn deref(&self) -> &Self::Target {
        if self.spent { panic!("UB detected -> attempt to reference a spent lock guard"); }

        self.lockee.as_ref().unwrap()
    }
}
impl<'n, T> const DerefMut for LockGuard<'n, T> {
    
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.spent { panic!("UB detected -> attempt to reference a spent lock guard"); }

        if self.readonly {
            panic!("UB detected -> attempt to mutably deref non-exclusive shared lock guard");
        }

        self.lockee.as_mut().unwrap()
    }
}
impl<'n, T> Drop for LockGuard<'n, T> {

    fn drop(&mut self) {
        // give the lockee back to the lock (if we weren't forcibly unlocked)
        if !self.spent {
            unsafe { self.parent_lock.data.get().as_mut().unwrap().replace(self.lockee.take().unwrap()) };
        }
        self.parent_lock._unlock();
    }
}

#[repr(u16)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LockType {
    Unknown = 0,
    NonExclusiveReadOnly = 1337,
    ExclusiveReadWrite = 5173,
}
impl LockType {
    pub fn into_bits(self) -> u16 {
        self as u16
    }

    pub fn from_bits(bits: u16) -> Self {
        match bits {
            1337 => Self::NonExclusiveReadOnly,
            5173 => Self::ExclusiveReadWrite,
            _ => Self::Unknown,
        }
    }
}

// multimode hybrid lock -> can be locked as immediate,
// spin, wait, or hybrid spin-then-wait, depending on the 
// caller's needs.
//
// supports both readonly (shared) and read/write 
// (exclusive) locks
pub struct HybridLock<T> {
    data: UnsafeCell<Option<T>>,
    lock_var: AtomicUsize,
    lock_type: LockType,
}
impl<T> HybridLock<T> {
    
    pub fn as_ref(&self) -> &Self {
        self
    }

    pub fn as_mut(&mut self) -> &mut Self {
        self
    }
    
    pub fn new(lock_type: LockType, data: T) -> Self {
        if lock_type == LockType::NonExclusiveReadOnly {
            panic!("should not have any shared locks yet");
        }

        #[cfg(all(debug_assertions, feature = "serialdbg"))]
        serial_println!("creating new lock of type {:?}", lock_type);

        Self {
            data: UnsafeCell::new(Some(data)),
            lock_var: AtomicUsize::new(ZERO_USIZE),
            lock_type,
        }
    }

    pub fn lock_non_exclusive_read(&mut self) -> Option<LockGuard<T>> {
        
        if self.lock_type != LockType::NonExclusiveReadOnly {
            return None;
        }

        self.lock_var.fetch_add(1, Ordering::SeqCst);
        Some(LockGuard::new(self, true))
    }

    pub fn try_lock_rw_immediate(&mut self) -> Option<LockGuard<T>> {

        if self.lock_type != LockType::ExclusiveReadWrite {
            return None;
        }

        if self.lock_var.compare_exchange(ZERO_USIZE, 1, Ordering::SeqCst, Ordering::Relaxed) != Ok(ZERO_USIZE) {
            return None;
        }

        Some(LockGuard::new(self, false))
    }

    pub fn lock_rw_spin(&mut self) -> LockGuard<T> {

        if self.lock_type != LockType::ExclusiveReadWrite {
            panic!("attmpt to take an exclusive rw lock (spin) on non-exclusive lock; lock_type == {:?}", self.lock_type);
        }

        while self.lock_var.compare_exchange(ZERO_USIZE, 1, Ordering::SeqCst, Ordering::Relaxed) != Ok(ZERO_USIZE) {
            // if the lock is already taken, we need to
            // wait for it to be released
            // this is a spin lock, so we just spin
            noop();
        }

        // we've acquired the lock, so we can return
        LockGuard::new(self, false)
    }

    pub fn lock_rw_wait(&mut self) -> LockGuard<T> {

        if self.lock_type != LockType::ExclusiveReadWrite {
            panic!("attmpt to take rw lock (wait) on non-exclusive lock");
        }
        
        while self.lock_var.compare_exchange(ZERO_USIZE, 1, Ordering::SeqCst, Ordering::Relaxed) != Ok(ZERO_USIZE) {
            // if the lock is already taken, we need to
            // wait for it to be released;
            // TODO: implement a wait queue
            // spin for now
            noop();
        }
        
        // we've acquired the lock, so we can return
        LockGuard::new(self, false)
    }

    pub fn lock_rw_hybrid(&mut self, max_tries_before_wait: usize) -> LockGuard<T> {

        if self.lock_type != LockType::ExclusiveReadWrite {
            panic!("attmpt to take rw lock (hybrid) on non-exclusive lock");
        }
        
        // if the fiber subsystem fuse hasn't been blown, there's
        // no wait method available yet, so we just spin on this 
        // particular invocation
        if !fiber_subsystem_fuse(true) {
            return self.lock_rw_spin();
        }

        let mut attempts = 0;
        
        // spin until we've reached the maximum number of attempts, then wait
        while self.lock_var.compare_exchange(ZERO_USIZE, 1, Ordering::SeqCst, Ordering::Relaxed) != Ok(ZERO_USIZE) {
            if attempts > max_tries_before_wait {
                // switch to wait method
                return self.lock_rw_wait();
            }
            noop();
            attempts += 1;
        }

        // we've acquired the lock, so we can return
        LockGuard::new(self, false)
    }

    fn _unlock(&self) {
        if self.lock_var.load(Ordering::Acquire) == 0 {
            return;
        }

        match self.lock_type {
            LockType::NonExclusiveReadOnly => { 
                self.lock_var.fetch_sub(1, Ordering::SeqCst);
            },
            LockType::ExclusiveReadWrite => {
                while self.lock_var.compare_exchange(1, 0, Ordering::SeqCst, Ordering::Relaxed) != Ok(1) { noop(); }
            },
            LockType::Unknown => { // should never happen
                panic!("attempt to unlock unknown lock type");
            },
        }
    }
}

unsafe impl<T> Send for HybridLock<T> {}
unsafe impl<T> Sync for HybridLock<T> {}