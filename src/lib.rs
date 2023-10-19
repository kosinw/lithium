#![no_std]
#![feature(abi_x86_interrupt)]

pub mod macros;
pub mod sys;

pub fn init(_boot_info: &'static bootloader::BootInfo) {
    sys::gdt::init();
    sys::interrupts::init();
    sys::serial::init();

    println!("welcome to \x1b[34mlithium\x1b[0m!\n\n");
    info!("Starting kernel version {}...", env!("CARGO_PKG_VERSION"));

    sys::memory::init();

    println!();
}
