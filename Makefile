# Building args
TARGET 			:= riscv64imac-unknown-none-elf
MODE 			:= debug
KERNEL_ELF 		:= target/$(TARGET)/$(MODE)/oshit_kernel
KERNEL_BIN 		:= kernel.bin
KERNEL_SYM 		:= kernel.sym
KERNEL_ASM		:= kernel.asm
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
FS_IMG 			:= ../fs.img

ifeq ($(BUILT_IN_PROC0), y)
	FEATURES += built_in_proc0
endif

# KERNEL ENTRY
ifeq ($(BOARD), qemu)
	KERNEL_ENTRY_PA := 0x80200000
else ifeq ($(BOARD), k210)
	KERNEL_ENTRY_PA := 0x80020000
endif

build: env $(KERNEL_BIN)

all: $(KERNEL_BIN)

env:
	rustup component add rust-src
	rustup component add llvm-tools-preview
	cargo install cargo-binutils
	rustup target add riscv64imac-unknown-none-elf

$(KERNEL_BIN): kernel
	$(OBJDUMP) -t $(KERNEL_ELF) | sed '1,/SYMBOL TABLE/d; s/ .* / /; /^$$/d'  | sort > $(KERNEL_SYM)
	$(OBJDUMP) -S $(KERNEL_ELF) > $(KERNEL_ASM)
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel:
	@cp src/linker_$(BOARD).ld src/linker.ld
ifeq ($(MODE), debug)
	cargo build --features "$(FEATURES)"
else
	cargo build --release --features "$(FEATURES)"
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
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-drive file=$(FS_IMG),if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0
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
			-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)\
			-drive file=$(FS_IMG),if=none,format=raw,id=x0 \
			-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0
			
.PHONY: build env kernel clean disasm run debug all