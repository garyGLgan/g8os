%include "boot/x86_64/constants.asm"

[BITS 32]
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

    mov dword [0xb8000], 0x2f332f30

    mov ah, 0x42
    mov si, da_packet
    mov dl, [BOOT_DRIVER]
    int 0x13
    mov al, 'D'
    jc print_error

    mov al, 'E'

    ; magic number 0x7f+'ELF'
    mov ah, '!'
    cmp dword [KERNEL_TMP_LOADPOINT], 0x464c457f
    jne error

    ; bitness and instruction set (must be 64, so values must be 2 and 0x3e) (error code: "EB")
    mov ah, 'B'
    cmp byte [KERNEL_TMP_LOADPOINT + 4], 0x2
    jne error
    cmp word [KERNEL_TMP_LOADPOINT + 18], 0x3e
    jne error

    ; endianess (must be little endian, so value must be 1) (error code: "EE")
    mov ah, 'E'
    cmp byte [KERNEL_TMP_LOADPOINT + 5], 0x1
    jne error

    ; elf version (must be 1) (error code: "EV")
    mov ah, 'V'
    cmp byte [KERNEL_TMP_LOADPOINT + 0x0006], 0x1
    jne error

    ; Now lets trust it's actually real and valid elf file

    ; kernel entry position must be correct
    ; (error code : "Ep")
    mov ah, 'p'
    cmp qword [KERNEL_TMP_LOADPOINT + 24], KERNEL_LOCATION
    jne error

    ; get how many sectors of kernel in the disk need to be loaded to memory
    ; size = elf_shoff + elf_shentsize * elf_shentnum, sectors = ( (size + 511) >> 9 )-1
    ; the first sector have been loaded at
    mov eax, [KERNEL_TMP_LOADPOINT + 58] 
    mul eax, [KERNEL_TMP_LOADPOINT + 60] 
    add eax, [KERNEL_TMP_LOADPOINT + 40]
    add eax, 0x1ff
    shl eax, 9
    sub eax, 1

    ; if the elf only in one sector, already loaded
    cmp eax, 0
    jz .readed
    mov dw [da_packet.count], eax
    mov eax, KERNEL_TMP_LOADPOINT
    add eax, 0x200
    mov dw [da_packet.address], eax
    mov dw [da_packet.low], 4

    ; loaded the kernel
    mov ah, 0x42
    mov si, da_packet
    mov dl, [bootdrive]
    int 0x13
    mov al, 'D'
    jc print_error

.loaded:
    ; Parse program headers
    ; http://wiki.osdev.org/ELF#Program_header
    mov ah, 'H'

    ; We know that program header size is 56 (=0x38) bytes
    ; still, lets check it:
    cmp word [KERNEL_TMP_LOADPOINT + 54], 0x38
    jne error


    ; program header table position
    mov rbx, qword [KERNEL_TMP_LOADPOINT + 32]
    add rbx, KERNEL_TMP_LOADPOINT ; now rbx points to first program header

    ; length of program header table
    mov rcx, 0
    mov cx, [KERNEL_TMP_LOADPOINT + 56]

    mov ah, '_'
    ; loop through headers
.loop_headers:
    ; First, lets check that this segment should be loaded

    cmp dword [rbx], 1 ; load: this is important
    jne .next   ; if not important: continue

    push rcx

    mov rsi, [rbx + 8]
    add rsi, BOOT_KERNEL_LOADPOINT  ; now points to begin of buffer we must copy

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
da_packet:
    db 16               ; size of this packet (constant)
    db 0                ; reserved (always zero)
.count:
    dw 1                ; count (how many sectors)
.address:                               ; ^ (127 might be a limit here, still 0xFF on most BIOSes)
    dw KERNEL_TMP_LOADPOINT ; offset (where)
.segment:
    dw 0                ; segment
.lba_low:
    dq 3                ; lba low (position on disk)
.lba_high:
    dq 0    

times (0x200-($-$$)) db 0 ; fill a sector 