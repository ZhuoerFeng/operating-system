use crate::task::{
    suspend_current_and_run_next,
    exit_current_and_run_next,
    current_task,
    current_user_token,
    add_task,

    set_current_priority,
    translate_va,
    insert_va,
    remove_va,
};
use crate::timer::get_time_ms;
use crate::mm::{
    translated_str,
    translated_refmut,
    translated_ref,
    VirtAddr,
    MapPermission,
};
use crate::fs::{
    open_file,
    OpenFlags,
};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use crate::config::{
    PAGE_SIZE, 
    PAGE_SIZE_BITS
};

pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
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

pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
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

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.acquire_inner_lock().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    let mut args_vec: Vec<String> = Vec::new();
    loop {
        let arg_str_ptr = *translated_ref(token, args);
        if arg_str_ptr == 0 {
            break;
        }
        args_vec.push(translated_str(token, arg_str_ptr as *const u8));
        unsafe { args = args.add(1); }
    }
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        let argc = args_vec.len();
        task.exec(all_data.as_slice(), args_vec);
        // return argc because cx.x[10] will be covered with it later
        argc as isize
    } else {
        -1
    }
}

pub fn sys_spawn(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    let mut args_vec: Vec<String> = Vec::new();
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let current_task = current_task().unwrap();
        let new_task = current_task.fork();
        let new_pid = new_task.pid.0;
        let trap_cx = new_task.acquire_inner_lock().get_trap_cx();
        trap_cx.x[10] = 0;
        new_task.exec(all_data.as_slice(), args_vec);
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();
    // find a child process

    // ---- hold current PCB lock
    let mut inner = task.acquire_inner_lock();
    if inner.children
        .iter()
        .find(|p| {pid == -1 || pid as usize == p.getpid()})
        .is_none() {
        return -1;
        // ---- release current PCB lock
    }
    let pair = inner.children
        .iter()
        .enumerate()
        .find(|(_, p)| {
            // ++++ temporarily hold child PCB lock
            p.acquire_inner_lock().is_zombie() && (pid == -1 || pid as usize == p.getpid())
            // ++++ release child PCB lock
        });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily hold child lock
        let exit_code = child.acquire_inner_lock().exit_code;
        // ++++ release child PCB lock
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB lock automatically
}