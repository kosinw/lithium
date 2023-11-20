use core::fmt;
use core::fmt::Write;
use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;
use x86_64::instructions::interrupts;
use x86_64::structures::idt::InterruptStackFrame;

lazy_static! {
    pub static ref SERIAL: Mutex<Serial> = Mutex::new(Serial::new(0x3F8));
}

pub struct Serial {
    port: SerialPort,
}

impl Serial {
    fn new(port: u16) -> Self {
        Serial {
            port: unsafe { SerialPort::new(port) },
        }
    }

    fn init(&mut self) {
        self.port.init()
    }

    fn read_byte(&mut self) -> u8 {
        self.port.receive()
    }
}

impl Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.port.write_str(s)
    }
}

pub fn print_fmt(args: fmt::Arguments) {
    interrupts::without_interrupts(|| {
        SERIAL
            .lock()
            .write_fmt(args)
            .expect("Could not print to serial device")
    })
}

pub(crate) fn init() {
    SERIAL.lock().init();
    crate::sys::interrupts::register_irq_handler(4, serial_intr_handler);

    crate::println!("Welcome to the \x1b[34mLithium\x1b[0m operating system!\n\n");
    crate::info!("Starting kernel version {}...", env!("CARGO_PKG_VERSION"));
}

fn serial_intr_handler(_stack_frame: InterruptStackFrame) {
    let ch = SERIAL.lock().read_byte();
    if ch == 0xFF {
        return;
    }

    let p = match ch as char {
        '\r' => '\n',
        '\x7F' => '\x08',
        c => c
    };

    crate::sys::console::keypress(p);
}
