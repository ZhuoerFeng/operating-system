#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::{
    ch8::{forktest, hash},
    mmap,
    console::{getchar},
    write,
    read,
    STDIN,
    STDOUT,
};

#[no_mangle]
pub unsafe fn main(argc: usize, argv: &[&str]) -> i32 {
    let mut buf = [0u8; 1];
    loop {
        let size = read(STDIN, &mut buf) as usize;
        if size == 0 {
            break;
        }
        write(STDOUT, &buf[0..size]);
    }
    0
}