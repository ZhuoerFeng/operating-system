use crate::task::{
    suspend_current_and_run_next,
    exit_current_and_run_next,
    set_current_priority,
};

use crate::timer::get_time_ms;


pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time(ptr : usize) -> isize {
    unsafe {
        let tmp = ptr as *mut (usize, usize);
        (*tmp).0 = get_time_ms() / 1000;
        (*tmp).1 = (get_time_ms() % 1000) * 1000;
    }
    0
}

pub fn sys_set_priority(prio : isize) -> isize {
    if prio >= 2 {
        set_current_priority(prio);
        prio
    } else {
        -1
    }
}