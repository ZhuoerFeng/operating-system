const FD_STDOUT: usize = 1;
use crate::loader::in_user_stack;
use crate::task::{in_app, get_cur_app};

pub fn sys_write(fd: usize, buf: *const u8, len: usize, ptr: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let a = buf as usize;
            let b = get_cur_app();
            if !in_user_stack(b, ptr, a, len) || !in_app(a, len) {
                -1
            } else {
                let slice = unsafe { core::slice::from_raw_parts(buf, len) };
                let str = core::str::from_utf8(slice).unwrap();
                print!("{}", str);
                len as isize
            }
            
        },
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}