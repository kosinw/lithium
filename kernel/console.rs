#![allow(dead_code)]

pub mod uart {
    use crate::spin_until;
    use bitflags::bitflags;
    use core::fmt::Write;
    use spin::Mutex;
    use x86_64::instructions::{interrupts, port::Port};

    pub const COM1: u16 = 0x3F8;

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
            const INPUT_FULL = 1 << 0;
            const OUTPUT_EMPTY = 1 << 5;
        }
    }

    #[inline]
    pub const fn ctrl(c: u8) -> u8 {
        c - b'@'
    }

    pub const BACKSPACE: u8 = ctrl(b'H');
    pub const DELETE: u8 = 0x7F;

    static mut UART: Mutex<Uart> = Mutex::new(Uart(COM1));

    pub fn init() {
        unsafe {
            UART.lock().init();
        }
    }

    pub fn print(args: core::fmt::Arguments) {
        interrupts::without_interrupts(|| unsafe {
            UART.lock().write_fmt(args).unwrap();
        });
    }

    pub fn read() -> Option<u8> {
        unsafe { UART.lock().receive() }
    }

    fn outb(port: u16, v: u8) {
        unsafe {
            Port::new(port).write(v);
        }
    }

    fn inb(port: u16) -> u8 {
        unsafe { Port::new(port).read() }
    }

    #[derive(Debug)]
    struct Uart(u16);

    impl Uart {
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

        fn line_status(&mut self) -> LineStatusFlags {
            LineStatusFlags::from_bits_truncate(inb(self.port_line_status()))
        }

        fn send(&mut self, data: u8) {
            match data {
                BACKSPACE | DELETE => {
                    spin_until!(self.line_status().contains(LineStatusFlags::OUTPUT_EMPTY));
                    outb(self.port_data(), b'\x08');
                    spin_until!(self.line_status().contains(LineStatusFlags::OUTPUT_EMPTY));
                    outb(self.port_data(), b' ');
                    spin_until!(self.line_status().contains(LineStatusFlags::OUTPUT_EMPTY));
                    outb(self.port_data(), b'\x08');
                }
                _ => {
                    spin_until!(self.line_status().contains(LineStatusFlags::OUTPUT_EMPTY));
                    outb(self.port_data(), data);
                }
            }
        }

        fn send_raw(&mut self, data: u8) {
            spin_until!(self.line_status().contains(LineStatusFlags::OUTPUT_EMPTY));
            outb(self.port_data(), data);
        }

        fn receive(&mut self) -> Option<u8> {
            if self.line_status().contains(LineStatusFlags::INPUT_FULL) {
                Some(inb(self.port_data()))
            } else {
                None
            }
        }
    }

    impl core::fmt::Write for Uart {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for byte in s.bytes() {
                self.send(byte);
            }
            core::fmt::Result::Ok(())
        }
    }
}

use crate::trap;
use spin::Mutex;

pub struct ConsoleInputBuffer {
    buffer: [char; 256],
    read_index: usize,
    write_index: usize,
    edit_index: usize,
    echo: bool,
}

static mut INPUT_BUFFER: Mutex<ConsoleInputBuffer> = Mutex::new(ConsoleInputBuffer {
    buffer: ['\x00'; 256],
    read_index: 0,
    write_index: 0,
    edit_index: 0,
    echo: false,
});

pub fn init() {
    uart::init();
    crate::print!("\x1bc"); // clears the screen
    crate::println!();
    crate::log!("console::init(): booting lithium... [ \x1b[0;32mOK\x1b[0m ]");
}

pub fn print(args: core::fmt::Arguments) {
    uart::print(args);
}

pub fn interrupt() {
    unsafe {
        // let ch = uart::read() as char;
        let mut buf = INPUT_BUFFER.lock();

        const CTRL_U: u8 = uart::ctrl(b'U');

        while let Some(mut ch) = uart::read() {
            match ch {
                CTRL_U => {
                    while {
                        let e = buf.edit_index;
                        let w = buf.write_index;
                        e != w && buf.buffer[(e - 1) % 256] != '\n'
                    } {
                        buf.edit_index -= 1;

                        if buf.echo {
                            crate::print!("{}", uart::BACKSPACE as char);
                        }
                    }
                }
                _ => {
                    if ch != b'\x00' && (buf.edit_index - buf.read_index) < 256 {
                        ch = if ch == b'\r' { b'\n' } else { ch };
                        let e = buf.edit_index;
                        buf.buffer[e] = ch as char;
                        buf.edit_index = buf.edit_index.wrapping_add(1) % 256;

                        if buf.echo {
                            crate::print!("{}", ch as char);
                        }

                        if ch == b'\n'
                            || ch == uart::ctrl(b'D')
                            || buf.edit_index == buf.read_index + 256
                        {
                            buf.write_index = buf.edit_index;
                        }
                    }
                }
            };
        }
    }
}

pub fn enable_interrupts() {
    // let _ = uart::read();
    trap::enable_irq(trap::IRQ_COM1);
}

pub fn enable_echo(v: bool) {
    unsafe {
        INPUT_BUFFER.lock().echo = v;
    }
}

#[macro_export]
macro_rules! print {
    ($($args:tt)*) => ({
        use $crate::console::uart::print;
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
            use $crate::cpu;
            const ANSI_FOREGROUND_YELLOW: &str = "\x1b[33m";
            const ANSI_CLEAR: &str = "\x1b[0m";
            const ANSI_FOREGROUND_CYAN: &str = "\x1b[36m";
            let ticks = cpu::ticks();
            $crate::print!("{ANSI_FOREGROUND_YELLOW}[{ticks: >13.6}]{ANSI_CLEAR} ");
            $crate::print!("{ANSI_FOREGROUND_CYAN}");
            $crate::print!("{0: <20} | line {1: <5} | ", file!(), line!());
            $crate::print!("{ANSI_CLEAR}");
            $crate::println!(" {}", format_args!($($arg)*));
        }
    })
}

#[macro_export]
macro_rules! spin_until {
    ($cond:expr) => {
        while !$cond {
            core::hint::spin_loop();
        }
    };
}
