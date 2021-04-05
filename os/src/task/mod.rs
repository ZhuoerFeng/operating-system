mod context;
mod switch;
mod task;

use crate::config::{PAGE_SIZE_BITS, BIG_STRIDE};
use crate::loader::{get_num_app, get_app_data};
use crate::trap::TrapContext;
use crate::mm::{MemorySet, VirtAddr, VirtPageNum, MapPermission};
use core::cell::RefCell;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};
use alloc::vec::Vec;
use crate::timer::get_time_ms;

pub use context::TaskContext;

pub struct TaskManager {
    num_app: usize,
    inner: RefCell<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
    last_t: usize,
}

unsafe impl Sync for TaskManager {}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(
                get_app_data(i),
                i,
            ));
        }
        TaskManager {
            num_app,
            inner: RefCell::new(TaskManagerInner {
                tasks,
                current_task: 0,
                last_t: 0,
            }),
        }
    };
}

impl TaskManager {
    fn run_first_task(&self) {
        self.inner.borrow_mut().tasks[0].task_status = TaskStatus::Running;
        let next_task_cx_ptr2 = self.inner.borrow().tasks[0].get_task_cx_ptr2();
        let _unused: usize = 0;
        unsafe {
            __switch(
                &_unused as *const _,
                next_task_cx_ptr2,
            );
        }
    }

    fn get_cur_app(&self) -> usize {
        let inner = self.inner.borrow_mut();
        inner.current_task
    }

    fn set_current_priority(&self, prio: isize) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_prio = prio;
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
        // update stride
        let new_t = get_time_ms();
        inner.tasks[current].task_start += new_t as isize - inner.last_t as isize;
        inner.last_t = new_t;
        inner.tasks[current].task_stride += (BIG_STRIDE as isize) / (inner.tasks[current].task_prio as isize);  
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        // use stride algorithm
        let a = (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app);
        let mut small = (BIG_STRIDE * BIG_STRIDE) as u64;
        let mut n = -1 as isize;
        for item in a {
            if (inner.tasks[item].task_status == TaskStatus::Ready) && ((inner.tasks[item].task_stride as u64) < small) {
                n = item as isize;
                small = inner.tasks[item].task_stride as u64;
            }
        }
        if n == -1 { 
            None
        } else {
            Some(n as usize)
        }
    }

    fn get_current_token(&self) -> usize {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        inner.tasks[current].get_user_token()
    }

    fn get_current_trap_cx(&self) -> &mut TrapContext {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        inner.tasks[current].get_trap_cx()
    }

    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.borrow_mut();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr2 = inner.tasks[current].get_task_cx_ptr2();
            let next_task_cx_ptr2 = inner.tasks[next].get_task_cx_ptr2();
            core::mem::drop(inner);
            unsafe {
                __switch(
                    current_task_cx_ptr2,
                    next_task_cx_ptr2,
                );
            }
        } else {
            panic!("All applications completed!");
        }
    }

    fn translate_va(&self, ptr: usize) -> usize {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        (usize::from(inner.tasks[current].memory_set.translate(VirtAddr::from(ptr).floor()).unwrap().ppn()) << PAGE_SIZE_BITS) + VirtAddr::from(ptr).page_offset()
    }

    // true for no conflict
    // false for conflict
    fn check_conflict(&self, v_start: VirtAddr, v_end: VirtAddr) -> bool {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        let memory_set = &inner.tasks[current].memory_set.areas;
        let mut disjoint = true;
        for mm in memory_set {
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
        disjoint
    }

    // assume no conflicts
    fn insert_va(&self, v_start: VirtAddr, v_end: VirtAddr, permission: MapPermission) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].memory_set.insert_framed_area(
            v_start, 
            v_end,
            permission
        );
    }

    // true for matched
    // false for expetion(exist not included)
    fn find_matching(&self, v_start: VirtAddr, v_end: VirtAddr) -> bool {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        let memory_set = &inner.tasks[current].memory_set.areas;
        let mut matching = false;
        for mm in memory_set {
            let range = mm.vpn_range;
            if range.get_start() == v_start.floor() && range.get_end() == v_end.floor() {
                matching = true;
                break;
            }
        }
        matching
    }

    fn dismatch(&self, v_start: VirtAddr, v_end: VirtAddr) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].memory_set.remove_framed_area(v_start, v_end);
    }
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

pub fn set_current_priority(prio: isize) {
    TASK_MANAGER.set_current_priority(prio);
}

pub fn translate_va(ptr: usize) -> usize {
    TASK_MANAGER.translate_va(ptr)
}

pub fn get_cur_app() -> usize {
    TASK_MANAGER.get_cur_app()
}

pub enum IntervalRelationship {
    Same,
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

pub fn insert_va(v_start: VirtAddr, v_end: VirtAddr, permission: MapPermission) -> bool {
    if TASK_MANAGER.check_conflict(v_start, v_end) { // no conflict
        TASK_MANAGER.insert_va(v_start, v_end, permission);
        true
    } else { // conflict
        false
    }
}

pub fn remove_va(v_start: VirtAddr, v_end: VirtAddr) -> bool {
    if TASK_MANAGER.find_matching(v_start, v_end) { // matched
        TASK_MANAGER.dismatch(v_start, v_end);
        true
    } else {
        false
    }
}