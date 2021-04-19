use super::File;
use alloc::sync::{Arc, Weak};
use spin::Mutex;
use crate::mm::{
    UserBuffer,
};
use crate::task::suspend_current_and_run_next;

const MAIL_RING_BUFFER_SIZE: usize = 256;
const MAIL_NUM: usize = 16;

#[derive(Copy, Clone, PartialEq)]
enum MailBufferStatus {
    FULL,
    EMPTY,
    NORMAL,
}

pub struct MailBoxBuffer {
    buffer: [u8; MAIL_RING_BUFFER_SIZE * MAIL_NUM],
    len: [usize; MAIL_NUM],
    head: usize,
    tail: usize,
    status: MailBufferStatus,
    writable: bool,
    readable: bool,
    cnt: isize,
}

pub struct MailBox {
    pub buffer: Arc<Mutex<MailBoxBuffer>>
}

impl MailBoxBuffer {
    pub fn new() -> Self {
        Self {
            buffer: [0; MAIL_RING_BUFFER_SIZE * MAIL_NUM],
            len: [0; MAIL_NUM],
            head: 0,
            tail: 0,
            status: MailBufferStatus::EMPTY,
            writable: true,
            readable: false,
            cnt: 0,
        }
    }
}

impl MailBox {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(MailBoxBuffer::new())),
        }
    }
    pub fn readable(&self) -> bool {
        let mut ring_buffer = self.buffer.lock();
        let flag = ring_buffer.cnt > 0;
        drop(ring_buffer);
        flag
    }
    pub fn writable(&self) -> bool {
        let mut ring_buffer = self.buffer.lock();
        let flag = ring_buffer.cnt < 16;
        drop(ring_buffer);
        flag
    }
    pub fn read(&self, buf: UserBuffer) -> isize {
        let mut ring_buffer = self.buffer.lock();
        // assert_eq!(ring_buffer.readable, true);
        let mut buf_iter = buf.into_iter();
        let mut read_size = 0usize;
        let now = ring_buffer.head * MAIL_RING_BUFFER_SIZE;

        // if ring_buffer.readable == false {
        //     drop(ring_buffer);
        //     -1
        if ring_buffer.cnt == 0{
            drop(ring_buffer);
            -1
        } else {
            // read at most loop_read bytes
            for _ in 0..ring_buffer.len[now / MAIL_RING_BUFFER_SIZE] {
                if let Some(byte_ref) = buf_iter.next() {
                    unsafe { *byte_ref = ring_buffer.buffer[now + read_size]; }
                    read_size += 1;
                } else {
                    // return read_size as isize;
                    break;
                }
            }
            ring_buffer.head = (ring_buffer.head + 1) % MAIL_NUM;
            ring_buffer.cnt -= 1;
            if ring_buffer.tail == ring_buffer.head {
                ring_buffer.readable = false;
                ring_buffer.writable = true;
                ring_buffer.status = MailBufferStatus::EMPTY;
            } else {
                ring_buffer.readable = true;
                ring_buffer.writable = true;
                ring_buffer.status = MailBufferStatus::NORMAL;
            }  
            drop(ring_buffer);
            read_size as isize
        }
    }
    pub fn write(&self, buf: UserBuffer) -> isize {
        let mut ring_buffer = self.buffer.lock();
        // assert_eq!(ring_buffer.writable, true);
        let mut buf_iter = buf.into_iter();
        let mut write_size = 0usize;
        let mut now = ring_buffer.tail * MAIL_RING_BUFFER_SIZE;
        
        // if ring_buffer.writable == false {
        //     drop(ring_buffer);
        //     println!("not writable");
        //     -1
        if ring_buffer.cnt == 16 {
            drop(ring_buffer);
            println!("not writable");
            -1
        } else {
            // write at most loop_write bytes
            for _ in 0..MAIL_RING_BUFFER_SIZE {
                if let Some(byte_ref) = buf_iter.next() {
                    ring_buffer.buffer[now + write_size] = unsafe {*byte_ref };
                    write_size += 1;
                } else {
                    // return write_size as isize;
                    break;
                }
            }
            ring_buffer.cnt += 1;
            ring_buffer.len[now / MAIL_RING_BUFFER_SIZE] = write_size;
            ring_buffer.tail = (ring_buffer.tail + 1) % MAIL_NUM;
            if ring_buffer.tail == ring_buffer.head {
                ring_buffer.status = MailBufferStatus::FULL;
                ring_buffer.readable = true;
                ring_buffer.writable = false;
            } else {
                ring_buffer.status = MailBufferStatus::NORMAL;
                ring_buffer.readable = true;
                ring_buffer.writable = true;
            }
            drop(ring_buffer);

            write_size as isize
        }
    }
}

