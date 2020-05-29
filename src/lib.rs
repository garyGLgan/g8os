#![no_std]
#![cfg_attr(test, no_main)]
#![feature(const_fn)]
#![feature(alloc_layout_extra)]
#![feature(const_in_array_repeat_expressions)]
#![feature(abi_x86_interrupt)]
#![feature(const_raw_ptr_deref)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]

extern crate alloc;

use core::panic::PanicInfo;
pub mod gdt;
pub mod idt;
pub mod kernel_const;
pub mod memory;
pub mod util;
pub mod vga_buffer;

use alloc::boxed::Box;
use alloc::vec::Vec;
use kernel_const::STACK_BOTTOM;
use memory::frame_controller::FRAME_ALLOC;
use memory::heap_allocator;
use memory::paging::g8_page_table::PAGE_TABLE;
use x86_64::VirtAddr;

#[no_mangle]
pub unsafe extern "C" fn g8start() {
    println!("Welcom to G8 OS!");
    println!("Auth: Gary Gan");
    init();
    many_boxes_long_lived();
    hlt_loop()
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub fn init() {
    gdt::init();
    idt::init_idt();
    unsafe { idt::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
    FRAME_ALLOC.lock().print_out();

    if let Ok((frame, flusher)) = PAGE_TABLE.lock().unmap(VirtAddr::new(STACK_BOTTOM)) {
        FRAME_ALLOC.lock().deallocate(frame);
        flusher.flush();
    } else {
        panic!("unmap failed")
    }
    heap_allocator::init();
}

fn many_boxes_long_lived() {
    print!("many_boxes_long_lived... ");

    let mut v_box = Vec::<Box<i32>>::new();
    for i in 0..10000 {
        let x = Box::new(i);
        if *x != i {
            panic!("error the value in box is not correct");
        }
        v_box.push(x);
    }
    println!("[ok]");
}

#[panic_handler]
#[no_mangle]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic: {}", info);
    hlt_loop();
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
