//! Definition of all the primitive data type in memory management module.  
//! Including Virtual address, Physical address, Physical Page Number, Virtual Page Number and Page table entry.  
//! Operator overloading is also put here.
#![allow(dead_code)]

use core::fmt::{self, Debug, Formatter};
use core::ops;
use crate::config::{PAGE_SIZE, PAGE_OFFSET};
use super::PageTableEntry;
use core::mem::size_of;
use crate::utils::{
    StepByOne,
    SimpleRange
};

/// The representation of physical address (used in kernel space).
//  63                                                            12 11           0
//  |                           PPN                                | |   offset   |
// [XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX
#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

/// The representation of virtua; address (used in user space).
//  63                           3938       3029       2120       12 11           0
//  |            EXT              ||   L2    ||   L1    ||    L0   | |   offset   |
// [XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX
#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

/// The representation of physical page number.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);

/// The representation of virtual page number.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPageNum(pub usize);

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(" VA: {:#x}", self.0))
    }
}
impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VPN: {:#x}", self.0))
    }
}
impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(" PA: {:#x}", self.0))
    }
}
impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PPN: {:#x}", self.0))
    }
}

/// convertions
impl From<usize> for PhysAddr       { fn from(num: usize) -> Self { Self(num) } }
impl From<usize> for VirtAddr       { fn from(num: usize) -> Self { Self(num) } }
impl From<usize> for VirtPageNum    { fn from(num: usize) -> Self { Self(num) } }
impl From<usize> for PhysPageNum    { fn from(num: usize) -> Self { Self(num) } }

impl From<PhysAddr> for PhysPageNum {
    fn from(pa: PhysAddr) -> Self {
        return Self(pa.0 >> PAGE_OFFSET);
    }
}

impl From<PhysPageNum> for PhysAddr {
    fn from(ppn: PhysPageNum) -> Self {
        return Self(ppn.0 << PAGE_OFFSET);
    }
}

impl From<VirtAddr> for VirtPageNum {
    fn from(va: VirtAddr) -> Self {
        return Self(va.0 >> PAGE_OFFSET);
    }
}

impl From<VirtPageNum> for VirtAddr {
    fn from(vpn: VirtPageNum) -> Self {
        return Self(vpn.0 << PAGE_OFFSET);
    }
}

/// utility for virtaddr
impl VirtAddr {
    /// Truncate down to Virtual Page Number.
    /// # Description
    /// Truncate down to Virtual Page Number.
    /// # Return
    /// Return the virtual page number of the virtual address.
    pub fn to_vpn(&self) -> VirtPageNum {
        return VirtPageNum(self.0 / PAGE_SIZE);
    }

    /// Truncate up to Virtual Page Number.
    /// # Description
    /// Truncate up to Virtual Page Number.
    /// # Return
    /// Return the virtual page number of the virtual address, rounded up.
    pub fn to_vpn_ceil(&self) -> VirtPageNum {
        return VirtPageNum((self.0 - 1) / PAGE_SIZE + 1);
    }

    /// Get the page offset of the virtual address
    /// # Description
    /// Get the page offset of the virtual address
    /// # Return
    /// The page offset of the virtual address
    pub fn page_offset(&self) -> usize {
        return self.0 % PAGE_SIZE;
    }
}

impl ops::Add<usize> for VirtAddr {
    type Output = VirtAddr;
    fn add(self, rhs: usize) -> VirtAddr {
        return VirtAddr(self.0 + rhs);
    }
}

impl ops::AddAssign<usize> for VirtAddr {
    fn add_assign(&mut self, rhs: usize) { 
        self.0 += rhs;
    }
}

impl ops::Sub<usize> for VirtAddr {
    type Output = VirtAddr;
    fn sub(self, rhs: usize) -> VirtAddr {
        return VirtAddr(self.0 - rhs);
    }
}

impl ops::Sub<VirtAddr> for VirtAddr {
    type Output = usize;
    fn sub(self, rhs: VirtAddr) -> usize {
        return self.0 - rhs.0;
    }
}

impl ops::SubAssign<usize> for VirtAddr {
    fn sub_assign(&mut self, rhs: usize) { 
        self.0 -= rhs;
    }
}

impl ops::Add<VirtAddr> for usize {
    type Output = VirtAddr;
    fn add(self, rhs: VirtAddr) -> VirtAddr {
        return rhs + self;
    }
}

/// utility for physaddr
impl PhysAddr {
    /// Truncate down to physical Page Number.
    /// # Description
    /// Truncate down to physical Page Number.
    /// # Return
    /// Return the physical page number of the physical address.
    pub fn to_ppn(&self) -> PhysPageNum {
        return PhysPageNum(self.0 / PAGE_SIZE);
    }

    /// Truncate up to physical Page Number.
    /// # Description
    /// Truncate up to physical Page Number.
    /// # Return
    /// Return the physical page number of the physical address, rounded up.
    pub fn to_ppn_ceil(&self) -> PhysPageNum {
        return PhysPageNum((self.0 - 1) / PAGE_SIZE + 1);
    }

    /// Get the page offset of the physical address
    /// # Description
    /// Get the page offset of the physical address
    /// # Return
    /// The page offset of the physical address
    pub fn page_offset(&self) -> usize {
        return self.0 % PAGE_SIZE;
    }

    /// Get an mutable object from corresponding physical address.
    /// # Description
    /// Get an mutable object from corresponding physical address.  
    /// Warning: Will not work properly on page edge, for continuous virtual pages are not always continuous in physical memory.
    /// # Return
    /// A mutable reference of the object at that physical memory
    #[deprecated]
    pub fn get_mut<T>(&self) -> &'static mut T {
        unsafe {
            return (self.0 as *mut T).as_mut().unwrap();
        }
    }
}

impl ops::Add<usize> for PhysAddr {
    type Output = PhysAddr;
    fn add(self, rhs: usize) -> PhysAddr {
        return PhysAddr(self.0 + rhs);
    }
}

impl ops::AddAssign<usize> for PhysAddr {
    fn add_assign(&mut self, rhs: usize) { 
        self.0 += rhs;
    }
}

impl ops::Sub<usize> for PhysAddr {
    type Output = PhysAddr;
    fn sub(self, rhs: usize) -> PhysAddr {
        return PhysAddr(self.0 - rhs);
    }
}

impl ops::Sub<PhysAddr> for PhysAddr {
    type Output = usize;
    fn sub(self, rhs: PhysAddr) -> usize {
        return self.0 - rhs.0;
    }
}

impl ops::SubAssign<usize> for PhysAddr {
    fn sub_assign(&mut self, rhs: usize) { 
        self.0 -= rhs;
    }
}

impl ops::Add<PhysAddr> for usize {
    type Output = PhysAddr;
    fn add(self, rhs: PhysAddr) -> PhysAddr {
        return rhs + self;
    }
}

impl ops::Add<usize> for PhysPageNum {
    type Output = PhysPageNum;
    fn add(self, rhs: usize) -> PhysPageNum {
        return PhysPageNum(self.0 + rhs);
    }
}

impl ops::AddAssign<usize> for PhysPageNum {
    fn add_assign(&mut self, rhs: usize) { 
        self.0 += rhs;
    }
}

impl ops::Sub<usize> for PhysPageNum {
    type Output = PhysPageNum;
    fn sub(self, rhs: usize) -> PhysPageNum {
        return PhysPageNum(self.0 - rhs);
    }
}

impl ops::SubAssign<usize> for PhysPageNum {
    fn sub_assign(&mut self, rhs: usize) { 
        self.0 -= rhs;
    }
}

impl ops::Add<PhysPageNum> for usize {
    type Output = PhysPageNum;
    fn add(self, rhs: PhysPageNum) -> PhysPageNum {
        return rhs + self;
    }
}

