//! SV39 pagetable implementation.
#![macro_use]
extern crate bitflags;

use bitflags::*;
use super::{
    PhysPageNum,
    VirtPageNum,
    VirtAddr,
    PhysAddr,
    FrameTracker,
    alloc_frame,
    UserBuffer
};
use alloc::vec::Vec;
use core::cmp::min;
use crate::utils::StepByOne;
use alloc::string::String;

bitflags! {
    /// Pagetable entry flags, indicating privileges.
    pub struct PTEFlags: u8 {
        /// valid
        const V = 1 << 0;   
        /// read enable
        const R = 1 << 1;   
        /// write enable
        const W = 1 << 2;   
        /// execute enable
        const X = 1 << 3;   
        /// user accessability
        const U = 1 << 4;   
        /// ?
        const G = 1 << 5;   
        /// Accessed
        const A = 1 << 6;   
        /// Dirty
        const D = 1 << 7;   
    }
}


/// A pagetable entry for SV39 standard. Looked something like this:  
///` 63         5352                                                  1098          `  
///` | reserved  ||                        PPN                         ||| DAGU XWRV`  
///`[XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX`  
#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize
}


impl PageTableEntry {
    /// Construct a new pagetable entry with ppn and flags.
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize
        }
    }

    /// Construct a new, empty pte.
    pub fn empty() -> Self {
        PageTableEntry {
            bits: 0
        }
    }

    pub fn modify_access(&mut self, flags: PTEFlags) {
        // preserve valid bits
        let mask: usize = 0xffff_ffff_ffff_ff01;
        self.bits &= mask;
        self.bits |= (flags.bits() as usize) & 0x0000_0000_0000_00fe;
    }

    /// Read the physical page number from pagetable entry.
    pub fn ppn(&self) -> PhysPageNum {
        PhysPageNum::from(self.bits >> 10 & 0xFFF_FFFF_FFFF)
    }

    /// Read the flags from pagetable entry.
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    /// Check if this PTE is valid
    pub fn valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }

    /// Check if the corresponding physical page is executable
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
    
    pub fn dirty(&self) -> bool {
        self.flags().contains(PTEFlags::D)
    }

    /// Check if the corresponding physical page is writbale
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    /// Check if the corresponding physical page is readable
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
}

/// The pagetable.
pub struct PageTable {
    /// The root physical page number for the pagetable, used in SATP
    root_ppn: PhysPageNum,
    /// Physical frames that this pagetable have in the whole memory layout.
    frames  : Vec<FrameTracker>
}

impl PageTable {
    /// Create a new and empty pagetable
    /// # Description
    /// Create a new and empty pagetable, alloc itself the root page.
    pub fn new() -> Self {
        let root = alloc_frame().unwrap();     // might panic when OOM, but who cares?
        PageTable {
            root_ppn: root.ppn,
            frames: vec![root]
        }
    }

    /// Get the SATP value of the pagetable.
    /// # Description
    /// Get the SATP value of the pagetable. Use to write into the SATP CSR, thus change the pagetable the MMU is using.
    pub fn get_satp(&self) -> usize {
        return 8usize << 60 | self.root_ppn.0;
    }

    /// Get the page table entry from the pagetable.
    /// # Description
    /// Similar to xv6, get pte representing vpn from pagetable, and create parent pte if not present.
    /// # Return
    /// Return a reference to the corrersponding page table entry
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

