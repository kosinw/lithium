#![no_std]
#![feature(panic_info_message)]

mod console;
mod cpu;
mod memory;
mod multiboot;
mod runtime;

use crate::multiboot::MultibootInfo;

/// Entrypoint for lithium kernel.
///
/// The library operating system calls initiailization routines in this function
/// related to memory management and drivers before transferring control to the
/// linked unikernel application.
#[no_mangle]
pub extern "C" fn kernel_main(mbi_ptr: *const MultibootInfo) {
    cpu::init(0); // per-cpu kernel data structure
    console::init(); // console driver
    memory::framealloc::init(mbi_ptr); // physical frame allocator
    memory::vm::init(); // create kernel page table
}
