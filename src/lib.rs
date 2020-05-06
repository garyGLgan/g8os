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

#[naked]
#[no_mangle]
pub unsafe extern "C" fn g8start() {
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| vga_buffer::WRITER.lock().write_byte('a' as u8));
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
    hlt_loop();
}
