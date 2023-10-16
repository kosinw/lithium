# Lithium

Lithium is an experimental, library operating system designed to be a *lightweight* and *secure* execution environment for serverless functions. Leveraging the power of [unikernels](https://en.wikipedia.org/wiki/Unikernel), Lithium provides a new runtime providing networking, concurrency, and I/O primitives for writing serverless Rust functions while providing necessary isolation through the hypervisor.

## Usage

Build the image to `target/kernel.img`:

```sh
$ make image
```

Run kernel in QEMU:

```sh
$ make qemu
```

## Examples
- [ ] Hello World
- [ ] TCP Echo
- [ ] Image Resizing

## Required Features
- [x] Booting (bootloader)
- [ ] x86 CPU support (x86_64)
- [ ] Hardware Interrupts (pic8259)
- [ ] PS/2 Keyboard (pc-keyboard)
- [x] Serial Output (uart_16550)
- [ ] Paging (self-implemented)
- [ ] Heap Allocation (linked_list_allocator)
- [ ] RTL8139 Network Card (self-implemented)
- [ ] Syscall Interface (self-implemented)
- [ ] IPv4 network stack (smoltcp)
	- [ ] IP
	- [ ] TCP
	- [ ] UDP
	- [ ] DHCP
	- [ ] DNS
	- [ ] HTTP

## Stretch Features
- [ ] RTC Clock (self-implemented)
- [ ] Filesystem (self-implemented)
- [ ] Scheduling (self-implemented)

## License
Lithium is released under MIT licensing.

## Resources
- [Writing an OS in Rust](https://os.phil-opp.com/)
- [OSDev Wiki](https://wiki.osdev.org)