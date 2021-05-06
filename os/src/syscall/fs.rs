use crate::mm::{
    UserBuffer,
    translated_byte_buffer,
    translated_refmut,
    translated_str,
    check_table_exist,
};
use crate::task::{current_user_token, current_task, find_mailbox, clear_mailbox, translate_va};
use crate::fs::{make_pipe, OpenFlags, open_file, linkat, unlinkat, Stat};
use alloc::sync::Arc;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.acquire_inner_lock();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release Task lock manually to avoid deadlock
        drop(inner);
        file.write(
            UserBuffer::new(translated_byte_buffer(token, buf, len))
        ) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.acquire_inner_lock();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release Task lock manually to avoid deadlock
        drop(inner);
        file.read(
            UserBuffer::new(translated_byte_buffer(token, buf, len))
        ) as isize
    } else {
        -1
    }
}

pub fn sys_linkat(old_dirfd: usize, old_path: *const u8, new_dirfd: usize, new_path: *const u8, flag: usize) -> isize {
    let token = current_user_token();
    let old_name = translated_str(token, old_path);
    let new_name = translated_str(token, new_path);
    if linkat(&old_name, &new_name, flag) {
        0
    } else {
        -1
    }
}

pub fn sys_unlinkat(dirfd: usize, path: *const u8, flags: usize) -> isize {
    let token = current_user_token();
    let name = translated_str(token, path);
    if unlinkat(&name) {
        0
    } else {
        -1
    }
}

pub fn sys_fstat(fd: u32, st: usize) -> isize {
    let token = current_user_token();
    let cur_st = translate_va(st);
    let tmp = cur_st as *mut Stat;
    let task = current_task().unwrap();
    let inner = task.acquire_inner_lock();
    if fd >= (inner.fd_table.len() as u32) {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd as usize] {
        let file = file.clone();
        unsafe {*tmp = file.get_stat()};
        return 0;
    }
    return -1
}

pub fn sys_open(dirfd: usize, path: *const u8, flags: u32, mode: u32) -> isize {
    if flags & 0x3 == 0x3 {
        return -1;
    }
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(
        path.as_str(),
        OpenFlags::from_bits(flags).unwrap()
    ) {
        let mut inner = task.acquire_inner_lock();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.acquire_inner_lock();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let mut inner = task.acquire_inner_lock();
    let (pipe_read, pipe_write) = make_pipe();
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write);
    *translated_refmut(token, pipe) = read_fd;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
    0
}

pub fn sys_mail_read(buf: *mut u8, len: usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let len = if len < 256 {len} else {256};
    let mut inner = task.acquire_inner_lock();
    let pid = task.pid.0;
    let mut mb = find_mailbox(pid);
    if check_table_exist(token, buf, len) == false {
        drop(inner);
        -1
    } else {
        if len == 0 {
            if mb.readable() {
                0
            } else {
                -1
            }
        } else {
            let outlen = mb.read(
                UserBuffer::new(
                    translated_byte_buffer(token, buf, len)
                )
            );
            drop(inner);
            outlen
        }
    }
}

pub fn sys_mail_write (pid: usize, buf: *mut u8, len: usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let len = if len < 256 {len} else {256};
    let mut inner = task.acquire_inner_lock();
    let mut mb = find_mailbox(pid);
    if check_table_exist(token, buf, len) == false {
        drop(inner);
        -1
    } else {
        if len == 0 {
            if mb.writable() {
                0
            } else {
                -1
            }
        } else {
            let outlen = mb.write(
                UserBuffer::new(
                    translated_byte_buffer(token, buf, len)
                )
            );
            drop(inner);
            outlen
        }
    }
}

pub fn sys_dup(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.acquire_inner_lock();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    let new_fd = inner.alloc_fd();
    inner.fd_table[new_fd] = Some(Arc::clone(inner.fd_table[fd].as_ref().unwrap()));
    new_fd as isize
}