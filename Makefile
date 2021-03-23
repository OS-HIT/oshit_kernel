# Building args
TARGET 			:= riscv64gc-unknown-none-elf
MODE 			:= release
KERNEL_ELF 		:= target/$(TARGET)/$(MODE)/oshit_kernel
KERNEL_BIN 		:= $(KERNEL_ELF).bin
DISASM_TMP 		:= target/$(TARGET)/$(MODE)/asm
KERNEL_ENTRY_PA := 0x80200000
OBJDUMP 		:= rust-objdump --arch-name=riscv64
OBJCOPY 		:= rust-objcopy --binary-architecture=riscv64
DISASM 			?= -x
BOARD			?= qemu
FEATURES		?= board_qemu min_log_level_verbose

build: env $(KERNEL_BIN)

env:
	rustup component add rust-src
	rustup component add llvm-tools-preview
	cargo install cargo-binutils
	rustup target add riscv64gc-unknown-none-elf

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel:
	@cp src/linker_$(BOARD).ld src/linker.ld
	@cargo build --release --features "$(FEATURES)"
	@rm src/linker.ld

clean:
	@cargo clean

disasm: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) | less

run: build
	@qemu-system-riscv64 \
			-machine virt \
			-nographic \
			-bios default \
			-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)
			
.PHONY: build env kernel clean disasm run