#![no_std]
#![cfg_attr(test, no_main)]
#![feature(const_fn)]
#![feature(alloc_layout_extra)]
#![feature(const_in_array_repeat_expressions)]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
pub mod gdt;
pub mod idt;
pub mod vga_buffer;

#[no_mangle]
pub unsafe extern "C" fn g8start() {
    println!("Welcom to G8 OS!");
    println!("Auth: Gary Gan");
    init();
    stack_overflow();
    hlt_loop()
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

fn stack_overflow() {
    stack_overflow(); 
}

pub fn init() {
    gdt::init();
    idt::init_idt();
    unsafe { idt::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

#[panic_handler]
#[no_mangle]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic: {}", info);
    hlt_loop();
}
