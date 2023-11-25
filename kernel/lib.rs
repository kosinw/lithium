#![no_std]

mod arch;
mod param;

use crate::arch::paging::Page;

static mut PERCORE0: Page = Page::empty();

/// Entry point for Lithium kernel.
///
/// # Safety
///
/// This function represents the entry point for the kernel. It is marked as `unsafe`
/// because it is typically called during the kernel initialization, and the caller
/// is responsible for ensuring that the kernel is in a valid state before invoking
/// this function.
///
#[no_mangle]
pub unsafe extern "C" fn kernel_main() -> ! {
    arch::cpu::init(&mut PERCORE0, 0);

    loop {}
}

mod runtime {
    use core::panic::PanicInfo;

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        loop {}
    }
}