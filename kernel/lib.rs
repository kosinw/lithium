#![no_std]

mod arch;
mod console;
mod param;
mod spinlock;

use crate::arch::paging::Page;

static mut PERCPU0: Page = Page::empty();

#[no_mangle]
pub unsafe extern "C" fn kernel_main() -> ! {
    arch::cpu::init(&mut PERCPU0, 0);       // initialize per-cpu kernel structures
    console::init();                                 // initialize uart serial driver

    loop {}
}

mod runtime {
    use core::panic::PanicInfo;

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        loop {}
    }
}