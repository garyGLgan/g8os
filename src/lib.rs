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

#[naked]
#[no_mangle]
pub unsafe extern "C" fn g8start() {
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| print_str("Hello world!"));
    
    hlt_loop()
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub fn print_str(s: &str) {
    let vga_buf = 0xb8000 as *mut u8;

    for (i, &byte) in s.as_bytes().iter().enumerate() {
        unsafe {
            *vga_buf.offset((i as isize) * 2) = byte;
            *vga_buf.offset((i as isize) * 2 + 1) = 0xb;
        }
    }
}

#[panic_handler]
#[no_mangle]
fn panic(info: &PanicInfo) -> ! {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| print_str("ERROR!"));
    hlt_loop();
}
