#![macro_use]
extern crate bitflags;

use bitflags::*;
use super::{
    PhysPageNum,
    VirtPageNum,
    VirtAddr,
    FrameTracker,
    alloc_frame,
};
use alloc::vec::Vec;
use core::cmp::min;
use crate::utils::StepByOne;

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

///  63         5352                                                  1098 
///  | reserved  ||                        PPN                         ||| DAGU XWRV
/// [XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX
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

    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }

    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
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

    pub fn get_satp(&self) -> usize {
        return 8usize << 60 | self.root_ppn.0;
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
    fn walk(&self, vpn: VirtPageNum) -> Option<&PageTableEntry> {
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

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.walk(vpn).map(|pte| pte.clone())
    }
}

pub fn get_user_data(satp: usize, mut start: VirtAddr, len: usize) -> Vec<&'static [u8]> {
    let pagetable = PageTable::from_satp(satp);
    let end = start + len;
    let mut pages = Vec::new();
    while start < end {
        let mut vpn = start.to_vpn();
        let ppn = pagetable.translate(vpn).unwrap().ppn();
        vpn.step();
        let copy_end = min(vpn.into(), end);    // page end or buf end
        pages.push(&ppn.page_ptr()[
            start.page_offset()
            ..
            copy_end.page_offset()
        ]);
        start = copy_end;
    }

    return pages;
}

pub fn write_user_data(satp: usize, data: &[u8], mut start: VirtAddr, len: usize) {
    let pagetable = PageTable::from_satp(satp);
    let origin = start.0;
    let end = start + len;
    while start < end {
        let mut vpn = start.to_vpn();
        let ppn = pagetable.translate(vpn).unwrap().ppn();
        vpn.step();
        let copy_end = min(vpn.into(), end);    // page end or buf end
        &ppn.page_ptr()[
            start.page_offset()
            ..
            copy_end.page_offset()
        ].copy_from_slice(&data[(start - origin).0..(copy_end - origin).0]);
        start = copy_end;
    }
}


// FIXME: Might go wrong if object happened to be on two (or even more) pages.
pub fn translate_user_va<T>(satp: usize, va: VirtAddr) -> *mut T {
    let pagetable = PageTable::from_satp(satp);
    let vpn = va.to_vpn();
    let ppn = pagetable.translate(vpn).unwrap().ppn();
    // HACK: FUCK ME that is evil
    return (&mut ppn.page_ptr()[va.page_offset()]) as *mut u8 as usize as *mut T;
}