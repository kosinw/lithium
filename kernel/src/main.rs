#![no_std]
#![no_main]

mod arch;

/// Entrypoint for Lithium kernel.
///
#[no_mangle]
pub unsafe extern "C" fn main() -> ! {
    loop {}
}