/// utils for the physical page number
impl PhysPageNum {
    /// Get address of head of the physical page.
    /// # Description
    /// Get the address of first byte in the physical page
    /// # Return
    /// The physical address the first byte in the physical page.
    pub fn head_pa(&self) -> PhysAddr {
        return PhysAddr(self.0 << PAGE_OFFSET);
    }

    /// Get raw bytes from physical page as array
    /// # Description
    /// Get raw bytes from physical page as array.  
    /// Read/write to the returned array will change the corresponding memory.
    /// # Return
    /// `[u8; PAGE_SIZE]` representing corresponding memory area
    pub fn page_ptr(&self) -> &'static mut [u8] {
        unsafe {
            return core::slice::from_raw_parts_mut(self.head_pa().0 as *mut u8, PAGE_SIZE);
        }
    }


    /// Read the page table entries from the physical page
    /// # Description
    /// Read the page table entries from the physical page, according to RISC-V SV39 standard. 
    /// # Return
    /// A list of PageTableEntries.
    pub fn read_pte(&self) -> &'static mut [PageTableEntry] {
        unsafe {
            return core::slice::from_raw_parts_mut(self.head_pa().0 as *mut PageTableEntry, PAGE_SIZE / size_of::<PageTableEntry>());
        }
    }
}

impl ops::Add<usize> for VirtPageNum {
    type Output = VirtPageNum;
    fn add(self, rhs: usize) -> VirtPageNum {
        return VirtPageNum(self.0 + rhs);
    }
}

impl ops::AddAssign<usize> for VirtPageNum {
    fn add_assign(&mut self, rhs: usize) { 
        self.0 += rhs;
    }
}

impl ops::Sub<usize> for VirtPageNum {
    type Output = VirtPageNum;
    fn sub(self, rhs: usize) -> VirtPageNum {
        return VirtPageNum(self.0 - rhs);
    }
}

impl ops::Sub<VirtPageNum> for VirtPageNum {
    type Output = usize;
    fn sub(self, rhs: VirtPageNum) -> usize {
        return self.0 - rhs.0;
    }
}

impl ops::SubAssign<usize> for VirtPageNum {
    fn sub_assign(&mut self, rhs: usize) { 
        self.0 -= rhs;
    }
}

impl ops::Add<VirtPageNum> for usize {
    type Output = VirtPageNum;
    fn add(self, rhs: VirtPageNum) -> VirtPageNum {
        return rhs + self;
    }
}

impl VirtPageNum {
    /// Get the L2/L1/L0 bits from the virtual page number
    /// # Description
    /// Get the L2/L1/L0 bits from the virtual page number, which looks something like this:  
    /// ` 63                           3938       3029       2120       12 11           0`  
    /// ` |            EXT              ||   L2    ||   L1    ||    L0   | |   offset   |`  
    /// `[XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX XXXX`  
    /// # Example
    /// ```
    /// let vpn: VirtPageNum = va.into();
    /// let (l2, l1, l0) = vpn.indexes();
    /// ```
    /// # Return
    /// Returns the three indexes of the virtual page number
    pub fn indexes(&self) -> [usize; 3] {
        return [
            (self.0 >> 18) & 0b1_1111_1111,
            (self.0 >>  9) & 0b1_1111_1111,
            (self.0 >>  0) & 0b1_1111_1111,
        ];
    }
}

impl StepByOne for VirtAddr {
    fn step(&mut self) { self.0 += 1; }
}

impl StepByOne for PhysAddr {
    fn step(&mut self) { self.0 += 1; }
}

impl StepByOne for VirtPageNum {
    fn step(&mut self) { self.0 += 1; }
}

impl StepByOne for PhysPageNum {
    fn step(&mut self) { self.0 += 1; }
}

pub type VARange    = SimpleRange<VirtAddr>;
pub type PARange    = SimpleRange<PhysAddr>;
pub type VPNRange   = SimpleRange<VirtPageNum>;
pub type PPNRange   = SimpleRange<PhysPageNum>;