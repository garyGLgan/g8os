use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

// lazy_static! {
//     pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
//         col_pos: 0,
//         color_code: ColorCode::new(Color::Yellow, Color::Black),
//         buffer: unsafe { &mut *(0xb800 as *mut Buffer) },
//     });
// }

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_char: u8,
    color: ColorCode,
}

const BUFFER_WIDTH: usize = 80;
const BUFFER_HEIGHT: usize = 25;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    col_pos: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, b: u8) {
        match b {
            b'\n' => self.new_line(),
            b => {
                // if self.col_pos >= BUFFER_WIDTH {
                //     self.new_line();
                // }

                // let r = 6;
                // let c = self.col_pos;

                let color_code = self.color_code;
                self.buffer.chars[20][0].write(ScreenChar {
                    ascii_char: b,
                    color: color_code,
                });
                // self.col_pos += 1;
                // if self.col_pos == BUFFER_WIDTH {
                //     self.new_line();
                // }
            }
        }
    }

    pub fn new_line(&mut self) {
        // for r in 1..BUFFER_HEIGHT {
        //     for c in 0..BUFFER_WIDTH {
        //         let ch = self.buffer.chars[r][c].read();
        //         self.buffer.chars[r - 1][c].write(ch);
        //     }
        // }
        // self.clear_row(BUFFER_HEIGHT - 1);
        // self.col_pos = 0;
    }

    pub fn clear_row(&mut self, r: usize) {
        let blank = ScreenChar {
            ascii_char: b'\n',
            color: self.color_code,
        };

        for c in 0..BUFFER_WIDTH {
            self.buffer.chars[r][c].write(blank);
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for b in s.bytes() {
            match b {
                0x20..=0x7e | b'\n' => self.write_byte(b),
                _ => self.write_byte(0xfe),
            }
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// #[macro_export]
// macro_rules! print {
//     ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
// }

// #[macro_export]
// macro_rules! println {
//     () => ($crate::print!("\n"));
//     ($($arg:tt)*) => ($crate::print!("{}\n",  format_args!($($arg)*)));
// }

// #[doc(hidden)]
// pub fn _print(args: fmt::Arguments) {
//     use core::fmt::Write;
//     use x86_64::instructions::interrupts;
//     interrupts::without_interrupts(|| WRITER.lock().write_fmt(args).unwrap());
// }
