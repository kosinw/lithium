#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod sys;

#[macro_export]
macro_rules! magic {
    ($path:path) => {
        use core::panic::PanicInfo;
        use lithium::*;

        extern crate alloc;

        bootloader::entry_point!(kernel_main);

        fn kernel_main(boot_info: &'static bootloader::BootInfo) -> ! {
            $crate::init(boot_info);

            let f: fn() = $path;
            f();

            loop {
                x86_64::instructions::hlt();
            }
        }

        #[panic_handler]
        fn panic(info: &PanicInfo) -> ! {
            err!("{}", info);
            loop {
                x86_64::instructions::hlt();
            }
        }
    };
}

pub fn init(boot_info: &'static bootloader::BootInfo) {
    sys::serial::init();
    sys::gdt::init();
    sys::interrupts::init();
    sys::time::init();
    sys::memory::init(boot_info);

    println!();
}
