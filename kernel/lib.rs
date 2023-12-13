#![no_std]
#![feature(panic_info_message)]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod console;
mod cpu;
mod heap;
mod memory;
mod multiboot;
mod net;
mod panic;
mod pci;
mod trap;

/// The library operating system calls initialization routines in this function
/// related to memory management and drivers before transferring control to the
/// statically-linked unikernel application.
#[no_mangle]
pub extern "C" fn kernel_main(mbi: *const multiboot::MultibootInformation) -> ! {
    cpu::init(0);
    console::init();
    memory::init(mbi);
    heap::init();
    trap::init();
    pci::init();
    net::init();

    console::enable_echo(true);

    loop {
        x86_64::instructions::hlt();
    }
}
