//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
// use crate::task::TaskStatus;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    /// 将一个任务加入队尾
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        // task_inner.strid += task_inner.pass;

        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    /// 从队头中取出一个任务来执行
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // self.ready_queue.pop_front()
        // let queue: VecDeque<Arc<TaskControlBlock>> = 
        //     self.ready_queue.drain(..).collect();


        let mut fetch_task: Option<Arc<TaskControlBlock>> = None;
        let mut min = 2000;
        for task in self.ready_queue.iter() {
            let task_inner = task.inner_exclusive_access();
                if task_inner.strid < min {
                    min = task_inner.strid;
                    fetch_task = Some(task.clone());
                }
        }
        if let Some(fetch_task) = &fetch_task {
            // 获取选定任务的 PID（假设 PID 是一个字段）
            let clone_pid = fetch_task.pid.0;
            // 获取选定任务的内部可变引用
            let mut inner = fetch_task.inner_exclusive_access();
            // 更新任务的步幅
            inner.strid += inner.pass;
    
            // 从就绪队列中移除具有相同 PID 的任务
            self.ready_queue.retain(|x| x.pid.0 != clone_pid);
        }
        
        fetch_task
        
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
