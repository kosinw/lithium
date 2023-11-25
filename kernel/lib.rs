#![no_std]

mod arch;
mod console;
mod kalloc;
mod param;
mod spinlock;

use crate::arch::paging::Page;

static mut PERCPU0: Page = Page::empty();

#[no_mangle]
pub unsafe extern "C" fn kernel_main(boot_info: u64) {
    arch::cpu::init(&mut PERCPU0, 0);
    console::init();
    print!("\x1bc"); // clears the screen
    println!("lithium kernel is booting...\n");
    kernel_println!("cpu0: cpu0 started...");
    kernel_println!("cons: uart device started...");
    kalloc::init(boot_info);    // physical page allocator
}

mod runtime {
    use core::panic::PanicInfo;

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        loop {}
    }
}