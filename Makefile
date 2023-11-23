# Remove built-in rules and variables.
override MAKEFlAGS += -rR

# Name of the final kernel image to run in QEMU.
override IMAGE 	 := bin/kernel.elf32
override IMAGE64 := bin/kernel.elf64

# Name of architecture we are compiling for.
override ARCH 	 := x86_64

override OBJCOPY := $(ARCH)-elf-objcopy
override QEMU    := qemu-system-$(ARCH)

override QEMUOPTS := -M q35
override QEMUOPTS += -cpu qemu64,fsgsbase,msr
override QEMUOPTS += -nographic
override QEMUOPTS += -m 2G
override QEMUOPTS += -kernel $(IMAGE)

.PHONY: all
all: $(IMAGE)

.PHONY: qemu
qemu: $(IMAGE)
	$(QEMU) $(QEMUOPTS)

.PHONY: kernel
kernel:
	mkdir -p "$$(dirname $(IMAGE64))"
	cargo build --manifest-path=kernel/Cargo.toml --target x86_64-unknown-none
	cp kernel/target/x86_64-unknown-none/debug/lithium $(IMAGE64)

$(IMAGE64): kernel

$(IMAGE): $(IMAGE64)
	$(OBJCOPY) --input-target=elf64-x86-64 --output-target=elf32-i386 $< $@

.PHONY: clean
clean:
	cargo clean --manifest-path=kernel/Cargo.toml
	rm -rf bin