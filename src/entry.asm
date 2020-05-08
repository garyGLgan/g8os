%include "boot/x86_64/constants.asm"
[BITS 64]

global start
extern g8start

section .entry
start:   
    ; update segments
    mov dx, GDT_SELECTOR_DATA
    mov ss, dx  ; stack segment
    mov ds, dx  ; data segment
    mov es, dx  ; extra segment
    mov fs, dx  ; f-segment
    mov gs, dx  ; g-segment
    ; set up stack
    mov rsp, stack_top
    ; jump to bootloader
    jmp g8start
    ;hlt
; reserve space for stack
section .bss
stack_bottom:
    resb (4096*1024)
stack_top: