//! A helper class, repersenting a chunk of user memory.
use alloc::{vec::Vec};
use core::ops::{Index, IndexMut};
use core::mem::size_of;
/// A helper class, repersenting a chunk of user memory.
pub struct UserBuffer {
    pub parts: Vec<&'static mut [u8]>
}

impl UserBuffer {
    /// Construct a new user buffer from fragmented physical memory.
    pub fn new(parts: Vec<&'static mut [u8]>) -> Self {
        Self {parts}
    }

    /// Get the length of the user buffer
    /// # Return
    /// the length of the user buffer
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for p in self.parts.iter() {
            total += p.len();
        }
        return total;
    }

    /// Write a object to the user buffer
    /// # Description
    /// Write a object to the user buffer to the offset
    /// # Example
    /// ```
    /// let proc = current_process().unwarp();
    /// let arcpcb = proc.get_inner_locked();
    /// let mut user_buffer = arcpcb.layout.get_user_buffer(0x10000.into(), 5 * core::mem::size_of::<usize>())
    /// for i in 0..5 {
    ///     user_buffer.write(i * core::mem::size_of::<usize>(), i as usize);
    /// }
    /// ```
    pub fn write<T>(&mut self, offset: usize, obj: &T) {
        let size = size_of::<T>();
        let mut iter = obj as *const T as usize as *const u8;
        for i in 0..size {
            unsafe {
                self[offset + i] = *iter;
                iter = iter.add(1) 
            };
        }
    }

    /// Read a object from the user buffer
    /// # Description
    /// Read a object from the user buffer on the offset, and return it's clone
    /// # Example
    /// ```
    /// let proc = current_process().unwarp();
    /// let arcpcb = proc.get_inner_locked();
    /// let user_buffer = arcpcb.layout.get_user_buffer(0x10000.into(), 5 * core::mem::size_of::<usize>())
    /// for i in 0..5 {
    ///     let res: usize = user_buffer.write(i * core::mem::size_of::<usize>());
    ///     print!("{}", res);
    /// }
    /// ```
    /// # Return
    /// A copy of the object in the user memory space
    pub fn read<T: Copy>(&self, offset: usize) -> T {
        let mut res: Vec<u8> = Vec::new();
        for i in 0..size_of::<T>() {
            res.push(self[offset+i]);
        }
        let u8s: &[u8] = &res;
        let dst: *mut T = u8s.as_ptr() as usize as *mut T;
        return unsafe{dst.as_mut().unwrap().clone()};
    }

    /// Clone all bytes in the userbuffer.
    /// # Return
    /// A clone of all bytes in the user buffer
    pub fn clone_bytes(&self) -> Vec<u8> {
        let mut cloned: Vec<u8> = Vec::new();
        for b in self {
            // TODO: use Vec::from_raw_parts
            cloned.push(b);
        }
        cloned
    }

    /// write a sequence of bytes to the user buffer
    /// # Description
    /// Write a sequence of bytes to the user buffer on offset. Panic on overflow.
    pub fn write_bytes(&mut self, bytes: &[u8], offset: usize)  {
        if offset + bytes.len() > self.len() {
            panic!("UserBuffer overflow!");
        }

        for (idx, b) in bytes.iter().enumerate() {
            self[offset + idx] = *b;
        }
    }
}

/// A iterator for userbuffer
/// # Example
/// ```
/// let proc = current_process().unwarp();
/// let arcpcb = proc.get_inner_locked();
/// let user_buffer = arcpcb.layout.get_user_buffer(0x10000.into(), 100)
/// for b in user_buffer.into_iter() {
///     print!("{}", b as char);   
/// }
/// ```
impl<'a> IntoIterator for &'a UserBuffer {
    type Item = u8;
    type IntoIter = UserBufferIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        UserBufferIter {
            index: 0,
            buffer: self
        }
    }
}

/// Make us able to index
/// # Example
/// ```
/// let proc = current_process().unwarp();
/// let arcpcb = proc.get_inner_locked();
/// let user_buffer = arcpcb.layout.get_user_buffer(0x10000.into(), 100)
/// let res: u8 = user_buffer[2];
/// ```
impl Index<usize> for UserBuffer {
    type Output = u8;
    fn index(&self, idx: usize) -> &Self::Output {
        let mut len = idx;
        for p in self.parts.iter() {
            if len < p.len() {
                return &p[len];
            }
            len -= p.len();
        }
        panic!("Index out of bound!");
    }
}


/// Make us able to index
/// # Example
/// ```
/// let proc = current_process().unwarp();
/// let arcpcb = proc.get_inner_locked();
/// let user_buffer = arcpcb.layout.get_user_buffer(0x10000.into(), 100)
/// let mut res: u8 = user_buffer[2];
/// ```
impl IndexMut<usize> for UserBuffer {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output { 
        let mut len = idx;
        for p in self.parts.iter_mut() {
            if len < p.len() {
                return &mut p[len];
            }
            len -= p.len();
        }
        panic!("Index out of bound!");
    }
}

/// The iterator for UserBuffer
pub struct UserBufferIter<'a> {
    index: usize,
    buffer: &'a UserBuffer,
}

impl<'a> Iterator for UserBufferIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.buffer.len() {
            return None;
        } else {
            self.index = self.index + 1;
            return Some(self.buffer[self.index - 1]);
        }
    }
}