#![no_std]
#![cfg_attr(test, no_main)]
#![feature(const_fn)]
#![feature(alloc_layout_extra)]
#![feature(const_in_array_repeat_expressions)]

use core::panic::PanicInfo;
pub mod vga_buffer;

#[no_mangle]
pub unsafe extern "C" fn g8start() {
    println!("Welcom to G8 OS!");
    println!("Auth: Gary Gan");
    hlt_loop()
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
#[no_mangle]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic: {}", info);
    hlt_loop();
}
