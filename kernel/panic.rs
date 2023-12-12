use crate::print;

use core::panic::PanicInfo;

use x86_64::instructions;
use x86_64::instructions::port::PortWriteOnly;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    const ANSI_FOREGROUND_RED: &str = "\x1b[31m";
    const ANSI_FOREGROUND_CYAN: &str = "\x1b[36m";
    const ANSI_CLEAR: &str = "\x1b[0m";

    instructions::interrupts::disable();

    // print!("\x1bc");
    print!("{ANSI_FOREGROUND_RED}[        panic]{ANSI_CLEAR} ");

    if let Some(location) = info.location() {
        print!("{ANSI_FOREGROUND_CYAN}");
        print!(
            "{0: <20} | line {1: <5} | {ANSI_CLEAR} ",
            location.file(),
            location.line()
        );
    }

    if let Some(msg) = info.message() {
        print!("{}\n", format_args!("{}", msg));
    } else if let Some(payload) = info.payload().downcast_ref::<&'static str>() {
        print!("{}\n", payload);
    }

    unsafe {
        PortWriteOnly::new(0x604).write(0x2000u16);
        loop {
            instructions::hlt();
        }
    }
}
