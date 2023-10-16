#![no_std]
#![feature(abi_x86_interrupt)]

pub mod sys;
pub mod macros;

pub fn init(_boot_info: &'static bootloader::BootInfo) {
    sys::interrupts::init();
    sys::serial::init();

    println!("Welcome to \x1b[34mLithium\x1b[0m!\n\n");
    info!("Starting kernel version {}", env!("CARGO_PKG_VERSION"));
}
