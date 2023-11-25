#![allow(dead_code)]

pub const PAGESIZE: usize = 4096;       // page size

extern "C" {
    pub static KERNBASE: [u64; 0];          // kernel start address
    pub static KERNSTOP: [u64; 0];          // kernel end address
}