%include "boot/x86_64/constants.asm"

[BITS 16]
[ORG 0x7c00]

boot:
    cli                         ; We do not want to be interrupted
    xor ax, ax                  ; 0 AX
    mov ds, ax                  ; Set Data Segment to 0
    mov es, ax                  ; Set Extra Segment to 0
    mov ss, ax                  ; Set Stack Segment to 0
    mov sp, ax                  ; Set Stack Pointer to 0

    mov sp, 0x7c00              ; initialize stack

    mov [BOOT_DRIVER], dl         ; save boot drive

    mov si, welcome
    call print_string

    ; get memory map
    mov al, 'M'                 ; set flag for print error
    call get_memory_map
    jc print_error              ; carry flag set on error

    ; test if LBA is enable
    ; https://wiki.osdev.org/ATA_in_x86_RealMode_(BIOS)#LBA_in_Extended_Mode
    clc                         
    mov al, 'R'                  ; set flag for print error
    mov ah, 0x41
    mov bx, 0x55AA
    mov dl, 0x80
    int 0x13
    jc print_error


    ; enable a20
    ; http://wiki.osdev.org/A20_Line
    in al, 0x92
    test al, 2
    jnz .done
    or al, 2
    and al, 0xFE
    out 0x92, al
.done:

    ; enter big unreal mode
    ; https://wiki.osdev.org/Unreal_mode#Big_Unreal_Mode
    push ds ; Save real mode
    lgdt [gdtinfo]

    mov  eax, cr0          ; switch to pmode by
    or al,1                ; set pmode bit
    mov  cr0, eax
    
    jmp $+2                ; tell 386/486 to not crash
    
    mov  bx, 0x08          ; select descriptor 1
    mov  ds, bx            ; 8h = 1000b
    
    and al,0xFE            ; back to realmode
    mov  cr0, eax          ; by toggling bit again
    
    pop ds  

    ; Load sectors
    ; Rest of the bootloader (da_packet already set up)
    mov ah, 0x42
    mov si, da_packet
    mov dl, [BOOT_DRIVER]
    int 0x13
    mov al, 'D'
    jc print_error


    mov si, done
    call print_string

    mov bh, 0
    mov ah, 2
    mov dx, 0xFFFF
    int 0x10

    lgdt [gdtr32]
    lidt [idtr32]

    mov eax, cr0
    or eax, 1
    mov cr0, eax

    jmp 0x08:0x7e00

    hlt
print_string:    ; prints E and one letter from al and terminates, (error in boot sector 0)
    lodsb        ; grab a byte from SI
 
    or al, al  ; logical or AL by itself
    jz .done   ; if the result is zero, get out
    
    mov ah, 0x0E
    int 0x10      ; otherwise, print out the character!
    
    jmp print_string
    
.done:
    ret

print_error:    ; prints E and one letter from al and terminates, (error in boot sector 0)
    push ax
        mov si, err
        call print_string
    pop ax
    mov ah, 0x0e
    int 0x10
    hlt

ALIGN 4
welcome db 'Welcom to G8 OS!', 0x0D, 0x0A, 0
err db 'Error: ', 0x0D, 0x0A, 0

done db 'Boot success', 0x0D, 0x0A, 0
da_packet:
    db 16               ; size of this packet (constant)
    db 0                ; reserved (always zero)
.count:
    dw (BOOTLOADER_SECTOR_COUNT - 1)    ; count (how many sectors)
.address:                               ; ^ (127 might be a limit here, still 0xFF on most BIOSes)
    dw STAGE_1_LOADPOINT ; offset (where)
.segment:
    dw 0                ; segment
.lba_low:
    dq 1                ; lba low (position on disk)
.lba_high:
    dq 0                ; lba high

