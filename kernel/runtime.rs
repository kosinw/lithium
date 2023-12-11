use crate::print;

use core::panic::PanicInfo;

use x86_64::instructions;
use x86_64::instructions::port::PortWriteOnly;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    const ANSI_FOREGROUND_RED: &str = "\x1b[31m";
    const ANSI_FOREGROUND_CYAN: &str = "\x1b[36m";
    const ANSI_CLEAR: &str = "\x1b[0m";

    print!("\x1bc");
    print!("{ANSI_FOREGROUND_RED}[        panic]{ANSI_CLEAR} ");

    if let Some(location) = info.location() {
        let file_location = location.file();
        print!("{ANSI_FOREGROUND_CYAN}{file_location:<22.22}{ANSI_CLEAR}");
        print!("{}:{} ", location.file(), location.line());
    }

    if let Some(msg) = info.message() {
        print!("{}\n", format_args!("{}", msg));
    } else if let Some(payload) = info.payload().downcast_ref::<&'static str>() {
        print!("{}\n", payload);
    }

    unsafe {
        PortWriteOnly::new(0x604).write(4u16); // tell QEMU to turn off
        loop {
            instructions::hlt();
        }
    }
}
