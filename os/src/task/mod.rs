mod context;
mod switch;
mod task;
mod manager;
mod processor;
mod pid;

use crate::loader::{get_app_data_by_name};
use crate::config::{PAGE_SIZE_BITS};
use crate::mm::{VirtAddr, VirtPageNum, MapPermission};
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};
use alloc::sync::Arc;
use manager::fetch_task;

pub use context::TaskContext;
pub use processor::{
    run_tasks,
    current_task,
    current_user_token,
    current_trap_cx,
    take_current_task,
    schedule,
};
pub use manager::{add_task, find_mailbox, clear_mailbox};
pub use pid::{PidHandle, pid_alloc, KernelStack};

pub enum IntervalRelationship {
    Disjoint,
    BeIncluded,
    IncludeAtRight,
    IncludeAtLeft,
    IncludeAtMiddle,
    Unknown,
}

// [self_vpn_1, self_vpn_r), [vpn_1, vpn_r)
// my: vpn   match_target: self_vpn
pub fn check_relationship(self_vpn_1: VirtPageNum, self_vpn_r: VirtPageNum, vpn_1: VirtPageNum, vpn_r: VirtPageNum) -> IntervalRelationship {
    if vpn_r <= self_vpn_1 || vpn_1 >= self_vpn_r {
        IntervalRelationship::Disjoint
    } else if self_vpn_1 <= vpn_1 && vpn_r <= self_vpn_r {
        IntervalRelationship::BeIncluded
    } else if self_vpn_1 <= vpn_1 && vpn_1 < self_vpn_r && self_vpn_r < vpn_r {
        IntervalRelationship::IncludeAtRight
    } else if vpn_1 < self_vpn_1 && self_vpn_1 < vpn_r && vpn_r <= self_vpn_r {
        IntervalRelationship::IncludeAtLeft
    } else if vpn_1 <= self_vpn_1 && self_vpn_r <= vpn_r {
        IntervalRelationship::IncludeAtMiddle
    } else {
        IntervalRelationship::Unknown
    }
}

// true for no conflict
// false for conflict
pub fn check_conflict(v_start: VirtAddr, v_end: VirtAddr) -> bool {
    let task = current_task().unwrap();
    let task_inner = task.acquire_inner_lock();
    let mut disjoint = true;
    for mm in &task_inner.memory_set.areas {
        let range = mm.vpn_range;
        match check_relationship(range.get_start(), range.get_end(), v_start.floor(), v_end.floor()) {
            IntervalRelationship::Disjoint => {
                // do nothing
            }
            _ => {
                disjoint = false;
                break;
            }
        }
    }
    drop(task_inner);
    disjoint
}

pub fn check_match(v_start: VirtAddr, v_end: VirtAddr) -> bool {
    let task = current_task().unwrap();
    let task_inner = task.acquire_inner_lock();
    let mut matching = false;
    for mm in &task_inner.memory_set.areas {
        let range = mm.vpn_range;
        if range.get_start() == v_start.floor() && range.get_end() == v_end.floor() {
            matching = true;
            break;
        }
    }
    drop(task_inner);
    matching
}

pub fn set_current_priority(task_prio: isize) {
    let task = current_task().unwrap();
    let mut task_inner = task.acquire_inner_lock();
    task_inner.task_prio = task_prio;
    drop(task_inner);
}

pub fn translate_va(ptr: usize) -> usize {
    let task = current_task().unwrap();
    let task_inner = task.acquire_inner_lock();
    let res: usize = (usize::from(task_inner.memory_set.translate(VirtAddr::from(ptr).floor()).unwrap().ppn()) << PAGE_SIZE_BITS) + VirtAddr::from(ptr).page_offset();
    drop(task_inner);
    res
}

pub fn insert_va(v_start: VirtAddr, v_end: VirtAddr, permission: MapPermission) -> bool {
    if check_conflict(v_start, v_end) { // true for no conflict
        let task = current_task().unwrap();
        let mut task_inner = task.acquire_inner_lock();
        task_inner.memory_set.insert_framed_area(
            v_start, 
            v_end,
            permission
        );
        drop(task_inner);
        true
    } else {
        println!("Fing conlict in writter");
        false
    }
}

pub fn remove_va(v_start: VirtAddr, v_end: VirtAddr) -> bool {
    if check_match(v_start, v_end) { // matched
        let task = current_task().unwrap();
        let mut task_inner = task.acquire_inner_lock();
        task_inner.memory_set.remove_framed_area(v_start, v_end);
        drop(task_inner);
        true
    } else {
        false
    }
}

pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- hold current PCB lock
    let mut task_inner = task.acquire_inner_lock();
    let task_cx_ptr2 = task_inner.get_task_cx_ptr2();
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;

    // let mut new_t = get_time_ms();
    // task_inner.task_time = new_t as isize - inner.last_t;
    // task_inner.last_t = new_t;
    // task_inner.task_stride += (BIG_STRIDE as isize) / (inner.task_prio as isize);  

    drop(task_inner);
    // ---- release current PCB lock

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr2);
}

pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();
    let pid = task.pid.0;
    clear_mailbox(pid);
    // **** hold current PCB lock
    let mut inner = task.acquire_inner_lock();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;

    // Record exit code
    inner.exit_code = exit_code;

    // do not move to its parent but under initproc

    // ++++++ hold initproc PCB lock here
    {
        let mut initproc_inner = INITPROC.acquire_inner_lock();
        for child in inner.children.iter() {
            child.acquire_inner_lock().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB lock here

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB lock
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let _unused: usize = 0;
    schedule(&_unused as *const _);
}

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(
        TaskControlBlock::new(get_app_data_by_name("initproc").unwrap())
    );
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}
