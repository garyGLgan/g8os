#![no_std]
#![cfg_attr(test, no_main)]
#![feature(const_fn)]
#![feature(alloc_layout_extra)]
#![feature(const_in_array_repeat_expressions)]
#![feature(abi_x86_interrupt)]
#![feature(const_raw_ptr_deref)]

use core::panic::PanicInfo;
pub mod gdt;
pub mod idt;
pub mod kernel_const;
pub mod memory;
pub mod vga_buffer;

use memory::frame_controller;

#[no_mangle]
pub unsafe extern "C" fn g8start() {
    println!("Welcom to G8 OS!");
    println!("Auth: Gary Gan");
    init();
    hlt_loop()
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

// fn stack_overflow() {
//     stack_overflow();
// }

pub fn init() {
    gdt::init();
    idt::init_idt();
    unsafe { idt::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
    let frame_alloc = &frame_controller::FRAME_ALLOC;
    println!("Frame allocator initialized!");
    frame_alloc.print_out();
    hlt_loop()
}

#[panic_handler]
#[no_mangle]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic: {}", info);
    hlt_loop();
}