    /// Get the page table entry from the pagetable.
    /// # Description
    /// Get the page table entry from the pagetable. Return None if not mapped.
    /// # Return
    /// Return a reference to the corrersponding page table entry, or None if not found.
    pub fn walk(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let indexes = vpn.indexes();
        let mut ppn = self.root_ppn;
        for i in 0..3 {
            let pte = &mut ppn.read_pte()[indexes[i]];
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

    /// Map a vpn-ppn pair in the page table
    /// # Description
    /// Map a pair of virtual page and physical page, alone with specified flags.
    /// Panic on remapping.
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.walk_create(vpn);
        assert!(!pte.valid(), "{:?} has already been mapped.", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }


    /// Unmap a vpn-ppn pair in the page table
    /// # Description
    /// Unmap a pair of virtual page and physical page.
    /// Panic on unmapping not mapped memory.
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.walk_create(vpn);
        assert!(pte.valid(), "{:?} hasn't been mapped.", vpn);
        *pte = PageTableEntry::empty();
    }

    pub fn modify_access(&mut self, vpn: VirtPageNum, flags: PTEFlags) -> Option<()> {
        let pte = self.walk(vpn)?;
        // assert!(pte.valid(), "{:?} has already been mapped.", vpn);
        pte.modify_access(flags);
        Some(())
    }

    /// Read and construct a pagetable from SATP value.
    /// # Description
    /// Read and construct a pagetable from SATP value, for SATP contains the root_ppn info.
    /// # Return
    /// Return the corresponding pagetable.
    pub fn from_satp(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    /// Translate a virtual page number to the page table entry.
    /// # Description
    /// Translate a virtual page number to the page table entry, and return a clone of it. Return None if not found.
    /// # Return
    /// Some(PageTableEntry) containing a copy of the original pte, or None if not found.
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.walk(vpn).map(|pte| pte.clone())
    }


    /// Translate a virtual address to physical address.
    /// # Description
    /// Translate a virtual address to the physical address, and return a clone of it. Return None if not found.
    /// # Return
    /// Some(PhysAddr) containing a copy of the original pte, or None if not found.
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.walk(va.clone().to_vpn()).map(|pte| {
            return PhysAddr::from(pte.ppn()) + va.page_offset()
        })
    }
}


/// Tranlate a chunk of user memory into kernel space
/// # Description
/// Tranlate a user buffer into kernel space. Note that due to paging, the result is not continuous.
#[allow(unused)]
pub fn get_user_data(satp: usize, mut start: VirtAddr, len: usize) -> Vec<&'static mut [u8]> {
    let pagetable = PageTable::from_satp(satp);
    let end = start + len;
    let mut pages = Vec::new();
    while start < end {
        let mut vpn = start.to_vpn();
        let ppn = pagetable.translate(vpn).unwrap().ppn();
        vpn.step();
        let copy_end = min(vpn.into(), end);    // page end or buf end
        pages.push(&mut ppn.page_ptr()[
            start.page_offset()
            ..
            copy_end.page_offset()
        ]);
        start = copy_end;
    }

    return pages;
}

/// Get a UserBuffer in user space
/// # Description
/// Get a UserBuffer in user space. Modify to UserBuffer will modify the corresponding user space memory.
/// # Return
/// The userbuffer of corresponding area
#[allow(unused)]
pub fn get_user_buffer(satp: usize, start: VirtAddr, len: usize) -> UserBuffer {
    return UserBuffer::new(get_user_data(satp, start, len));
}

/// Write a object into user space.
/// # Description
/// Write a object into user space. Can cross page boundry
/// # Example
/// ```
/// let to_write: usize = 123456;
/// write_user_data(layout.get_satp(), 0x10000.into(), to_write);
/// ```
#[allow(unused)]
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
/// translate a virtual address to a certern object pointer
/// # Description
/// Derepcated, use layout::read_user_data and layout::write_user_data instead.
/// This might go wrone when the object is on the boundry of pages.
#[deprecated]
pub fn translate_user_va<T>(satp: usize, va: VirtAddr) -> *mut T {
    let pagetable = PageTable::from_satp(satp);
    let vpn = va.to_vpn();
    let ppn = pagetable.translate(vpn).unwrap().ppn();
    // HACK: FUCK ME that is evil
    return (&mut ppn.page_ptr()[va.page_offset()]) as *mut u8 as usize as *mut T;
}

// TODO: can optimize this. copy_from_slice until page boundry will be much faster
/// Get a c-style string from the user space.
/// # Description
/// Get a c-style string from the user space, that is, read until a `b'\0'` is encountered.  
/// Note that this function returns a clone of the original string.
/// # Return
/// A clone of the original c-style string in the user space, in a vector of bytes.
pub fn get_user_cstr(satp: usize, mut va: VirtAddr) -> String {
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        let byte: u8 = unsafe{*translate_user_va(satp, va)};
        if byte == 0 {break;}
        bytes.push(byte);
        va = va + 1;
    }
    let string = alloc::string::String::from_utf8_lossy(&bytes);
    return string.into_owned();
}