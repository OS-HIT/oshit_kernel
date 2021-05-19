use _core::convert::TryInto;
use bitflags::*;
use alloc::sync::{Arc, Weak};
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use riscv::register::hpmcounter17::read;
use spin::Mutex;
use crate::{memory::UserBuffer, process::suspend_switch};

bitflags! {
    pub struct PipeFlags: u8 {
        const R = 1 << 0;
        const W = 1 << 1;
    }
}

pub struct Pipe {
    buffer: VecDeque<u8>,
    read_ends: Vec<Weak<PipeEnd>>,
    write_ends: Vec<Weak<PipeEnd>>,
}

pub struct PipeEnd {
    flags: PipeFlags,
    pipe:  Arc<Mutex<Pipe>>
}

impl Pipe {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(
            Pipe{
                buffer:VecDeque::new(),
                read_ends: Vec::new(),
                write_ends: Vec::new()
            }
        ))
    }

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

    // no need to un-register, for all those Weak will automagically dies.
    pub fn register_read(&mut self, pipe_end: &Arc<PipeEnd>) {
        self.read_ends.push(Arc::downgrade(pipe_end));
    }

    pub fn register_write(&mut self, pipe_end: &Arc<PipeEnd>) {
        self.write_ends.push(Arc::downgrade(pipe_end));
    }

    pub fn all_write_closed(&self) -> bool {
        for i in self.write_ends.iter() {
            if i.upgrade().is_some() {
                return false;
            }
        }
        return true;
    }

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

impl super::File for PipeEnd {
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
}

pub fn make_pipe() -> (Arc<PipeEnd>, Arc<PipeEnd>) {
    let pipe = Pipe::new();
    let read_end = PipeEnd::new_read(&pipe);
    let write_end = PipeEnd::new_write(&pipe);
    return (read_end, write_end);
}