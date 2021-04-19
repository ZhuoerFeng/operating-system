#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{getpid, mail_read, mail_write};

const BUF_LEN: usize = 256;

/// 测试邮箱基本功能，输出　mail0 test OK! 就算正确。

#[no_mangle]
fn main() -> i32 {
    let pid = getpid();
    let buffer0 = ['a' as u8; 27];
    println!("1");
    assert_eq!(mail_write(pid as usize, &buffer0), 27);
    println!("2");
    let buffer1 = ['b' as u8; BUF_LEN + 1];
    assert_eq!(mail_write(pid as usize, &buffer1), BUF_LEN as isize);
    println!("3");
    let mut buf = [0u8; BUF_LEN];
    assert_eq!(mail_read(&mut buf), 27);
    println!("4");
    assert_eq!(buf[..27], buffer0);
    println!("5");
    assert_eq!(mail_read(&mut buf[..27]), 27);
    println!("6");
    assert_eq!(buf[..27], buffer1[..27]);
    println!("7");
    println!("mail0 test OK!");
    0
}
