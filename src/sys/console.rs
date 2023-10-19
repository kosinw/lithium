use core::fmt;

#[allow(dead_code)]
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
