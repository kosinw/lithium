# Remove random Makefile defaults.
override MAKEFLAGS += -rR

# Final target to build.
KERNEL := target/kernel
PROFILE ?= dev
PROFILE_DIR ?= debug

# Target architecture and toolchain.
QEMU := qemu-system-x86_64

ARCH := x86_64-elf
AS := nasm
CC := $(ARCH)-gcc
CARGO := cargo
OBJCOPY := $(ARCH)-objcopy
OBJDUMP := $(ARCH)-objdump


# Use "find" to glob all *.S, *.rs, and *.ld files in the tree and obtain the
# object and header dependency file names.
ASMFILES := $(shell cd kernel && find -L * -type f -name '*.S')
LINKERFILE := $(shell find -L * -type f -name '*.ld')
RUSTFILES := $(shell find -L * -type f -name '*.rs')
OBJFILES := $(addprefix target/obj/, $(ASMFILES:.S=.S.o))
HEADER_DEPS := $(addprefix target/obj/,$(ASMFILES:.S=.S.d))

# Options for running the QEMU emulator.
QEMUOPTS := -M microvm -no-reboot -serial mon:stdio
QEMUOPTS += -nographic -cpu qemu64,fsgsbase,msr -m 512M

# Default target.
.PHONY: all
all: kernel

# Run qemu
qemu: $(KERNEL)
	$(QEMU) $(QEMUOPTS) -kernel $(KERNEL)

qemu-gdb: $(KERNEL)
	$(QEMU) -S -s $(QEMUOPTS) -kernel $(KERNEL)

# Build the kernel.
.PHONY: kernel
kernel: $(KERNEL)

$(KERNEL): target/obj/kernel.o $(OBJFILES) $(LINKERFILE)
	$(CC) -z noexecstack -ffreestanding -O2 -nostdlib -T $(LINKERFILE) -o target/obj/kernel.elf $(OBJFILES) target/obj/kernel.o
	$(OBJDUMP) -M intel -S target/obj/kernel.elf > target/kernel.S
	$(OBJDUMP) -t target/obj/kernel.elf | sed '1,/SYMBOL TABLE/d; s/ .* / /; /^$$/d' > target/kernel.sym
	$(OBJCOPY) --input-target=elf64-x86-64 --output-target=elf32-i386 target/obj/kernel.elf $@

# Compilation rules for kernel.o
target/obj/kernel.o: $(RUSTFILES) Makefile
	mkdir -p "$$(dirname $@)"
	$(CARGO) build \
	--profile $(PROFILE) \
	-Z build-std-features=compiler-builtins-mem \
	-Z build-std=alloc,core,compiler_builtins \
	--target cfg/lithium.json
	cp target/lithium/$(PROFILE_DIR)/libkernel.a $@

# Compilation rules for *.S files.
target/obj/%.S.o: kernel/%.S Makefile
	mkdir -p "$$(dirname $@)"
	$(AS) -f elf64 -Wall -F dwarf -g $< -o $@

# Clean up folders
.PHONY: clean
clean:
	rm -rf target

# Include header deps.
-include $(HEADER_DEPS)