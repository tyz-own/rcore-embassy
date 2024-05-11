//! File and filesystem-related syscalls

use crate::fs::{fstat, link, open_file, unlink, OSInode, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};
use alloc::sync::Arc;


/// buf:缓冲区
/// 把缓冲区的内容写入文件
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}


/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    trace!("kernel:pid[{}] sys_fstat NOT IMPLEMENTED", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let inner = task.inner_exclusive_access();
    let inode = unsafe {
        &*(Arc::as_ptr(inner.fd_table[fd].as_ref().unwrap()) as *const OSInode)
    };
    let st_info = translated_refmut(token, st);
    fstat(inode, st_info);
    0
     
    // let inode= inner.fd_table[fd].unwrap() as Arc<OSInode>;
    // let mut st_info = unsafe{st.as_mut().unwrap()};
    // st_info.dev = 0;
    // st_info.ino = inode.get_inode().get_id();
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_linkat NOT IMPLEMENTED", current_task().unwrap().pid.0);
    if old_name == new_name{
        return -1;
    }
    // let task = current_task().unwrap();
    let token = current_user_token();
    let old_name = translated_str(token, old_name);
    let new_name = translated_str(token, new_name);
    // if let Some(inode) = open_file(old_name.as_str(), OpenFlags::from_bits(2 as u32).unwrap()) {
    //     // let mut inner = task.inner_exclusive_access();
    //     // let fd = inner.alloc_fd();
    //     // inner.fd_table[fd] = Some(inode);
    link(new_name.as_str(),old_name.as_str());
    0
    
    
    

}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED", current_task().unwrap().pid.0);
    // let _task = current_task().unwrap();
    let token = current_user_token();
    let name = translated_str(token, name);
    if let Some(_inode) = open_file(name.as_str(), OpenFlags::from_bits(2 as u32).unwrap()) {
        // let mut inner = task.inner_exclusive_access();
        // let fd = inner.alloc_fd();
        // inner.fd_table[fd] = Some(inode);
        unlink(name.as_str());
        0
    } else {
        -1
    }
    // let fd = find_inode_id(name.as_str());
    // if fd.is_none() {
    //     return -1;
    // }
    // let mut inner = task.inner_exclusive_access();

    // unlink(name.as_ptr(),fd.unwrap());
    // inner.fd_table[fd.unwrap()] = None;
}
