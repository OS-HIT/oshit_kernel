#![macro_use]
extern crate bitflags;

use bitflags::*;
use super::{
    PhysPageNum,
    // VirtPageNum,
};

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;   // valid
        const R = 1 << 1;   // read enable
        const W = 1 << 2;   // write enable
        const X = 1 << 3;   // execute enable
        const U = 1 << 4;   // user accessability
        const G = 1 << 5;   // ?
        const A = 1 << 6;   // Accessed
        const D = 1 << 7;   // Dirty
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize
        }
    }

    pub fn empty() -> Self {
        PageTableEntry {
            bits: 0
        }
    }

    pub fn ppn(&self) -> PhysPageNum {
        PhysPageNum::from(self.bits >> 10 & 0xFFF_FFFF_FFFF)
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    pub fn valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
}