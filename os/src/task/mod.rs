mod context;
mod switch;
mod task;

use crate::config::{MAX_APP_NUM, BIG_STRIDE, APP_BASE_ADDRESS, APP_SIZE_LIMIT};
use crate::loader::{get_num_app, init_app_cx};
use core::cell::RefCell;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

pub struct TaskManager {
    num_app: usize,
    inner: RefCell<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

unsafe impl Sync for TaskManager {}

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [
            TaskControlBlock { task_cx_ptr: 0, 
                                task_status: TaskStatus::UnInit,
                                task_prio: 16,
                                task_stride: 0,
                                task_start: 0,
                                flag: true,
                            };
            MAX_APP_NUM
        ];
        for i in 0..num_app {
            tasks[i].task_cx_ptr = init_app_cx(i) as * const _ as usize;
            tasks[i].task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: RefCell::new(TaskManagerInner {
                tasks,
                current_task: 0,
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
        let current = inner.current_task;
        current
    }

    fn set_current_priority(&self, prio : isize) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_prio = prio;
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
        if inner.tasks[current].task_start > 1001 {
            inner.tasks[current].task_status = TaskStatus::Exited;
        } else {
            inner.tasks[current].task_start += 1;
            inner.tasks[current].task_stride += BIG_STRIDE as isize / inner.tasks[current].task_prio as isize;
        }   
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.borrow_mut();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        let a = (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app);
            // .find(|id| {
            //     inner.tasks[*id].task_status == TaskStatus::Ready
            // })
        let mut small = 0xFFFF_FFFF_FFFF_FFFF as u64;
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

    fn in_app_room(&self, addr : usize, len: usize) -> bool {
        let inner = self.inner.borrow_mut();
        let current = inner.current_task;
        if addr >= APP_BASE_ADDRESS + APP_SIZE_LIMIT * current && addr + len < APP_BASE_ADDRESS + APP_SIZE_LIMIT * (current + 1) {
            true
        } else {
            false
        }
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

pub fn set_current_priority(prio : isize) {
    TASK_MANAGER.set_current_priority(prio);
}

pub fn in_app(addr : usize, len:usize) ->bool {
    TASK_MANAGER.in_app_room(addr, len)
}

pub fn get_cur_app() -> usize {
    TASK_MANAGER.get_cur_app()
}