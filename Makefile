MEMORY		:= 	128M
MODE		:= 	debug
KERNEL 		:= 	target/x86_64-lithium/$(MODE)/bootimage-lithium.bin
BOOTIMAGE	:= 	target/kernel.img
QEMU		:= 	qemu-system-x86_64
QEMU_OPTS	:=	-nographic -m $(MEMORY) -machine pc \
			   	-drive file=$(BOOTIMAGE),format=raw
				-device isa-debug-exit,iobase=0xf4,iosize=0x04


.PHONY: clean
clean:
	cargo clean

.PHONY: image
image: $(BOOTIMAGE)

.PHONY: $(KERNEL)
$(KERNEL):
	cargo bootimage

$(BOOTIMAGE): $(KERNEL)
	dd conv=notrunc if=$(KERNEL) of=$(BOOTIMAGE)

.PHONY: qemu
qemu: $(BOOTIMAGE)
	$(QEMU) $(QEMU_OPTS)