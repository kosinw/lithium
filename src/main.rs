#![no_std]
#![no_main]

lithium::magic!(main);
fn main() {
    x86_64::instructions::interrupts::int3();
}

