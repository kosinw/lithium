#![no_std]
#![feature(panic_info_message)]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod console;
mod cpu;
mod heap;
mod memory;
mod multiboot;
mod runtime;
mod trap;

/// Entrypoint for the Lithium kernel.
///
/// The library operating system calls initialization routines in this function
/// related to memory management and drivers before transferring control to the
/// linked unikernel application.
#[no_mangle]
pub extern "C" fn kernel_main(mbi_ptr: *const multiboot::MultibootInformation) {
    cpu::init(0);
    console::init();
    memory::init(mbi_ptr);
    heap::init();
    trap::init();
}
