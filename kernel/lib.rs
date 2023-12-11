#![no_std]
#![feature(panic_info_message)]

extern crate alloc;

mod console;
mod cpu;
mod heap;
mod memory;
mod multiboot;
mod runtime;

use multiboot::MultibootInformation;

use alloc::vec;

/// Entrypoint for lithium kernel.
///
/// The library operating system calls initialization routines in this function
/// related to memory management and drivers before transferring control to the
/// linked unikernel application.
#[no_mangle]
pub extern "C" fn kernel_main(mbi_ptr: *const MultibootInformation) {
    cpu::init(0);
    console::init();
    memory::init(mbi_ptr);
    heap::init();

    for i in 0..10 {
        let test = vec![i*3, i*3+1, i*3+2];
        log!("my vectors :) {:?} {:016p}", test, test.as_ptr());
    }
}
