#![allow(dead_code)]

use crate::spinlock::SpinMutex;

pub(crate) mod uart {
    use core::fmt::Write;

    use crate::{
        arch::asm::{inb, outb},
        spinlock::SpinMutex,
    };
    use bitflags::bitflags;

    macro_rules! spin {
        ($cond:expr) => {
            while !$cond {
                $crate::arch::asm::pause();
            }
        };
    }

    bitflags! {
        pub struct InterruptEnableFlags: u8 {
            const RECEIVED = 1 << 0;
            const SENT = 1 << 1;
            const ERRORED = 1 << 2;
            const STATUS_CHANGE = 1 << 3;
        }
    }

    bitflags! {
        pub struct LineStatusFlags: u8 {
            const INPUT_FULL = 1 << 1;
            const OUTPUT_EMPTY = 1 << 5;
        }
    }

    #[derive(Debug, Copy, Clone)]
    pub struct SerialPort(u16);

    pub const COM1: u16 = 0x3F8;

    const fn ctrl(b: u8) -> u8 {
        b - b'@'
    }

    const BACKSPACE: u8 = ctrl(b'H');
    const DELETE: u8 = 0x7F;

    static mut UART: SpinMutex<SerialPort> = SpinMutex::new("uart", SerialPort::new(COM1));

    pub fn init() {
        unsafe {
            UART.lock().init();
        }
    }

    pub fn print(args: core::fmt::Arguments) {
        unsafe {
            UART.lock().write_fmt(args).unwrap();
        }
    }

    // do not acquire lock in case panic was caused by locking issues
    pub fn panic_print(args: core::fmt::Arguments) {
        SerialPort::new(COM1).write_fmt(args).unwrap()
    }

    impl SerialPort {
        fn port_base(&self) -> u16 {
            self.0
        }

        fn port_data(&self) -> u16 {
            self.port_base()
        }

        fn port_intr_enable(&self) -> u16 {
            self.port_base() + 1
        }

        fn port_fifo_ctrl(&self) -> u16 {
            self.port_base() + 2
        }

        fn port_line_ctrl(&self) -> u16 {
            self.port_base() + 3
        }

        fn port_modem_ctrl(&self) -> u16 {
            self.port_base() + 4
        }

        fn port_line_status(&self) -> u16 {
            self.port_base() + 5
        }

        pub const fn new(base: u16) -> Self {
            Self(base)
        }

        pub fn init(&mut self) {
            unsafe {
                // Disable interrupts from serial port.
                outb(self.port_intr_enable(), 0x00);

                // Enable DLAB.
                outb(self.port_line_ctrl(), 0x80);

                // Set maximum speed to 38400 bps by configuring DLL and DLM.
                outb(self.port_data(), 0x03);
                outb(self.port_intr_enable(), 0x00);

                // Disable DLAB and set data word length to 8 bits.
                outb(self.port_line_ctrl(), 0x03);

                // Enable FIFO, clear TX/RX queues and
                // set interrupt watermark at 14 bytes
                outb(self.port_fifo_ctrl(), 0xc7);

                // Mark data terminal ready, signal request to send
                // and enable auxilliary output #2 (used as interrupt line for CPU)
                outb(self.port_modem_ctrl(), 0x0b);

                // Enable interrupts
                outb(self.port_intr_enable(), 0x01);
            }
        }

        fn line_status(&mut self) -> LineStatusFlags {
            unsafe { LineStatusFlags::from_bits_truncate(inb(self.port_line_status())) }
        }

        pub fn send(&mut self, data: u8) {
            unsafe {
                match data {
                    BACKSPACE | DELETE => {
                        spin!(self.line_status().contains(LineStatusFlags::OUTPUT_EMPTY));
                        outb(self.port_data(), b'\x08');
                        spin!(self.line_status().contains(LineStatusFlags::OUTPUT_EMPTY));
                        outb(self.port_data(), b' ');
                        spin!(self.line_status().contains(LineStatusFlags::OUTPUT_EMPTY));
                        outb(self.port_data(), b'\x08');
                    }
                    _ => {
                        spin!(self.line_status().contains(LineStatusFlags::OUTPUT_EMPTY));
                        outb(self.port_data(), data);
                    }
                }
            }
        }

        pub fn send_raw(&mut self, data: u8) {
            unsafe {
                spin!(self.line_status().contains(LineStatusFlags::OUTPUT_EMPTY));
                outb(self.port_data(), data);
            }
        }

        pub fn receive(&mut self) -> u8 {
            unsafe {
                spin!(self.line_status().contains(LineStatusFlags::INPUT_FULL));
                inb(self.port_data())
            }
        }
    }

    impl core::fmt::Write for SerialPort {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for byte in s.bytes() {
                self.send(byte);
            }
            core::fmt::Result::Ok(())
        }
    }
}

pub struct Console {
    buffer: [u8; 256],
    read_index: usize,
    write_index: usize,
    edit_index: usize,
}

static mut CONSOLE: SpinMutex<Console> = SpinMutex::new(
    "cons",
    Console {
        buffer: [0u8; 256],
        read_index: 0,
        write_index: 0,
        edit_index: 0,
    },
);

pub fn init() {
    uart::init();
}

pub fn print(args: core::fmt::Arguments) {
    uart::print(args);
}

#[macro_export]
macro_rules! panicf {
    ($($args:tt)*) => ({
        use $crate::console::uart::panic_print;
        panic_print(format_args!($($args)*));
    })
}

#[macro_export]
macro_rules! print {
    ($($args:tt)*) => ({
        use $crate::console::print;
        print(format_args!($($args)*));
    })
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ({
        unsafe {
            use $crate::arch::asm;
            use $crate::arch::cpu;
            const ANSI_FOREGROUND_YELLOW: &str = "\x1b[33m";
            const ANSI_CLEAR: &str = "\x1b[0m";
            const ANSI_FOREGROUND_CYAN: &str = "\x1b[36m";
            let id = cpu::id();
            let tsc = asm::r_tsc();
            let freq = asm::r_tschz();
            let timestamp = tsc as f64 / freq as f64;
            let this_file = file!();
            $crate::print!("{ANSI_FOREGROUND_YELLOW}[{timestamp: >13.6}]{ANSI_CLEAR} ");
            $crate::print!("{ANSI_FOREGROUND_CYAN}");
            $crate::print!("{this_file:<22.22}");
            $crate::print!("{ANSI_CLEAR}");
            $crate::println!("cpu{id}: {}", format_args!($($arg)*));
        }
    })
}