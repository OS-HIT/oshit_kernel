//! Pipe implementation for oshit-kernel
use _core::convert::TryInto;
use bitflags::*;
use alloc::sync::{Arc, Weak};
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
use crate::{memory::UserBuffer, process::suspend_switch};

bitflags! {
    /// Flags to mark read/write end of pipe.
    pub struct PipeFlags: u8 {
        const R = 1 << 0;
        const W = 1 << 1;
    }
}

/// Pipe ring buffer and end weak references.
pub struct Pipe {
    /// The ring buffer.
    buffer: VecDeque<u8>,
    /// weak reference to read ends of pipe
    read_ends: Vec<Weak<PipeEnd>>,
    /// weak reference to write ends of pipe
    write_ends: Vec<Weak<PipeEnd>>,
}

/// Pipe read/write end. Maybe we should use two different struuct but whatever.
pub struct PipeEnd {
    /// Flags to indicate read/write privilege
    flags: PipeFlags,
    /// shared, locked reference to Pipe (The ring buffer)
    pipe:  Arc<Mutex<Pipe>>
}

impl Pipe {
    /// create a pipe ring buffer
    /// # Description
    /// Create a pipe ring buffer and wrap it in a arc and spin lock
    /// # Examples
    /// ```
    /// let pipe = Pipe::new();
    /// let read = PipeEnd::new_read(&pipe);
    /// let write = PipeEnd::new_write(&pipe);
    /// ```
    /// # Return
    /// A new pipe, wrapped in an Arc and spin lock
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(
            Pipe{
                buffer:VecDeque::new(),
                read_ends: Vec::new(),
                write_ends: Vec::new()
            }
        ))
    }

    /// write to the pipe ring buffer
    /// # Description
    /// Write data from `buf: UserBuffer` with offset to pipe ring buffer.  
    /// Note that buffer overflow will cause the opration to fail.
    /// # return
    /// return how many bytes has been written into the buffer. -1 on fail.
    pub fn write(&mut self, buf: &UserBuffer, offset: usize) -> isize {
        if buf.len() + self.buffer.len() + offset > crate::config::PIP_BUF_MAX {
            error!("Buffer Overflow!");     // TODO: change to write as much as possible
            return -1;
        }
        let len = buf.len() - offset;
        for idx in offset..buf.len() {
            self.buffer.push_back(buf[idx]);
        }
        return len.try_into().unwrap();
    }


    /// Read from the pipe ring buffer
    /// # Description
    /// Read data from pipe ring buffer to `buf: UserBuffer` with offset.  
    /// # return
    /// return how many bytes has been written into the buffer.
    pub fn read(&mut self, buf: &mut UserBuffer, offset: usize) -> isize {
        let mut idx = offset;
        loop {
            if idx < buf.len() {
                if let Some(byte) = self.buffer.pop_front() {
                    buf[idx] = byte;
                    idx += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        return idx.try_into().unwrap();
    }

    /// Register read end for pipe
    /// # Description
    /// Register a weak reference of read end in the pipe.  
    /// Note: no need to un-register, for all those Weak reference will automagically dies.
    pub fn register_read(&mut self, pipe_end: &Arc<PipeEnd>) {
        self.read_ends.push(Arc::downgrade(pipe_end));
    }


    /// Register write end for pipe
    /// # Description
    /// Register a weak reference of write end in the pipe.  
    /// Note: no need to un-register, for all those Weak reference will automagically dies.
    pub fn register_write(&mut self, pipe_end: &Arc<PipeEnd>) {
        self.write_ends.push(Arc::downgrade(pipe_end));
    }

    /// Check if all write end are closed.
    /// # Description
    /// Check if all write end are closed, so we can return instantly on read on a dead pipe.
    /// # Return
    /// `true` if all write end has been closed. 
    pub fn all_write_closed(&self) -> bool {
        for i in self.write_ends.iter() {
            if i.upgrade().is_some() {
                return false;
            }
        }
        return true;
    }

    /// Check if the ring buffer is empty
    /// # Description
    /// Check if the pipe has nothing in it.
    /// # Return
    /// `true` if the buffer is empty.
    pub fn empty(&self) -> bool {
        return self.buffer.is_empty();
    }
}

impl PipeEnd {
    fn new_read(pipe: &Arc<Mutex<Pipe>>) -> Arc<Self> {
        let ret = Arc::new(Self {
            flags: PipeFlags::R,
            pipe: pipe.clone()
        });
        pipe.lock().register_read(&ret);
        return ret;
    }

    fn new_write(pipe: &Arc<Mutex<Pipe>>) -> Arc<Self> {
        let ret = Arc::new(Self {
            flags: PipeFlags::W,
            pipe: pipe.clone()
        });
        pipe.lock().register_write(&ret);
        return ret;
    }
}

impl super::VirtFile for PipeEnd {
    /// Read from Pipe read end
    fn read(&self, mut buf: UserBuffer) -> isize {
        verbose!("Reading from pipe.");
        if !self.flags.contains(PipeFlags::R) {
            error!("no priviledge.");
            return -1;
        }
        let mut read_size = 0;
        let mut offset = 0;
        loop {
            // verbose!("Trying to lock.");
            let mut pipe = self.pipe.lock();
            // verbose!("locked.");
            if pipe.empty() {
                // verbose!("is empty.");
                if pipe.all_write_closed() {
                    // verbose!("all write closed.");
                    return read_size;
                } else {
                    drop(pipe);
                    // verbose!("unlocked, waiting.");
                    suspend_switch();
                    continue;
                }
            }
            let len = pipe.read(&mut buf, offset);
            read_size += len;
            if read_size == buf.len() as isize {
                break;
            } else {
                offset += len as usize;
            }
        }
        return read_size;
    }

    /// Write to Pipe write end
    fn write(&self, buf: UserBuffer) -> isize {
        verbose!("Writing to pipe.");
        if !self.flags.contains(PipeFlags::W) {
            error!("no priviledge.");
            return -1;
        }
        verbose!("Trying to lock.");
        let res = self.pipe.lock().write(&buf, 0);
        verbose!("write done");
        return res;
    }
    fn to_fs_file_locked(&self) -> Result<MutexGuard<super::FILE>, &str> {
        Err("Not a file")
    }
}

/// Create a pipe and a pair of read end and write end
/// # Description
/// Create a new pipe and the initial two end of the pipe.
/// # Example
/// ```
/// let proc = current_process().unwarp();
/// let mut arcpcb = proc.get_inner_locked();
/// let (read, write) = make_pipe();
/// let read_fd = arcpcb.alloc_fd();
/// arcpcb.files[read_fd] = read;
/// let write_fd = arcpcb.alloc_fd();
/// arcpcb.files[write_fd] = write;
/// ```
/// # Return
/// A pair of PipeEnd of the pipe.
pub fn make_pipe() -> (Arc<PipeEnd>, Arc<PipeEnd>) {
    let pipe = Pipe::new();
    let read_end = PipeEnd::new_read(&pipe);
    let write_end = PipeEnd::new_write(&pipe);
    return (read_end, write_end);
}
