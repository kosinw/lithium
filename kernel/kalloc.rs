use crate::kernel_println;

pub fn init(boot_info: u64) {
    kernel_println!("kalloc: page allocator starting...");
}