const FD_STDOUT: usize = 1;

use crate::batch::in_valid_space;

pub fn sys_write(fd: usize, buf: *const u8, len: usize, ptr : usize) -> isize {
    match fd {
        FD_STDOUT => {
            let addr = buf as usize;
            if !in_valid_space(addr, len, ptr) {
                -1
            } else {
                let slice = unsafe { core::slice::from_raw_parts(buf, len) };
                let str = core::str::from_utf8(slice).unwrap();
                print!("{}", str);
                len as isize
            }
        },
        _ => {
            // panic!("Unsupported fd in sys_write!");
            -1
        }
    }
}