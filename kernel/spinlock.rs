#![allow(dead_code)]

use crate::arch::{asm, cpu, without_interrupts};
use core::option::Option;
use core::cell::UnsafeCell;
use core::marker::{Send,Sized,Sync};
use core::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct Spinlock {
    locked: u64,
    name: &'static str,
    cpu: Option<u32>
}

unsafe impl Send for Spinlock {}
unsafe impl Sync for Spinlock {}

impl Spinlock {
    pub const fn new(name: &'static str) -> Spinlock {
        Spinlock {
            locked: 0,
            name,
            cpu: None
        }
    }

    pub fn acquire(&mut self) {
        unsafe {
            cpu::push_interrupt_off();
            let cpu = Some(cpu::id());
            assert!(!self.holding(), "acquire(): nested lock {} on cpu {cpu:?}", self.name);
            while asm::xchg(&mut self.locked, 1) != 0 {
                asm::pause();
            }
            self.cpu = Some(cpu::id());
        }
    }

    pub fn release(&mut self) {
        unsafe {
            assert!(self.holding(), "release(): unlocking unheld lock {}", self.name);
            self.cpu = None;
            asm::xchg(&mut self.locked, 0);
            cpu::pop_interrupt_off();
        }
    }

    pub fn holding(&self) -> bool {
        without_interrupts(|| {
            self.locked != 0 && self.cpu.is_some_and(|x| x == cpu::id())
        })
    }
}

#[derive(Debug)]
pub struct SpinMutex<T: ?Sized> {
    lock: UnsafeCell<Spinlock>,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized> Send for SpinMutex<T> {}
unsafe impl<T: ?Sized> Sync for SpinMutex<T> {}

impl<T> SpinMutex<T> {
    pub const fn new(name: &'static str, data: T) -> SpinMutex<T> {
        SpinMutex {
            lock: UnsafeCell::new(Spinlock::new(name)),
            data: UnsafeCell::new(data)
        }
    }

    pub fn acquire(&self) {
        unsafe { &mut *self.lock.get() }.acquire();
    }

    pub fn release(&self) {
        unsafe { &mut *self.lock.get() }.release();
    }

    pub fn lock(&self) -> MutexGuard<T> {
        self.acquire();
        MutexGuard {
            lock: &self.lock,
            data: unsafe { &mut *self.data.get() }
        }
    }

    pub fn lock_ref(&self) -> &Spinlock {
        unsafe { &*self.lock.get() }
    }

    pub fn holding(&self) -> bool {
        self.lock_ref().holding()
    }

    pub fn with_lock<U, F: FnMut(&mut T) -> U>(&self, mut thunk: F) -> U {
        self.acquire();
        let r = thunk(unsafe { &mut *self.data.get() });
        self.release();
        r
    }
}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a UnsafeCell<Spinlock>,
    data: &'a mut T
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { &mut *self.lock.get() }.release();
    }
}