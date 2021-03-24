const FD_STDOUT: usize = 1;
use crate::loader::in_user_stack;
use crate::task::{in_app, get_cur_app};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let addr = buf as usize;
            let cur = get_cur_app();
            if in_app(addr, len) || in_user_stack(cur, addr, len) {
                let slice = unsafe { core::slice::from_raw_parts(buf, len) };
                let str = core::str::from_utf8(slice).unwrap();
                print!("{}", str);
                len as isize   
            } else {
                // println!("cur app = {}, cur addr = {}, len = {}", cur, addr, len);
                -1
            }
        },
        _ => {
            -1
        }
    }
}