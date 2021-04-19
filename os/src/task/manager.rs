use super::TaskControlBlock;
use alloc::collections::{VecDeque, BTreeMap};
use alloc::sync::Arc;
use spin::Mutex;
use lazy_static::*;
use crate::fs::{MailBox};

pub use super::processor::{
    run_tasks,
    current_task,
    current_user_token,
    current_trap_cx,
    take_current_task,
    schedule,
};

pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
    map: BTreeMap<usize, Arc<MailBox>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    pub fn new() -> Self {
        Self { 
            ready_queue: VecDeque::new(), 
            map: BTreeMap::new(),
        }
    }
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    pub fn find_mailbox(&mut self, pid: usize) -> Arc<MailBox> {
        if self.map.contains_key(&pid)  {
            self.map.get(&pid).unwrap().clone()
        } else {
            self.map.insert(pid, Arc::new(MailBox::new()));
            self.map.get(&pid).unwrap().clone()
        }
    }
    pub fn clear_mailbox(&mut self, pid: usize) -> bool {
        if self.map.contains_key(&pid) {
            self.map.remove(&pid);
            true
        } else {
            false
        }
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.lock().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.lock().fetch()
}

pub fn find_mailbox(pid: usize) -> Arc<MailBox> {
    TASK_MANAGER.lock().find_mailbox(pid)
}

pub fn clear_mailbox(pid: usize) -> bool {
    TASK_MANAGER.lock().clear_mailbox(pid)
}


