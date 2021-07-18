use core::cmp::min;

use alloc::{collections::VecDeque, string::ToString, sync::{Arc, Weak}, vec::Vec};
use spin::Mutex;

use super::{File, file::FileStatus};

/// Pipe ring buffer and end weak references.
pub struct Pipe {
    /// buffer max size
    size: u64,
    /// The ring buffer.
    buffer: VecDeque<u8>,
    /// weak reference to read ends of pipe
    read_ends: Vec<Weak<PipeEnd>>,
    /// weak reference to write ends of pipe
    write_ends: Vec<Weak<PipeEnd>>,
}

impl Pipe {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(
            Pipe{
                size: 4096,
                buffer:VecDeque::new(),
                read_ends: Vec::new(),
                write_ends: Vec::new()
            }
        ))
    }

    
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        let len = min(buffer.len(), self.buffer.len());
        for i in 0..len {
            buffer[i] = self.buffer.pop_front().unwrap();
        }
        Ok(len)
    }

    pub fn write(&mut self, buffer: &[u8]) -> Result<usize, &'static str> {
        let len = min(buffer.len(), self.size as usize - self.buffer.len());
        for i in 0..len {
            self.buffer.push_back(buffer[i as usize]);
        }
        Ok(len)
    }

    pub fn read_user_buffer(&mut self, mut buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
        let len = min(buffer.len(), self.buffer.len());
        for i in 0..len {
            buffer[i] = self.buffer.pop_front().unwrap();
        }
        Ok(len)
    }

    pub fn write_user_buffer(&mut self, buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
        let len = min(buffer.len(), self.size as usize - self.buffer.len());
        for i in 0..len {
            self.buffer.push_back(buffer[i as usize]);
        }
        Ok(len)
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

/// Pipe read/write end. Maybe we should use two different struuct but whatever.
pub struct PipeEnd {
    /// Flags to indicate read/write privilege
    flags: FileStatus,
    /// shared, locked reference to Pipe (The ring buffer)
    pipe:  Arc<Mutex<Pipe>>
}

impl PipeEnd {
    fn new_read(pipe: &Arc<Mutex<Pipe>>) -> Arc<Self> {
        let ret = Arc::new(Self {
            flags: FileStatus {
                readable: true,
                writeable: false,
                size: 0,
                name: "".to_string(),
                ftype: super::file::FileType::FIFO,
                inode: 0,
                dev_no: 0,
                mode: 0,
                block_sz: 0,
                blocks: 0,
                uid: 0,
                gid: 0,
                atime_sec:  0,
                atime_nsec: 0,
                mtime_sec:  0,
                mtime_nsec: 0,
                ctime_sec:  0,
                ctime_nsec: 0,
            },
            pipe: pipe.clone()
        });
        pipe.lock().register_read(&ret);
        return ret;
    }

    fn new_write(pipe: &Arc<Mutex<Pipe>>) -> Arc<Self> {
        let ret = Arc::new(Self {
            flags: FileStatus {
                readable: false,
                writeable: true,
                size: 0,
                name: "".to_string(),
                ftype: super::file::FileType::FIFO,
                inode: 0,
                dev_no: 0,
                mode: 0,
                block_sz: 0,
                blocks: 0,
                uid: 0,
                gid: 0,
                atime_sec:  0,
                atime_nsec: 0,
                mtime_sec:  0,
                mtime_nsec: 0,
                ctime_sec:  0,
                ctime_nsec: 0,
            },
            pipe: pipe.clone()
        });
        pipe.lock().register_write(&ret);
        return ret;
    }
}

impl File for PipeEnd {
    fn seek(&self, _: isize, _: super::SeekOp) -> Result<(), &'static str> {
        Err("Cannot seek a pipe")
    }

    fn get_cursor(&self) -> Result<usize, &'static str> {
        Err("Pipe has no cursor")
    }

    fn read(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        self.pipe.lock().read(buffer)
    }

    fn write(&self, buffer: &[u8]) -> Result<usize, &'static str> {
        self.pipe.lock().write(buffer)
    }

    fn read_user_buffer(&self, mut buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
        self.pipe.lock().read_user_buffer(buffer)
    }

    fn write_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
        self.pipe.lock().write_user_buffer(buffer)
    }

    fn to_common_file(&self) -> Option<alloc::sync::Arc<dyn super::CommonFile>> {
        None
    }

    fn to_dir_file(&self) -> Option<alloc::sync::Arc<dyn super::DirFile>> {
        None
    }

    fn to_device_file(&self) -> Option<alloc::sync::Arc<dyn super::DeviceFile>> {
        None
    }

    fn poll(&self) -> super::file::FileStatus {
        self.flags.clone()
    }

    fn rename(&self, _: &str) -> Result<(), &'static str> {
        Err("Pipe has no name")
    }

    fn get_vfs(&self) -> Result<Arc<(dyn super::VirtualFileSystem + 'static)>, &'static str> {
        Err("Pipe has no vfs")
    }

    fn get_path(&self) -> alloc::string::String {
        "".to_string()
    }
}

impl Drop for PipeEnd {
    fn drop(&mut self) {
        // just die.
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
