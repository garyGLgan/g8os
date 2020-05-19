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
use memory::paging::g8_page_table::PAGE_TABLE;
use kernel_const::{STACK_BOTTOM, STACK_TOP};
use x86_64::VirtAddr;

#[no_mangle]
pub unsafe extern "C" fn g8start() {
    println!("Welcom to G8 OS!");
    println!("Auth: Gary Gan");
    init();
    stack_overflow(1);
    hlt_loop()
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

fn stack_overflow(mut i : u64) {
    println!("{}.",i);
    let j = i +1;
    for i in 0..10 {
        x86_64::instructions::hlt();
    }
    stack_overflow(j);
}

pub fn init() {
    gdt::init();
    idt::init_idt();
    unsafe { idt::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
    let frame_alloc = &frame_controller::FRAME_ALLOC;
    println!("Frame allocator initialized!");
    frame_alloc.print_out();

    if let Ok((frame, flusher)) = PAGE_TABLE.lock().unmap(VirtAddr::new(STACK_BOTTOM)) {
        flusher.flush();
    }else {
        panic!("unmap failed")
    }

    // let ptr = 0xdffff0 as *mut u64;
    // unsafe {
    //     ptr.write(10000);
    // }
}

#[panic_handler]
#[no_mangle]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic: {}", info);
    hlt_loop();
}
