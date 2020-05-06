[BITS 64]

global start
extern g8start

section .entry
start:
    cli
   
    ; update segments
    mov dx, 0x10
    mov ss, dx  ; stack segment
    mov ds, dx  ; data segment
    mov es, dx  ; extra segment
    mov fs, dx  ; f-segment
    mov gs, dx  ; g-segment

    ; set up stack
    mov rsp, stack_top

    ; jump to bootloader
    jmp g8start

; reserve space for stack
section .bss
stack_bottom:
    resb (4096*1024)
stack_top: