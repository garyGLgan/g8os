#![no_std]
#![cfg_attr(test, no_main)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(asm)]
#![feature(box_syntax, box_patterns)]
#![feature(const_fn)]
#![feature(core_intrinsics)]
#![feature(integer_atomics)]
#![feature(lang_items)]
#![feature(maybe_uninit_extra)]
#![feature(naked_functions)]
#![feature(no_more_cas)]
#![feature(panic_info_message)]
#![feature(ptr_internals)]
#![feature(stmt_expr_attributes)]
#![feature(trait_alias)]
#![feature(try_trait)]

use core::panic::PanicInfo;

pub mod vga_buffer;

#[no_mangle]
pub unsafe extern "C" fn g8_main() {
    println!("welcome to G8 OS!!!");
    hlt_loop()
}


pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("Panic: {}", info);
    hlt_loop();
}