use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

// #[cfg(test)]
// use crate::{serial_print, serial_println};

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_codes: ColorCodes {
            debug_color: ColorCode::new(Color::Yellow, Color::Black),
            info_color: ColorCode::new(Color::LightGray, Color::Black),
            warn_color: ColorCode::new(Color::Pink, Color::Black),
            error_color: ColorCode::new(Color::Red, Color::Black),
            input_color: ColorCode::new(Color::White, Color::Black),
            blank_color: ColorCode::new(Color::Black, Color::Black),
        },
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    Darkgray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightRed = 11,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ColorCodes {
    debug_color: ColorCode,
    info_color: ColorCode,
    warn_color: ColorCode,
    error_color: ColorCode,
    input_color: ColorCode,
    blank_color: ColorCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_codes: ColorCodes,
    buffer: &'static mut Buffer,
}

impl Writer {
    fn write_byte(&mut self, byte: u8, color: ColorCode) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 2;
                let col = self.column_position;

                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code: color,
                });
                self.column_position += 1;
            }
        }
    }

    pub fn debug(&mut self, s: &str) {
        self.write_string(s, self.color_codes.debug_color);
    }

    pub fn info(&mut self, s: &str) {
        self.write_string(s, self.color_codes.info_color);
    }

    pub fn warn(&mut self, s: &str) {
        self.write_string(s, self.color_codes.warn_color);
    }

    pub fn error(&mut self, s: &str) {
        self.write_string(s, self.color_codes.error_color);
    }

    fn write_string(&mut self, s: &str, color: ColorCode) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte, color),
                _ => self.write_byte(0xfe, color),
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_codes.blank_color,
        };

        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.info(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n",  format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

// #[test_case]
// fn test_println_simple() {
//     serial_print!("test_println...");
//     println!("test_println_simple output");
//     serial_println!("[Ok]");
// }

// #[test_case]
// fn test_println_many() {
//     serial_print!("test_println_many...");
//     for _ in 0..200 {
//         println!("test_println_many output");
//     }
//     serial_println!("[Ok]");
// }

// #[test_case]
// fn test_println_output() {
//     use core::fmt::Write;
//     use x86_64::instructions::interrupts;

//     serial_print!("test_println_output");

//     let s = "Some test string that fits on a single line";

//     interrupts::without_interrupts(|| {
//         let mut writer = WRITER.lock();
//         writeln!(writer, "\n{}", s).expect("writerln failed");

//         for (i, c) in s.chars().enumerate() {
//             let screen_char = writer.buffer.chars[BUFFER_HEIGHT - 2][i].read();
//             assert_eq!(char::from(screen_char.ascii_character), c);
//         }
//     });
//     serial_println!("[Ok]");
// }
