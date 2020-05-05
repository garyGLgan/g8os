%include "boot/x86_64/constants.asm"

[BITS 64]
[ORG 0x8000]

stage2:
    cli
    ; update segments
    mov dx, GDT_SELECTOR_DATA
    mov ss, dx  ; stack segment
    mov ds, dx  ; data segment
    mov es, dx  ; extra segment
    mov fs, dx  ; f-segment
    mov gs, dx  ; g-segment

    xor rax, rax
    mov rbx, rax
    mov rcx, rax
    mov rdx, rax

    mov dword [0xb8000], 0x2f332f30

    mov al, 'E'

    ; magic number 0x7f+'ELF'
    mov ah, '!'
    cmp dword [KERNEL_LOADPOINT], 0x464c457f
    jne error

    ; bitness and instruction set (must be 64, so values must be 2 and 0x3e) (error code: "EB")
    mov ah, 'B'
    cmp byte [KERNEL_LOADPOINT + 4], 0x2
    jne error
    cmp word [KERNEL_LOADPOINT + 18], 0x3e
    jne error

    ; endianess (must be little endian, so value must be 1) (error code: "EE")
    mov ah, 'E'
    cmp byte [KERNEL_LOADPOINT + 5], 0x1
    jne error

    ; elf version (must be 1) (error code: "EV")
    mov ah, 'V'
    cmp byte [KERNEL_LOADPOINT + 0x0006], 0x1
    jne error

    ; Now lets trust it's actually real and valid elf file

    ; kernel entry position must be correct
    ; (error code : "Ep")
    mov ah, 'p'
    cmp qword [KERNEL_LOADPOINT + 24], KERNEL_LOCATION
    jne error

    ; get how many sectors of kernel in the disk need to be loaded to memory
    ; size = elf_shoff + elf_shentsize * elf_shentnum, sectors = ( (size + 511) >> 9 )-1
    ; the first sector have been loaded at
    mov ax, [KERNEL_LOADPOINT + 58]
    mov bx, [KERNEL_LOADPOINT + 60] 
    mul ebx, eax
    mov rax, [KERNEL_LOADPOINT + 40]
    add rbx, rax, 
    add rbx, 0x1ff
    shl rbx, 9
    cmp rbx, 1
    jbe loaded

    sub rbx, 1


    
.loaded:
    ; Parse program headers
    ; http://wiki.osdev.org/ELF#Program_header
    mov ah, 'H'

    ; We know that program header size is 56 (=0x38) bytes
    ; still, lets check it:
    cmp word [KERNEL_LOADPOINT + 54], 0x38
    jne error


    ; program header table position
    mov rbx, qword [KERNEL_LOADPOINT + 32]
    add rbx, KERNEL_LOADPOINT ; now rbx points to first program header

    ; length of program header table
    mov rcx, 0
    mov cx, [KERNEL_LOADPOINT + 56]

    mov ah, '_'
    ; loop through headers
.loop_headers:
    ; First, lets check that this segment should be loaded

    cmp dword [rbx], 1 ; load: this is important
    jne .next   ; if not important: continue

    push rcx

    mov rsi, [rbx + 8]
    add rsi, KERNEL_LOADPOINT  ; now points to begin of buffer we must copy

    ; rdi = p_vaddr
    mov rdi, [rbx + 16]

    ; rcx = p_memsz
    mov rcx, [rbx + 40]

    ; <1> clear p_memsz bytes at p_vaddr to 0
    push rdi

.loop_clear:
    mov byte [rdi], 0
    inc rdi
    loop .loop_clear
    pop rdi
    ; </1>

    ; rcx = p_filesz
    mov rcx, [rbx + 32]

    ; <2> copy p_filesz bytes from p_offset to p_vaddr
    ; uses: rsi, rdi, rcx
    rep movsb
    ; </2>

    pop rcx
.next:
    add rbx, 0x38   ; skip entry (0x38 is entry size)
    loop .loop_headers

    mov ah, '-'

    ; ELF relocation done
.over:

    ; looks good, going to jump to kernel entry
    ; prints green "JK" for "Jump to Kernel"
    mov dword [0xb8000 + 80*4], 0x2f6b2f6a

    jmp KERNEL_LOCATION ; jump to kernel

ata_lab_mode:
    pushfq
    and rax, 0x0FFFFFFF
    push rax
    push rbx
    push rcx
    push rdx
    push rdi

    mov rbx, rax         ; Save LBA in RBX

    mov edx, 0x01F6      ; Port to send drive and bit 24 - 27 of LBA
    shr eax, 24          ; Get bit 24 - 27 in al
    or al, 11100000b     ; Set bit 6 in al for LBA mode
    out dx, al

    mov edx, 0x01F2      ; Port to send number of sectors
    mov al, cl           ; Get number of sectors from CL
    out dx, al

    mov edx, 0x1F3       ; Port to send bit 0 - 7 of LBA
    mov eax, ebx         ; Get LBA from EBX
    out dx, al

    mov edx, 0x1F4       ; Port to send bit 8 - 15 of LBA
    mov eax, ebx         ; Get LBA from EBX
    shr eax, 8           ; Get bit 8 - 15 in AL
    out dx, al


    mov edx, 0x1F5       ; Port to send bit 16 - 23 of LBA
    mov eax, ebx         ; Get LBA from EBX
    shr eax, 16          ; Get bit 16 - 23 in AL
    out dx, al

    mov edx, 0x1F7       ; Command port
    mov al, 0x20         ; Read with retry.
    out dx, al

.still_going:  in al, dx
    test al, 8           ; the sector buffer requires servicing.
    jz .still_going      ; until the sector buffer is ready.

    mov rax, 256         ; to read 256 words = 1 sector
    xor bx, bx
    mov bl, cl           ; read CL sectors
    mul bx
    mov rcx, rax         ; RCX is counter for INSW
    mov rdx, 0x1F0       ; Data port, in and out
    rep insw             ; in to [RDI]

    pop rdi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    popfq
    ret
done:
    mov dword [0xb8000], 0x2f4b2f4f
    hlt

error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov dword [0xb800a], 0x4f204f20
    mov byte  [0xb800a], al
    mov byte  [0xb800c], ah
    hlt

kernel_sectors: dw 0
times (0x200-($-$$)) db 0 ; fill a sector 