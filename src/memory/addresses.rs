/// Virtual address, Physical address, Physical Page Number, Virtual Page Number

use core::fmt::{self, Debug, Formatter};
use core::ops;
use crate::config::{PAGE_SIZE, PAGE_OFFSET};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysPageNum(pub usize);

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
    pub fn to_vpn(&self) -> VirtPageNum {
        return VirtPageNum(self.0 / PAGE_SIZE);
    }

    pub fn to_vpn_ceil(&self) -> VirtPageNum {
        return VirtPageNum((self.0 - 1) / PAGE_SIZE + 1);
    }

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

impl ops::Add<VirtAddr> for usize {
    type Output = VirtAddr;
    fn add(self, rhs: VirtAddr) -> VirtAddr {
        return rhs + self;
    }
}

/// utility for physaddr
impl PhysAddr {
    pub fn to_vpn(&self) -> PhysPageNum {
        return PhysPageNum(self.0 / PAGE_SIZE);
    }

    pub fn to_vpn_ceil(&self) -> PhysPageNum {
        return PhysPageNum((self.0 - 1) / PAGE_SIZE + 1);
    }

    pub fn page_offset(&self) -> usize {
        return self.0 % PAGE_SIZE;
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

impl ops::Add<PhysAddr> for usize {
    type Output = PhysAddr;
    fn add(self, rhs: PhysAddr) -> PhysAddr {
        return rhs + self;
    }
}