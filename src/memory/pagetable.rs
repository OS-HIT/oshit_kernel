#![macro_use]
extern crate bitflags;

use bitflags::*;
use super::{
    PhysPageNum,
    VirtPageNum,
    FrameTracker,
    alloc_frame,
};
use alloc::vec::Vec;

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

/// 63         5352                                                  1098 
/// | reserved  ||                        PPN                         ||| DAGU XWRV
/// XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX
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

pub struct PageTable {
    root_ppn: PhysPageNum,
    frames  : Vec<FrameTracker>
}

impl PageTable {
    pub fn new() -> Self {
        let root = alloc_frame().unwrap();     // might panic when OOM, but who cares?
        PageTable {
            root_ppn: root.ppn,
            frames: vec![root]
        }
    }

    // similar to xv6, get pte representing vpn from pagetable, and create parent pte if not present.
    // self may be not valid, but parent must be.
    fn walk_create(&mut self, vpn: VirtPageNum) -> &mut PageTableEntry {
        let indexes = vpn.indexes();
        let mut ppn = self.root_ppn;
        for i in 0..3 {
            let pte = &mut ppn.read_pte()[indexes[i]];
            if i == 2 {         // leaf node, just return
                return pte;
            }
            if !pte.valid() {   // not a leaf node, yet invalid
                let frame = alloc_frame().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        unreachable!();     // don't comment this out, or compiler will be unhappy
    }

    // don't create parent node, just Some or None
    fn walk(&mut self, vpn: VirtPageNum) -> Option<&PageTableEntry> {
        let indexes = vpn.indexes();
        let mut ppn = self.root_ppn;
        for i in 0..3 {
            let pte = &ppn.read_pte()[indexes[i]];
            if i == 2 {         // leaf node, just return
                return Some(pte);
            }
            if !pte.valid() {   // not a leaf node, yet invalid
                return None;
            }
            ppn = pte.ppn();
        }
        unreachable!();     // don't comment this out, or compiler will be unhappy
    }

    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.walk_create(vpn);
        assert!(!pte.valid(), "{:?} has already been mapped.", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.walk_create(vpn);
        assert!(pte.valid(), "{:?} hasn't been mapped.", vpn);
        *pte = PageTableEntry::empty();
    }

    pub fn from_satp(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    pub fn translate(&mut self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.walk(vpn).map(|pte| pte.clone())
    }
}