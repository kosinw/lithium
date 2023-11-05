#![no_std]
#![no_main]

use alloc::vec;

lithium::magic!(main);

fn main() {
    let v = vec![1, 2, 3, 4];
    println!("the one piece is real!, {:?}", v);
}