; http://wiki.osdev.org/Detecting_Memory_(x86)#BIOS_Function:_INT_0x15.2C_EAX_.3D_0xE820
get_memory_map:
    mov di, (BOOT_TMP_MMAP_BUFFER+2)
	xor ebx, ebx               ; ebx must be 0 to start
	xor bp, bp                 ; keep an entry count in bp
	mov edx, 0x0534D4150       ; Place "SMAP" into edx
	mov eax, 0xe820
	mov [es:di + 20], dword 1  ; force a valid ACPI 3.X entry
	mov ecx, 24                ; ask for 24 bytes
	int 0x15
	jc short .failed           ; carry set on first call means "unsupported function"
	mov edx, 0x0534D4150       ; Some BIOSes apparently trash this register?
	cmp eax, edx               ; on success, eax must have been reset to "SMAP"
	jne short .failed
	test ebx, ebx              ; ebx = 0 implies list is only 1 entry long (worthless)
	je short .failed
	jmp short .jmpin
.e820lp:
	mov eax, 0xe820            ; eax, ecx get trashed on every int 0x15 call
	mov [es:di + 20], dword 1  ; force a valid ACPI 3.X entry
	mov ecx, 24                ; ask for 24 bytes again
	int 0x15
	jc short .e820f            ; carry set means "end of list already reached"
	mov edx, 0x0534D4150       ; repair potentially trashed register
.jmpin:
	jcxz .skipent              ; skip any 0 length entries
	cmp cl, 20                 ; got a 24 byte ACPI 3.X response?
	jbe short .notext
	test byte [es:di + 20], 1  ; if so: is the "ignore this data" bit clear?
	je short .skipent
.notext:
	mov ecx, [es:di + 8]       ; get lower uint32_t of memory region length
	or ecx, [es:di + 12]       ; "or" it with upper uint32_t to test for zero
	jz .skipent                ; if length uint64_t is 0, skip entry
	inc bp                     ; got a good entry: ++count, move to next storage spot
	add di, 24
.skipent:
	test ebx, ebx              ; if ebx resets to 0, list is complete
	jne short .e820lp
.e820f:
	mov [BOOT_TMP_MMAP_BUFFER], bp ; store the entry count just below the array
	clc                        ; there is "jc" on end of list to this point, so the carry must be cleared
	ret
.failed:
	stc	                       ; "function unsupported" error exit, set carry
	ret

gdtr32:
    dw gdt32_begin - gdt32_end - 1  ; size
    dd gdt32_begin                  ; offset

idtr32:
    dw 0
    dd 0

gdt32_begin:  ; from AMD64 system programming manual, page 132
    ; null entry
    dq 0
    ; code entry
    dw 0xffff       ; limit 0:15
    dw 0x0000       ; base 0:15
    db 0x00         ; base 16:23
    db 0b10011010   ; access P=1, DPL=00 (ring 0), S=1, TYPE=1010 (code, C=0, R=1 (readable), A=0)
    db 0b01001111   ; flags G=0, D/B=1, RESERVED=0, AVL=0 and limit 16:19 = 0b1111
    db 0x00         ; base 24:31
    ; data entry
    dw 0xffff       ; limit 0:15
    dw 0x0000       ; base 0:15
    db 0x00         ; base 16:23
    db 0b10010010   ; access P=1, DPL=00 (ring 0), S=1, TYPE=0010 (data, E=0, W=1 (writable), A=0)
    db 0b11001111   ; flags G=1 (limit marks 4 KiB blocks instead of 1 Byte), D/B=1, RESERVED=0, AVL=0 and limit 16:19 = 0b1111
    db 0x00         ; base 24:31
gdt32_end:
gdtinfo:
   dw gdt_end - gdt_begin - 1         ;last byte in table
   dd gdt_begin                       ;start of table
 
gdt_begin:  dd 0,0              ; entry 0 is always unused
flatdesc:   db 0xff, 0xff, 0, 0, 0, 10010010b, 11001111b, 0
gdt_end:

times (0x200 - 2) - ($ - $$) db 0
dw 0xaa55