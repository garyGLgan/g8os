arch ?= x86_64
os_name = g8os
img := build/g8os-$(arch).img

linker_script := src/linker.ld
asm_boot_src := $(wildcard boot/$(arch)/stage*.asm)
asm_boot_obj := $(patsubst boot/$(arch)/stage%.asm, \
	build/boot/$(arch)/stage%.bin, $(asm_boot_src))
kernel_lib = target/$(arch)-$(os_name)/release/lib$(os_name).a
kernel_entry_src = src/entry.asm
kernel_entry_lib = build/boot/$(arch)/entry.o
kernel_linked_elf = build/boot/$(arch)/kernel_ori.elf
kernel_stripped_elf := build/boot/$(arch)/kernel_strip.elf

.PHONY: all clean init qemu

all: $(img)

clean:
	@rm -r build

$(kernel_entry_lib): $(kernel_entry_src)
	nasm -f elf64 -o $(kernel_entry_lib) $(kernel_entry_src)

$(kernel_lib):
	cargo xbuild --target $(arch)-$(os_name).json --release

$(kernel_stripped_elf): $(kernel_lib) $(kernel_entry_lib)
	ld -T $(linker_script) -o $(kernel_linked_elf) $(kernel_entry_lib) $(kernel_lib)
	strip -o $(kernel_stripped_elf) $(kernel_linked_elf)

init:
	@mkdir -p build/boot/$(arch)

# compile assembly files
build/boot/$(arch)/stage%.bin: boot/$(arch)/stage%.asm
	@mkdir -p $(shell dirname $@)
	nasm -f bin -o $@ $<


 $(img): $(asm_boot_obj) $(kernel_obj) $(kernel_stripped_elf)
	dd if=/dev/zero of=$(img) bs=65535 conv=notrunc count=64
	dd of=$(img) if=build/boot/$(arch)/stage0.bin bs=512 conv=notrunc seek=0 count=1
	dd of=$(img) if=build/boot/$(arch)/stage1.bin bs=512 conv=notrunc seek=1 count=1
	dd of=$(img) if=build/boot/$(arch)/stage2.bin bs=512 conv=notrunc seek=2 count=1
	dd of=$(img) if=$(kernel_stripped_elf) bs=512 conv=notrunc seek=3

qemu: $(img)
	qemu-system-x86_64 -d int -m 4G -no-reboot -drive file=${img},format=raw,if=ide -monitor stdio


