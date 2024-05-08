//!Implementation of [`Processor`] and Intersection of control flow
//!
//! Here, the continuous operation of user apps in CPU is maintained,
//! the current running state of CPU is recorded,
//! and the replacement and transfer of control flow of different applications are executed.
//! 每个 Processor 都有一个 idle 控制流，它们运行在每个核各自的启动栈上，
//! 功能是尝试从任务管理器中选出一个任务来在当前核上执行。
//!  在内核初始化完毕之后，核通过调用 run_tasks 函数来进入 idle 控制流：
use super::__switch;
use super::{fetch_task, TaskStatus, TaskInfo};
use super::{TaskContext, TaskControlBlock};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;
use crate::timer::get_time_ms;
use crate::mm::{MapPermission,VirtAddr,VPNRange};

/// Processor management structure
pub struct Processor {
    /// The task currently executing on the current processor
    /// 当前处理器上正在执行的任务
    current: Option<Arc<TaskControlBlock>>,

    /// The basic control flow of each core, 
    /// helping to select and switch process
    /// 当前处理器上的 idle 控制流的任务上下文的地址。
    idle_task_cx: TaskContext,
}

impl Processor {
    ///Create an empty Processor
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    ///Get mutable reference to `idle_task_cx`
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }

    ///Get current task in moving semanteme
    /// 取出当前正在执行的任务
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        // Option::take 意味着 current 字段也变为 None
        self.current.take()
    }

    ///Get current task in cloning semanteme
    /// 返回当前执行的任务的一份拷贝
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        // let a = self.current.as_ref();
        self.current.as_ref().map(Arc::clone)
    }

    // 获取当前任务info
    fn get_current_task_info(&mut self) -> TaskInfo{
        let mut inner = self.current.as_mut().unwrap().inner_exclusive_access();
        // let current = inner.current_task;
        let gap = get_time_ms() - inner.start_time;
        inner.task_info.set_gap_time(gap);
        let status = inner.task_status.clone();
        inner.task_info.set_status(status);
        let current_task_info = inner.task_info.clone();
        // let mut current_task_info = inner.tasks[inner.current_task].task_info.clone();
        drop(inner);
        current_task_info
    }

    // 添加当前任务系统调用次数
    fn add_syscall_times(&mut self, syscall_id: usize) {
        let mut inner = self.current.as_mut().unwrap().inner_exclusive_access();
        inner.task_info.add_syscall_times(syscall_id);
        drop(inner);
    }

    // 为当前任务分配内存
    fn mmap(&mut self, start_vir_addr: VirtAddr, end_vir_addr: VirtAddr, port: usize) -> isize {
        let mut inner = self.current.as_mut().unwrap().inner_exclusive_access();
        if inner.
            memory_set
            .exist_some_range(VPNRange::new(start_vir_addr.floor().into(), end_vir_addr.ceil().into()))
            .is_some()
        {
            return -1;
        }
        let permission= MapPermission::from_bits_truncate((port<<1)as u8 | MapPermission::U.bits());
        inner.memory_set.
            insert_framed_area(start_vir_addr.floor().into(), end_vir_addr.ceil().into(), permission.into());
        drop(inner);
        0
    }

    // 为当前任务分配内存
    fn munmap(&mut self, start_vir_addr: VirtAddr, end_vir_addr: VirtAddr) -> isize {
        let mut inner = self.current.as_mut().unwrap().inner_exclusive_access();

        let result = inner.memory_set.
            delete_framed_area(start_vir_addr, end_vir_addr);
        drop(inner);
        result
    }
    
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

///The main part of process execution and scheduling
///Loop `fetch_task` to get the process that needs to run, and switch the process through `__switch`
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            // release coming task_inner manually
            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task);
            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            warn!("no tasks available in run_tasks");
        }
    }
}

/// Get current task through take, leaving a None in its place
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// Get a copy of the current task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// Get the current user token(addr of page table)
/// 提供当前正在执行的任务的user_token
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    task.get_user_token()
}

///Get the mutable reference to trap context of current task
/// 提供当前正在执行的任务的trap_context
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

///Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
/// Get Current TaskInfo
pub fn get_current_task_info() -> TaskInfo{
    PROCESSOR.exclusive_access().get_current_task_info()
}

/// add syscall times of current task
pub fn add_syscall_times(syscall_id: usize){
    PROCESSOR.exclusive_access().add_syscall_times(syscall_id);
}

/// 分配虚存
pub fn mmap(start_vir_addr: VirtAddr, end_vir_addr: VirtAddr, port: usize) -> isize {
    PROCESSOR.exclusive_access().mmap(start_vir_addr, end_vir_addr, port)
}

/// 取消分配虚存
pub fn munmap(start_vir_addr: VirtAddr, end_vir_addr: VirtAddr) -> isize {
    PROCESSOR.exclusive_access().munmap(start_vir_addr, end_vir_addr)
}
