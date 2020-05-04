arch ?= x86_64
os_name = g8os
img := build/g8os-$(arch).img

linker_script := linker.ld
asm_boot_src := $(wildcard boot/$(arch)/stage*.asm)
asm_boot_obj := $(patsubst boot/$(arch)/stage%.asm, \
	build/boot/$(arch)/stage%.bin, $(asm_boot_src))
kernel_obj := build/$(arch)/kernel_strip.elf

.PHONY: all clean init qemu

all: $(img)

clean:
	@rm -r build

init:
	@mkdir -p build/boot/$(arch)

# compile assembly files
build/boot/$(arch)/stage%.bin: boot/$(arch)/stage%.asm
	@mkdir -p $(shell dirname $@)
	nasm -f bin -o $@ $<

 $(img): $(asm_boot_obj) $(kernel_obj)
	dd if=/dev/zero of=$(img) bs=65535 conv=notrunc count=64
	dd of=$(img) if=build/boot/$(arch)/stage0.bin bs=512 conv=notrunc seek=0 count=1
	dd of=$(img) if=build/boot/$(arch)/stage1.bin bs=512 conv=notrunc seek=1 count=1
	dd of=$(img) if=build/boot/$(arch)/stage2.bin bs=512 conv=notrunc seek=2 count=1

qemu: $(img)
	qemu-system-x86_64 -d int -m 4G -no-reboot -drive file=${img},format=raw,if=ide -monitor stdio


