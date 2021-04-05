use crate::task::{
    suspend_current_and_run_next,
    exit_current_and_run_next,
    set_current_priority,
    translate_va,
    TASK_MANAGER,
    get_cur_app,
    insert_va,
    remove_va,
};

use crate::config::{PAGE_SIZE, PAGE_SIZE_BITS};
use crate::timer::get_time_ms;
use crate::mm::{MemorySet, KERNEL_SPACE, VirtAddr, VirtPageNum, MapPermission, PageTableEntry};

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time(ptr: usize) -> isize {
    unsafe {
        let pa = translate_va(ptr);
        let tmp = pa as *mut (usize, usize);
        (*tmp).0 = get_time_ms() / 1000;
        (*tmp).1 = (get_time_ms() % 1000) * 1000;
    }
    0
}

pub fn sys_set_priority(prio: isize) -> isize {
    if prio >= 2 {
        set_current_priority(prio);
        prio
    } else {
        -1
    }
}

pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    if (port & !0x7) != 0 {
        // println!("port type error");
        return -1;
    }
    if (port & 0x7) == 0 {
        // println!("port has no meanings");
        return -1;
    }
    if len > (1 << 30) {
        // println!("TOO BIG");
        return -1;
    }
    if len == 0 {
        return 0 as isize
    }
    if start & (PAGE_SIZE -1) != 0 {
        // println!("Start address not align");
        return -1;
    }
    let mut flag = MapPermission::R | MapPermission::W | MapPermission::X | MapPermission::U;
    // println!("{:X}", flag & (MapPermission::R | MapPermission::W | MapPermission::X | MapPermission::U));
    if port&1==0 {
        flag ^= MapPermission::R;
    }
    if port&2==0 {
        flag ^= MapPermission::W;
    }
    if port&4==0 {
        flag ^= MapPermission::X;
    }
    let size = (((len - 1) >> PAGE_SIZE_BITS) + 1) << PAGE_SIZE_BITS;
    let v_start = VirtAddr::from(start);
    let v_end = VirtAddr::from(start + size);
    let cur_app = get_cur_app();
    if !insert_va(v_start, v_end, flag) {
        // println!("Interval conflict");
        return -1;
    }
    size as isize
} 

pub fn sys_munmap(start: usize, len: usize) -> isize {
    if start & (PAGE_SIZE -1) != 0 {
        // println!("Start address not align");
        return -1;
    }
    let size = (((len - 1) >> PAGE_SIZE_BITS) + 1) << PAGE_SIZE_BITS;
    let v_start = VirtAddr::from(start);
    let v_end = VirtAddr::from(start + size);
    if !remove_va(v_start, v_end) {
        // println!("Remove Error");
        return -1;
    }
    size as isize
}