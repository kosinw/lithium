# Remove random Makefile defaults.
override MAKEFLAGS += -rR

# Final target to build.
KERNEL := target/kernel
PROFILE ?= dev
PROFILE_DIR ?= debug

# Target architecture and toolchain.
ARCH := x86_64
AS := nasm
CC := $(ARCH)-elf-gcc
CARGO := cargo
OBJCOPY := $(ARCH)-elf-objcopy
QEMU := qemu-system-$(ARCH)

# Use "find" to glob all *.S, *.rs, and *.ld files in the tree and obtain the
# object and header dependency file names.
ASMFILES := $(shell cd kernel && find -L * -type f -name '*.S')
LINKERFILE := $(shell find -L * -type f -name '*.ld')
RUSTFILES := $(shell find -L * -type f -name '*.rs')
OBJFILES := $(addprefix target/obj/, $(ASMFILES:.S=.S.o))
HEADER_DEPS := $(addprefix target/obj/,$(ASMFILES:.S=.S.d))

# Options for running the QEMU emulator.
QEMUOPTS := -cpu qemu64,fsgsbase -m 512 -M q35

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

$(KERNEL): target/obj/kernel.a $(OBJFILES) $(LINKERFILE)
	$(CC) -z noexecstack -ffreestanding -O2 -nostdlib -T $(LINKERFILE) -o target/obj/kernel.elf $(OBJFILES) target/obj/kernel.a
	$(OBJCOPY) --input-target=elf64-x86-64 --output-target=elf32-i386 target/obj/kernel.elf $@

# Compilation rules for kernel.a
target/obj/kernel.a: $(RUSTFILES) Makefile
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