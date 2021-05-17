use alloc::vec::Vec;
use core::ops::{Index, IndexMut};

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