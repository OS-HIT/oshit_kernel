# Building args
TARGET 			:= riscv64gc-unknown-none-elf
MODE 			:= debug
KERNEL_ELF 		:= target/$(TARGET)/$(MODE)/oshit_kernel
KERNEL_BIN 		:= $(KERNEL_ELF).bin
DISASM_TMP 		:= target/$(TARGET)/$(MODE)/asm
OBJDUMP 		:= rust-objdump --arch-name=riscv64
OBJCOPY 		:= rust-objcopy --binary-architecture=riscv64
DISASM 			?= -x
BOARD			?= qemu
LOG_LVL			?= debug
FEATURES		?= board_$(BOARD) min_log_level_$(LOG_LVL)
K210-SERIALPORT	:= /dev/ttyUSB0
K210-BURNER 	:= ../kflash.py/kflash.py
BOOTLOADER 		:= ../bootloader/rustsbi-$(BOARD).bin
K210_BOOTLOADER_SIZE := 131072
PY 				:= python3

# KERNEL ENTRY
ifeq ($(BOARD), qemu)
	KERNEL_ENTRY_PA := 0x80200000
else ifeq ($(BOARD), k210)
	KERNEL_ENTRY_PA := 0x80020000
endif



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
ifeq ($(MODE), debug)
	cargo build -vv --features "$(FEATURES)"
else
	cargo build -vv --release --features "$(FEATURES)"
endif
	@rm src/linker.ld

clean:
	@cargo clean

disasm: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) | less

run: build
ifeq ($(BOARD),qemu)
	qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER)\
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)
else
	@cp $(BOOTLOADER) $(BOOTLOADER).copy
	@dd if=$(KERNEL_BIN) of=$(BOOTLOADER).copy bs=$(K210_BOOTLOADER_SIZE) seek=1
	@mv $(BOOTLOADER).copy $(KERNEL_BIN)
	# @sudo chmod 777 $(K210-SERIALPORT)
	$(PY) $(K210-BURNER) -p $(K210-SERIALPORT) -b 1500000 $(KERNEL_BIN)
	$(PY) -m serial.tools.miniterm --eol LF --dtr 0 --rts 0 --filter direct $(K210-SERIALPORT) 115200
endif

debug: build
	@qemu-system-riscv64 \
			-s -S \
			-machine virt \
			-nographic \
			-bios $(BOOTLOADER)\
			-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)
			
.PHONY: build env kernel clean disasm run debug