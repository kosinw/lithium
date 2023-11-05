use alloc::string::{String, ToString};
use core::{
    fmt,
    sync::atomic::{AtomicBool, Ordering},
};
use lazy_static::lazy_static;
use spin::Mutex;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::sys::serial::print_fmt(format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! println {
    () => ({
        $crate::print!("\n");
    });
    ($($arg:tt)*) => ({
        $crate::print!("{}\n", format_args!($($arg)*));
    });
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ({
        let csi_green = $crate::sys::console::Style::new().foreground($crate::sys::console::Color::Green);
        let reset_color = $crate::sys::console::Style::reset();

        $crate::print!("{}[ INFO ]{} ", csi_green, reset_color);
        $crate::print!("{}", format_args!($($arg)*));
        $crate::println!();
    });
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        let csi_yellow = $crate::sys::console::Style::new().foreground($crate::sys::console::Color::Yellow);
        let reset_color = $crate::sys::console::Style::reset();

        $crate::print!("{}[ DEBUG ]{} ", csi_yellow, reset_color);
        $crate::print!("{}", format_args!($($arg)*));
        $crate::println!();
    });
}

#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => ({
        let csi_red = $crate::sys::console::Style::new().foreground($crate::sys::console::Color::Red);
        let reset_color = $crate::sys::console::Style::reset();

        $crate::print!("{}[ ERROR ]{} ", csi_red, reset_color);
        $crate::print!("{}", format_args!($($arg)*));
        $crate::println!();
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 30,
    Red = 31,
    Green = 32,
    Brown = 33,
    Blue = 34,
    Magenta = 35,
    Cyan = 36,
    LightGray = 37,
    DarkGray = 90,
    LightRed = 91,
    LightGreen = 92,
    Yellow = 93,
    LightBlue = 94,
    Pink = 95,
    LightCyan = 96,
    White = 97,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Style {
    foreground: Option<Color>,
    background: Option<Color>,
}

impl Style {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn reset() -> Self {
        Default::default()
    }

    pub fn foreground(self, color: Color) -> Self {
        Self {
            foreground: Some(color),
            background: self.background,
        }
    }

    pub fn background(self, color: Color) -> Self {
        Self {
            foreground: self.foreground,
            background: Some(color),
        }
    }
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(fg) = self.foreground {
            if let Some(bg) = self.background {
                write!(f, "\x1b[{};{}m", fg as u8, (bg as u8) + 10)
            } else {
                write!(f, "\x1b[{}m", fg as u8)
            }
        } else if let Some(bg) = self.background {
            write!(f, "\x1b[{},", (bg as u8) + 10)
        } else {
            write!(f, "\x1b[0m")
        }
    }
}

pub struct Console {
    stdin: Mutex<String>,
    echo: AtomicBool,
}

impl Console {
    fn new() -> Self {
        Self {
            stdin: Mutex::new(String::new()),
            echo: AtomicBool::new(true),
        }
    }

    pub fn is_echo(&self) -> bool {
        self.echo.load(Ordering::SeqCst)
    }

    pub fn set_echo(&mut self, b: bool) {
        self.echo.store(b, Ordering::SeqCst);
    }
}

lazy_static! {
    static ref CONSOLE: Mutex<Console> = Mutex::new(Console::new());
}

const ETX_KEY: char = '\x03'; // End of Text
const EOT_KEY: char = '\x04'; // End of Transmission
const BS_KEY: char = '\x08'; // Backspace
const ESC_KEY: char = '\x1B'; // Escape

pub fn is_echo_enabled() -> bool {
    CONSOLE.lock().is_echo()
}

pub fn enable_echo() {
    CONSOLE.lock().set_echo(true)
}

pub fn disable_echo() {
    CONSOLE.lock().set_echo(false)
}

pub fn keypress(key: char) {
    let console = CONSOLE.lock();
    let mut stdin = console.stdin.lock();

    if key == BS_KEY {
        if let Some(c) = stdin.pop() {
            let n = match c {
                ETX_KEY | EOT_KEY | ESC_KEY => 2,
                _ => c.len_utf8(),
            };

            crate::print!("{}", BS_KEY.to_string().repeat(n));
        }
    } else {
        stdin.push(key);
        if console.is_echo() {
            match key {
                ETX_KEY => crate::print!("^C"),
                EOT_KEY => crate::print!("^D"),
                ESC_KEY => crate::print!("^["),
                _ => crate::print!("{}", key),
            }
        }
    }
}
