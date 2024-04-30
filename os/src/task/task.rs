//! Types related to task management

use super::TaskContext;
use crate::{config::MAX_SYSCALL_NUM, timer::get_time};


/// Task information
#[allow(dead_code)]


/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// The task info
    pub task_info: TaskInfo,
    /// the start time
    pub start_time: usize,
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}

/// The TaskInfo of a task
#[derive(Copy, Clone)]
pub struct TaskInfo{
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

impl TaskInfo{
    /// Create a new `TaskInfo`
    pub fn new() -> Self{
        Self{
            status: TaskStatus::Running,
            syscall_times: [0; MAX_SYSCALL_NUM],
            time: 0,
        }
    }
    /// Set the status of task
    pub fn set_status(&mut self, status: TaskStatus){
        self.status = status;
    }
    /// Set the init_time of task
    pub fn set_init_time(&mut self) {
        self.time = get_time();
    }
    ///Set the Gap time of task
    pub fn set_gap_time(&mut self, gap: usize) {
        self.time = gap;
    }
    ///add_syscall_times
    pub fn add_syscall_times(&mut self, syscall_id: usize){
        self.syscall_times[syscall_id] += 1;
    }
}




