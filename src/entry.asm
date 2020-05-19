%include "boot/x86_64/constants.asm"
[BITS 64]

global start
extern g8start

section .entry
start:   
    ; update segments
    xor ax, ax
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    ; set up stack
    mov rsp, STACK_TOP
    ; jump to bootloader
    jmp g8start
    ;hlt
; reserve space for stack
