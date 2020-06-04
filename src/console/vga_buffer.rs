use core::fmt;
use core::ops::Range;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

// #[cfg(test)]
// use crate::{serial_print, serial_println};

lazy_static! {
    pub static ref WRITER: Mutex<Writer> =
        Mutex::new(Writer {
            log_row: BufferLine::new(
                BUFFER_HEIGHT - 2,
                ColorCode::new(Color::Black, Color::Black)
            ),
            input_row: BufferLine::new(
                BUFFER_HEIGHT - 1,
                ColorCode::new(Color::Black, Color::Black)
            ),
            color_codes: ColorCodes {
                debug_color: ColorCode::new(Color::Cyan, Color::Black),
                info_color: ColorCode::new(Color::LightGray, Color::Black),
                warn_color: ColorCode::new(Color::Yellow, Color::Black),
                error_color: ColorCode::new(Color::Red, Color::Black),
                input_color: ColorCode::new(Color::White, Color::Black),
                blank_color: ColorCode::new(Color::Black, Color::Black),
            },
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
            scroll_area: ScrollArea::new(0..BUFFER_HEIGHT-3),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollArea {
    start: usize, 
    end: usize,
}

impl ScrollArea {
    fn new(r: Range<usize>) -> Self {
        ScrollArea {start: r.start, end: r.end}
    }

    fn scroll_append_line(&mut self, row: usize, buffer: &mut Buffer) {
        self.scroll(buffer);
        self.move_line(row, self.end, buffer);
    }

    fn scroll(&mut self, buffer: &mut Buffer) {
        if self.end - self.start >=1 {
            for row in self.start..self.end {
                for col in 0..BUFFER_WIDTH {
                    self.move_line(row+1, row, buffer);
                }
            }
        }
    }

    fn move_line(&mut self, source: usize, target: usize, buffer: &mut Buffer){
        for col in 0..BUFFER_WIDTH {
            let character = buffer.chars[source][col].read();
            buffer.chars[target][col].write(character);
        }
    }

    
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BufferLine{
    row: usize,
    column_position: usize,
    blank_color: ColorCode,
}

impl BufferLine {
    fn new(row: usize, blank_color: ColorCode) -> Self {
        BufferLine{row: row, column_position: 0, blank_color: blank_color}
    }

    fn write_byte(&mut self, byte: u8, color: ColorCode, buffer: &mut Buffer, scroll_area: &mut ScrollArea){
        match byte {
            b'\n' => self.refresh(buffer, scroll_area),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.refresh(buffer, scroll_area);
                }

                let col = self.column_position;

                buffer.chars[self.row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code: color,
                });
                self.column_position += 1;
            }
        }
    }

    fn refresh(&mut self, buffer: &mut Buffer, scroll_area: &mut ScrollArea){
        scroll_area.scroll_append_line(self.row, buffer);
        self.clear(buffer);
        self.column_position = 0;
    }

    fn clear(&mut self, buffer: &mut Buffer) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.blank_color,
        };
        for col in 0..BUFFER_WIDTH {
            buffer.chars[self.row][col].write(blank);
        }
    }

    fn write_string(&mut self, s: &str, color: ColorCode, buffer: &mut Buffer, scroll_area: &mut ScrollArea) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte, color, buffer, scroll_area),
                _ => self.write_byte(0xfe, color, buffer, scroll_area),
            }
        }
    }
}

pub struct Writer {
    log_row: BufferLine,
    input_row: BufferLine,
    color_codes: ColorCodes,
    buffer: &'static mut Buffer,
    scroll_area: ScrollArea,
}

impl Writer {
    fn log_byte(&mut self, byte: u8, color: ColorCode) {
        self.log_row.write_byte(byte, color, self.buffer,&mut self.scroll_area);
    }

    fn input_byte(&mut self, byte: u8, color: ColorCode) {
        self.input_row.write_byte(byte, color, self.buffer, &mut self.scroll_area);
    }

    pub fn debug(&mut self, s: &str) {
        self.log_row
            .write_string(s, self.color_codes.debug_color, self.buffer, &mut self.scroll_area);
    }

    pub fn info(&mut self, s: &str) {
        self.log_row
            .write_string(s, self.color_codes.info_color, self.buffer, &mut self.scroll_area);
    }

    pub fn warn(&mut self, s: &str) {
        self.log_row
            .write_string(s, self.color_codes.warn_color, self.buffer, &mut self.scroll_area);
    }

    pub fn error(&mut self, s: &str) {
        self.log_row
            .write_string(s, self.color_codes.error_color, self.buffer, &mut self.scroll_area);
    }

    pub fn input(&mut self, s: &str) {
        self.input_row
            .write_string(s, self.color_codes.input_color, self.buffer, &mut self.scroll_area);
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
