#![no_std]
#![feature(panic_info_message)]

mod arch;
mod console;
mod memory;
mod multiboot;
mod spinlock;

use crate::arch::paging::KernelPage;
use crate::multiboot::MultibootInfo;

static mut PERCPU0: KernelPage = KernelPage::empty();

/// # Safety
#[no_mangle]
pub unsafe extern "C" fn kernel_main(mbi_ptr: *const MultibootInfo) {
    arch::cpu::init(&mut PERCPU0, 0); // per-cpu kernel data
    console::init(); // console driver
    memory::framealloc::init(mbi_ptr); // physical frame allocator
    memory::vm::init(); // create kernel page table
}

mod runtime {
    use crate::arch::asm;
    use crate::kprint;
    use core::panic::PanicInfo;

    #[panic_handler]
    fn panic(info: &PanicInfo) -> ! {
        const ANSI_FOREGROUND_RED: &str = "\x1b[31m";
        const ANSI_FOREGROUND_CYAN: &str = "\x1b[36m";
        const ANSI_CLEAR: &str = "\x1b[0m";

        kprint!("\n");
        kprint!("{ANSI_FOREGROUND_RED}[        panic]{ANSI_CLEAR} ");

        if let Some(location) = info.location() {
            let file_location = location.file();
            kprint!("{ANSI_FOREGROUND_CYAN}{file_location:<22.22}{ANSI_CLEAR}");
            kprint!("{}:{} ", location.file(), location.line());
        }

        if let Some(msg) = info.message() {
            kprint!("{}\n", format_args!("{}", msg));
        } else if let Some(payload) = info.payload().downcast_ref::<&'static str>() {
            kprint!("{}\n", payload);
        }

        system_off()
    }

    fn system_off() -> ! {
        unsafe {
            asm::outw(0x604, 0x2000);
            loop {
                asm::hlt();
            }
        }
    }
}
