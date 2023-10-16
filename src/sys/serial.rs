use uart_16550::SerialPort;
use spin::Mutex;
use lazy_static::lazy_static;
use core::fmt::Write;
use core::fmt;
use x86_64::instructions::interrupts;

lazy_static! {
    pub static ref SERIAL: Mutex<Serial> = Mutex::new(Serial::new(0x3F8));
}

pub struct Serial(SerialPort);

impl Serial {
    fn new(port: u16) -> Self {
        Serial(unsafe { SerialPort::new(port) })
    }

    fn init(&mut self) {
        self.0.init()
    }

    fn read_byte(&mut self) -> u8 {
        self.0.receive()
    }

    fn write_byte(&mut self, byte: u8) {
        self.0.send(byte);
    }
}

impl Write for Serial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }

        Ok(())
    }
}

pub fn print_fmt(args: fmt::Arguments) {
    interrupts::without_interrupts(|| {
        SERIAL.lock().write_fmt(args).expect("Could not print to serial device")
    })
}

pub fn init() {
    SERIAL.lock().init();
}