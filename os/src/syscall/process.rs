//! Process management syscalls
use crate::{
    timer::get_time_us,
    mm::VirtAddr,
    task::{
        change_program_brk, exit_current_and_run_next, 
        get_current_task_info, suspend_current_and_run_next, 
        mmap, munmap, TaskInfo, 
    },
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}


/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let phys_addr = VirtAddr(ts as usize).convert_to_phys_addr();
    let us = get_time_us();
    match phys_addr {
        Some(phys_addr) => {
            unsafe {
                *(phys_addr.0 as *mut TimeVal) = TimeVal {
                    sec: us / 1_000_000,
                    usec: us % 1_000_000,
                };
            }
            0
        },
        None => -1
    }
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let phys_addr = VirtAddr(ti as usize).convert_to_phys_addr();
    let info : TaskInfo = get_current_task_info();
    match phys_addr {
        Some(phys_addr1) => {
            unsafe {
                *(phys_addr1.0 as *mut TaskInfo) = info;
            }
            0
        },
        None => -1
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    let start_vir_addr = VirtAddr(start as usize);

    if !start_vir_addr.aligned() {
        return -1;
    }

    if port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }
    let end_vir_addr = VirtAddr(start + len).ceil().into();

    // let mut inner = TASK_MANAGER.inner.exclusive_access();
    let result = mmap(start_vir_addr, end_vir_addr, port);
    result
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let start_vir_addr = VirtAddr(start as usize);

    if !start_vir_addr.aligned() {
        return -1;
    }
    let end_vir_addr = VirtAddr(start + len);

    
    let result = munmap(start_vir_addr, end_vir_addr);
    result
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
