use alloc::{collections::binary_heap::Iter, vec::Vec};
use core::ops::{Index, IndexMut};
use core::mem::size_of;

pub struct UserBuffer {
    pub parts: Vec<&'static mut [u8]>
}

impl UserBuffer {
    pub fn new(parts: Vec<&'static mut [u8]>) -> Self {
        Self {parts}
    }

    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for p in self.parts.iter() {
            total += p.len();
        }
        return total;
    }

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

    // Note: this only return a copy of original data
    // TODO: check if this actually works.
    pub fn read<T: Copy>(&self, offset: usize) -> T {
        let mut res: Vec<u8> = Vec::new();
        for i in 0..size_of::<T>() {
            res.push(self[offset+i]);
        }
        let u8s: &[u8] = &res;
        let dst: *mut T = u8s.as_ptr() as usize as *mut T;
        return unsafe{dst.as_mut().unwrap().clone()};
    }

    // TODO: use Vec::from_raw_parts
    pub fn clone_bytes(&self) -> Vec<u8> {
        let mut cloned: Vec<u8> = Vec::new();
        for b in self {
            cloned.push(b);
        }
        cloned
    }

    pub fn write_bytes(&mut self, bytes: &[u8], offset: usize)  {
        if offset + bytes.len() > self.len() {
            panic!("UserBuffer overflow!");
        }

        for (idx, b) in bytes.iter().enumerate() {
            self[offset + idx] = *b;
        }
    }
}

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


// TODO: Is there a way to this without code duplication?
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