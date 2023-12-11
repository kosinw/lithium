#![no_std]
#![feature(panic_info_message)]

mod console;
mod cpu;
mod memory;
mod multiboot;
mod runtime;

/// Entrypoint for lithium kernel.
///
/// The library operating system calls initialization routines in this function
/// related to memory management and drivers before transferring control to the
/// linked unikernel application.
#[no_mangle]
pub extern "C" fn kernel_main(mbi_ptr: *const multiboot::MultibootInformation) {
    cpu::init(0); // per-cpu kernel data structure
    console::init(); // console driver

    let mut allocator = memory::PhysicalAllocator::new();

    allocator.reserve(x86_64::PhysAddr::new(0x1000000), 0x80000, 4096);

    log!("allocator: {:?}", allocator);

    let frame = allocator.allocate(15).unwrap();

    log!("allocator: {:?}", allocator);
}
