//! An abstract of memory layout, both kernel space and user space.  
//! Closly binded to page table stuff.
use super::{
    VPNRange,
    VirtPageNum,
    VirtAddr,
    PhysPageNum,
    PhysAddr,
    FrameTracker,
    PageTable,
    PageTableEntry,
    PTEFlags,
    alloc_frame,
    UserBuffer
};
use core::mem::size_of;
use _core::convert::TryInto;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use bitflags::*;
use crate::config::*;
use crate::fs::{File, SeekOp};
use crate::process::{AuxHeader, AuxType, CloneFlags};
use core::cmp::min;
use crate::utils::{SimpleRange, StepByOne};
use lazy_static::*;
use alloc::sync::Arc;
use spin::Mutex;
use riscv::register::satp;
use core::fmt::{self, Debug, Formatter};

lazy_static! {
    /// The kernel space memory layout.
    pub static ref KERNEL_MEM_LAYOUT: Arc<Mutex<MemLayout>> = Arc::new(Mutex::new(MemLayout::new_kernel()));
}

/// Get the SATP value of the kernel space
/// # Description
/// Get the SATP value of the kernel space, can be used to load to CSR SATP or used to extract pagetable from memory.
/// # Returns
/// The SATP value representing the kernel space page table.
pub fn kernel_satp() -> usize {
    return KERNEL_MEM_LAYOUT.lock().get_satp();
}

/// Maptype of a segment in the layout
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum MapType {
    /// Identity mapping, means that the virtual address and the physical address is the same.
    Identity,
    /// Normal mapping, physical pages are from `alloc_frame()`
    Framed,
    /// Virtual memory layout 
    VMA,
}

bitflags! {
    /// Segment flags indicaing privilege.
    pub struct SegmentFlags: u8 {
        /// Can this segment be read?
        const R = 1 << 1;
        /// Can this segment be write?
        const W = 1 << 2;
        /// Can this segment be executed?
        const X = 1 << 3;
        /// Can this segment be accessed from user mode?
        const U = 1 << 4;
    }
}

bitflags! {
    /// VMA flags indicaing privilege.
    pub struct VMAFlags: u8 {
        /// Can this segment be read?
        const R = 1 << 1;
        /// Can this segment be write?
        const W = 1 << 2;
        /// Can this segment be executed?
        const X = 1 << 3;
    }
}

/// Representing a continuous segment in the memroy layout.  
/// For example, data segments/text segments in the user program.
pub struct Segment {
    pub head_offset: usize,
    /// range of the Segment, [range.start()..range.end())
    pub range   : VPNRange,
    /// allocated physical frames, aloneside with their virtual page number.  
    /// It holds the FrameTracker so that it's not dropped.
    pub frames  : BTreeMap<VirtPageNum, FrameTracker>,
    /// the mapping type (identity or framed)
    pub map_type: MapType,
    /// the flags
    pub seg_flags   : SegmentFlags,
    /// vma flags
    pub vma_flags : VMAFlags,
    /// the mmap file
    pub file    : Option<Arc<dyn File>>,
    /// the mmap file offset
    pub offset  : usize
}

impl Debug for Segment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Segment: 0x{:x}..0x{:x}", self.range.get_start().0 << 12, self.range.get_end().0 << 12))
    }
}

impl Segment {
    /// Construct a new Segment.
    /// # Description
    /// Construct a new semgnet. Note that the physical frames are not allocated yet, 
    /// for mapping needs a pagetable.
    /// # Example
    /// ```
    /// let segment: Segment = Segment::new(0x10010000.into(), 0x10020000.into(), MapType::Identity, SegmentFlags::R);
    /// ```
    /// # Return
    /// Returns a new, unmapped segment.
    pub fn new(start: VirtAddr, stop: VirtAddr, map_type: MapType, seg_flags: SegmentFlags, vma_flags: VMAFlags, file: Option<Arc<dyn File>>, offset: usize) -> Self {
        verbose!("New Segment: {:?} <=> {:?}", start.to_vpn(), stop.to_vpn_ceil());
        Self {
            head_offset: start.0 % PAGE_SIZE,
            range   : VPNRange::new(start.to_vpn(), stop.to_vpn_ceil()),
            frames  : BTreeMap::new(),
            map_type,
            seg_flags,
            vma_flags,
            file,
            offset
        }
    }

    /// Alloc and map a page in the segment
    /// # Description
    /// Alloc and map the page `vpn` in the segment, using the `pagetable` as pagetable
    /// # Example
    /// ```
    /// let mut segment: Segment = Segment::new(0x10010000.into(), 0x10020000.into(), MapType::Identity, SegmentFlags::R);
    /// segment.map_page(pagetable, VirtPageNum::From(VirtAddr::From(0x10010000)));
    /// ```
    pub fn map_page(&mut self, pagetable: &mut PageTable, vpn: VirtPageNum) -> Result<(), &'static str> {
        if vpn < self.range.get_start() || vpn >= self.range.get_end() {
            return Err("Trying to map a page that is not in this Segment.");
        }
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identity => {
                ppn = PhysPageNum(vpn.0);
                pagetable.map(vpn, ppn, PTEFlags::from_bits(self.seg_flags.bits).unwrap());
                Ok(())
            },
            MapType::Framed => {
                if let Some(frame) = alloc_frame() {
                    ppn = frame.ppn;
                    self.frames.insert(vpn, frame);
                    pagetable.map(vpn, ppn, PTEFlags::from_bits(self.seg_flags.bits).unwrap());
                    // verbose!("Mapped framed page: {:?}<=>{:?}, flag {:?}", vpn, ppn, PTEFlags::from_bits(self.segFlags.bits).unwrap());
                    Ok(())
                } else {
                    Err("OOM!")
                }
                
            },
            MapType::VMA => {
                // let frame = alloc_frame().unwrap();
                // ppn = frame.ppn;
                // self.frames.insert(vpn, frame);
                // pagetable.map(vpn, ppn, PTEFlags::from_bits(self.segFlags.bits).unwrap());
                verbose!("Lazy map, not mapping");
                Ok(())
            }
        }
    }

    pub fn map_lazy_vma(&mut self, pagetable: &mut PageTable, va: VirtAddr) -> Result<(), &'static str> {
        let vpn = va.to_vpn();
        if vpn < self.range.get_start() || vpn >= self.range.get_end() {
            error!("Trying to map a page that is not in this Segment.");
            return Err("Trying to map a page that is not in this Segment.")
        }
        
        let frame = alloc_frame().unwrap();
        let ppn = frame.ppn;

        let bytes = ppn.page_ptr();
        let optfile = self.file.clone().unwrap();
        let inner_file = optfile.to_common_file().unwrap();
        let cur = inner_file.get_cursor()?;
        let offset: isize = (va - VirtAddr::from(self.range.get_start()) - self.offset).try_into().unwrap();
        let offset = offset - offset % PAGE_SIZE as isize;
        inner_file.seek(offset, SeekOp::SET).unwrap();
        let res = inner_file.read(bytes);
        inner_file.seek(cur as isize, SeekOp::SET).unwrap();

        if let Err(msg) = res {
            error!("{}", msg);
            return Err(msg);
        }

        self.frames.insert(vpn, frame);
        verbose!("Lazy mapped: {:?} <=> {:?}", vpn, ppn);
        pagetable.map(vpn, ppn, PTEFlags::from_bits(self.vma_flags.bits).unwrap() | PTEFlags::U);
        Ok(())
    }
    
    pub fn adjust_end(&mut self, pagetable: &mut PageTable, new_end: VirtPageNum) -> Option<()> {
        // We need to align the end to the 4K border of the page
        // let new_end = self.range.get_end() + (VirtAddr::from(sz)).0;
        // self.range.set_end(new_end);
        if self.map_type != MapType::Framed {
            panic!("Only framed segments can be adjusted.");
        }
        if new_end < self.range.get_end() {
            for i in new_end.0..self.range.get_end().0 {
                self.unmap_page(pagetable, i.into());
            }
            self.range.set_end(new_end);
            Some(())
        } else if new_end > self.range.get_end() {
            let original_end = self.range.get_end();
            self.range.set_end(new_end);
            for i in original_end.0..new_end.0 {
                if let Err(msg) = self.map_page(pagetable, i.into()) {
                    fatal!("segment resize failed: {}", msg);
                    for j in original_end.0..i {
                        self.unmap_page(pagetable, j.into())
                    }
                    return None;
                }
            }
            Some(())
        } else {
            Some(())
        }
    }

    /// Free and unmap a page in the segment
    /// # Description
    /// Free and unmap the page `vpn` in the segment, using the `pagetable` as pagetable.  
    /// By removing the corresponding FrameTracker, the physical frame is automatically freed.
    /// # Example
    /// ```
    /// segment.unmap_page(pagetable, VirtPageNum::From(VirtAddr::From(0x10010000)));
    /// ```
    #[allow(dead_code)]
    pub fn unmap_page(&mut self, pagetable: &mut PageTable, vpn: VirtPageNum) {
        // verbose!("Unmapping {:?}", vpn);
        if self.map_type == MapType::Framed {
            verbose!("Unmapping page {:?}", vpn);
            self.frames.remove(&vpn);
        } else if self.map_type == MapType::VMA {
            verbose!("Unmapping vma");
            if let Some(pte) = pagetable.walk(vpn) {
                verbose!("pte find: valid: {}, ditry: {}", pte.valid(), pte.dirty());
                if self.vma_flags.contains(VMAFlags::W) && pte.dirty() && pte.valid() {
                    let file = self.file.clone().unwrap();
                    let fs_file = file.to_common_file().unwrap();
                    let cur = fs_file.get_cursor().unwrap();
                    let offset = (vpn - self.range.get_start()) * PAGE_SIZE + self.offset;
                    fs_file.seek(offset as isize, SeekOp::SET).unwrap(); 
                    verbose!("Unmap page VMA write back, from {:?}({:?})", vpn, PhysPageNum::from(pagetable.translate_va(vpn.into()).unwrap()));
                    let page_ptr = PhysPageNum::from(pagetable.translate_va(vpn.into()).unwrap()).page_ptr();
                    if let Err(msg) = fs_file.write(page_ptr) {
                        error!("Failed to write to file: {}", msg);
                    }
                    fs_file.seek(cur as isize, SeekOp::SET).unwrap();
                    self.frames.remove(&vpn);
                } else {
                    verbose!("Lazy page detected, not unmapping");
                    return;
                }
            } else {
                verbose!("Lazy page detected, not unmapping");
                return;
            }
        }
        pagetable.unmap(vpn);
    }

    /// Alloc and map all page in the segment
    /// # Description
    /// Alloc and map all pages, using the `pagetable` as pagetable
    /// # Example
    /// ```
    /// let mut segment: Segment = Segment::new(0x10010000.into(), 0x10020000.into(), MapType::Identity, SegmentFlags::R);
    /// segment.map_pages(pagetable);
    /// ```
    pub fn map_pages(&mut self, pagetable: &mut PageTable) {
        for vpn in self.range {
            self.map_page(pagetable, vpn).unwrap();
        }
    }

    /// Free and unmap all pages in the segment
    /// # Description
    /// Free and unmap all pages in the segment, using the `pagetable` as pagetable.  
    /// By removing the corresponding FrameTracker, the physical frame is automatically freed.
    /// # Example
    /// ```
    /// segment.unmap_pages(pagetable);
    /// ```
    #[allow(dead_code)]
    pub fn unmap_pages(&mut self, pagetable: &mut PageTable) {
        for vpn in self.range {
            self.unmap_page(pagetable, vpn);
        }
    }

    /// Write data to a segment.
    /// # Description
    /// Write data to a segment. Ths segment need to be mapped before.  
    /// Also, the data should be no longer then the segment
    /// # Example
    /// ```
    /// let mut segment = Segment::new(0x10010000.into(), 0x10020000.into(), MapType::Identity, SegmentFlags::R);
    /// segment.write(pagetable, &[1u8; 0x10000]);
    /// ```
    pub fn write(&mut self, pagetable: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed, "Error: cannot write to identity mapped segment.");
        assert!(data.len() <= (self.range.get_end() - self.range.get_start()) * PAGE_SIZE, "Data too long to be written into segment.");
        let mut data_i: usize = 0;
        let mut vpn_i = self.range.get_start();
        let len = data.len();

        // copy first page with head_offset first
        let src1 = &data[data_i..min(data_i + PAGE_SIZE - self.head_offset, len)];
        let dst1: &mut [u8];
        if let Some(ppn) = pagetable.translate(vpn_i) {     // TODO: Isn't it the same to use that BTreeMap?
            dst1 = &mut ppn.ppn().page_ptr()[self.head_offset..self.head_offset+src1.len()];
        } else {
            panic!("{:?} hasn't been mapped.", vpn_i);
        }
        dst1.copy_from_slice(src1);
        vpn_i.step();
        data_i += min(data_i + PAGE_SIZE - self.head_offset, len);

        while data_i < len {
            let src = &data[data_i..min(data_i + PAGE_SIZE, len)];
            let dst: &mut [u8];
            if let Some(ppn) = pagetable.translate(vpn_i) {     // TODO: Isn't it the same to use that BTreeMap?
                dst = &mut ppn.ppn().page_ptr()[..src.len()];
            } else {
                panic!("{:?} hasn't been mapped.", vpn_i);
            }
            dst.copy_from_slice(src);
            vpn_i.step();
            data_i += PAGE_SIZE;
        }
    }

    /// Clone a Segment from another Segment.
    /// # Description
    /// Clone a Segment from another Segment. The new segment will be unmapped and need to be mapped with another pagetable.
    pub fn clone_from(src: &Segment) -> Self {
        Self {
            head_offset: src.head_offset,
            range: VPNRange::new(
                src.range.get_start(),
                src.range.get_end()
            ),
            frames: BTreeMap::new(),
            map_type: src.map_type,
            seg_flags: src.seg_flags,
            vma_flags: src.vma_flags,
            file: src.file.clone(),
            offset: src.offset
        }
    }
}

/// The memory layout, for kernel space or user space.
pub struct MemLayout {
    /// The pagetable of this memory layout.
    pagetable   : PageTable,
    /// The segments in this memory layout.
    pub segments    : Vec<Arc<Mutex<Segment>>>,
}

impl MemLayout {
    /// Return a new, empty memory layout.
    pub fn new() -> Self {
        Self {
            pagetable   : PageTable::new(),
            segments    : Vec::new(),
        }
    }

    /// Fork a memory layout
    /// # Description
    /// Fork a memory layout from a user space memory layout.  
    /// They will have exactly the same virtual memory layout, yet on different physical pages.
    /// # Return
    /// Forked memory layout.
    #[deprecated]
    #[allow(unused)]
    pub fn fork_from_user(src: &MemLayout) -> Self {
        let mut layout = Self::new();
        layout.map_trampoline();
        for m_segment in src.segments.iter() {
            let segment = m_segment.lock();
            let new_segment = Segment::clone_from(&segment);
            layout.add_segment(Arc::new(Mutex::new(new_segment)));
            for vpn in segment.range {
                let src_ppn = src.translate(vpn).unwrap().ppn();
                let dst_ppn = layout.translate(vpn).unwrap().ppn();
                dst_ppn.page_ptr().copy_from_slice(src_ppn.page_ptr());
            }
        }
        return layout;
    }

    pub fn clone_from_user(src: &MemLayout, flags: CloneFlags)-> Self {
        let mut layout = Self::new();
        layout.map_trampoline();
        for m_segment in src.segments.iter() {
            let segment = m_segment.lock();
            if flags.contains(CloneFlags::VM) {
                layout.add_segment(m_segment.clone());
            } else {
                let new_segment = Segment::clone_from(&segment);
                layout.add_segment(Arc::new(Mutex::new(new_segment)));
                for vpn in segment.range {
                    let src_ppn = src.translate(vpn).unwrap().ppn();
                    let dst_ppn = layout.translate(vpn).unwrap().ppn();
                    dst_ppn.page_ptr().copy_from_slice(src_ppn.page_ptr());
                }
            }
        }
        return layout;
    }

    pub fn alter_segment(&mut self, old_end: VirtPageNum, new_end: VirtPageNum) -> Option<()> {
        for m_segment in self.segments.iter_mut() {
            let mut segment = m_segment.lock();
            if segment.range.get_end() == old_end {
                return segment.adjust_end(&mut self.pagetable, new_end)
            }
        }
        error!("No segment end with {:?}", old_end);
        None
    }

    
    pub fn modify_access(&mut self, start: VirtAddr, len: usize, flags: PTEFlags, grow_up: bool, grow_down: bool) -> Option<()> {
        if grow_up || grow_down {
            // allocated, change access
            for m_seg in self.segments.iter() {
                let seg = m_seg.lock();
                if seg.range.get_start() <= start.into() && seg.range.get_end() > start.into() {
                    if seg.map_type != MapType::Framed {
                        fatal!("Cannot change access to non-Framed segments!");
                        return None;
                    }
                    let range = if grow_up {
                        SimpleRange::new(start.to_vpn(), seg.range.get_end())
                    } else {
                        SimpleRange::new(seg.range.get_start(), start.to_vpn_ceil())
                    };
                    for page in range {
                        self.pagetable.modify_access(page, flags)?;
                    }
                    return Some(())
                }
            }
            // not allocated, this won't work
            fatal!("vpn {:?} not in any segments!", start);
            return None;
        }
        // TODO: no crossing segment boundary for now... change?
        for m_seg in self.segments.iter() {
            let seg = m_seg.lock();
            // allocated, change access
            if seg.range.get_start() <= start.into() && seg.range.get_end() > start.into() {
                if seg.map_type != MapType::Framed {
                    fatal!("Cannot change access to non-Framed segments!");
                    return None;
                }
                for vpn in SimpleRange::new(start.to_vpn(), (start + len).to_vpn_ceil()) {
                    self.pagetable.modify_access(vpn, flags)?;
                }
                return Some(());
            }
        }
        // not allocated, allocate new Segment
        let mut seg_flags = SegmentFlags::U;
        if flags.contains(PTEFlags::U) {
            seg_flags |= SegmentFlags::U;
        }
        if flags.contains(PTEFlags::W) {
            seg_flags |= SegmentFlags::W;
        }
        if flags.contains(PTEFlags::R) {
            seg_flags |= SegmentFlags::R;
        }
        if flags.contains(PTEFlags::X) {
            seg_flags |= SegmentFlags::X;
        }
        self.add_segment(Arc::new(Mutex::new(
            Segment::new(
                start, 
                start + len, 
                MapType::Framed, 
                seg_flags, 
                VMAFlags::empty(), 
                None, 
                0)
        )));
        Some(())
    }
    
    /// Activate the memory layout as kernel memory layout
    /// # Description
    /// Activate the SV39 virtual memory mode and use this memory layout as kernel memory layout
    pub fn activate(&self) {
        verbose!("Kernel switching to virtual memory space...");
        let satp = self.pagetable.get_satp();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
        if satp::read().mode() != satp::Mode::Sv39 {
            fatal!("Failed switch to SV39!");
            warning!("This seems to be a known issue with k210 + rustsbi.");
        } else {
            info!("Kernel virtual memory layout has been activated.");
        }
    }

    /// Add a segment to this layout.
    /// # Description
    /// Add a segment to this layout, map it and allocate corresponding physical pages.
    pub fn add_segment(&mut self, segment: Arc<Mutex<Segment>>) {
        segment.lock().map_pages(&mut self.pagetable);
        self.segments.push(segment);
    }

    /// Add a segment to this layout, and copy data into it.
    /// # Description
    /// Add a segment to this layout, map it and allocate corresponding physical pages.  
    /// After that, we copy `data` into this segment.  
    /// Extra useful when loading user elf segments
    /// # Example
    /// ```
    /// let (data, start, stop) = get_user_data_segment();    // this is a psudo-function
    /// let segment = Segment::new(start, stop, MapType::Identity, SegmentFlags::R | SegmentFlags::W);
    /// layout.add_segment_with_source(segment, data);
    /// ```
    pub fn add_segment_with_source(&mut self, mut segment: Segment, data: &[u8]) {
        segment.map_pages(&mut self.pagetable);
        segment.write(&mut self.pagetable, data);
        verbose!("add_segment_with_source Mapping Segment with start = {:?}, end = {:?}", VirtAddr::from(segment.range.get_start()), VirtAddr::from(segment.range.get_end()));
        self.segments.push(Arc::new(Mutex::new(segment)));
    }

    /// Construct a new kernel memory layout
    /// # Description
    /// Construct a new kernel memory layout, including identity map of all physical memory, kernel segments, trampoline and MMIO region.
    pub fn new_kernel() -> Self {
        debug!("Building kernel memory layout...");
        extern "C" {
            fn stext();
            fn etext();
            fn srodata();
            fn erodata();
            fn sdata();
            fn edata();
            fn sbss_with_stack();
            fn ebss();
            fn ekernel();
        }
        
        let mut layout = Self::new();
        verbose!("Mapping trampoline...");
        layout.map_trampoline();
        
        verbose!("Mapping .text...");
        layout.add_segment(
            Arc::new(Mutex::new(
                Segment::new(
                    VirtAddr::from(stext as usize), 
                    VirtAddr::from(etext as usize),
                    MapType::Identity,
                    SegmentFlags::R | SegmentFlags::X,
                    VMAFlags::empty(),
                    None,
                    0
                )
            ))
        );
        debug!(".text mapped @ 0x{:X} ~ 0x{:X} (identity), R-X-.", stext as usize, etext as usize);
        
        verbose!("Mapping .rodata...");
        layout.add_segment(
            Arc::new(Mutex::new(
                Segment::new(
                    VirtAddr::from(srodata as usize), 
                    VirtAddr::from(erodata as usize),
                    MapType::Identity,
                    SegmentFlags::R | SegmentFlags::X,
                    VMAFlags::empty(),
                    None,
                    0
                )
            ))
        );
        debug!(".rodata mapped @ 0x{:X} ~ 0x{:X} (identity), R-X-.", srodata as usize, erodata as usize);
        
        verbose!("Mapping .data...");
        layout.add_segment(
            Arc::new(Mutex::new(
                Segment::new(
                    VirtAddr::from(sdata as usize), 
                    VirtAddr::from(edata as usize),
                    MapType::Identity,
                    SegmentFlags::R,
                    VMAFlags::empty(),
                    None,
                    0
                )
            ))
        );
        debug!(".data mapped @ 0x{:X} ~ 0x{:X} (identity), R---.", sdata as usize, edata as usize);
        
        verbose!("Mapping .bss...");
        layout.add_segment(
            Arc::new(Mutex::new(
                Segment::new(
                    VirtAddr::from(sbss_with_stack as usize), 
                    VirtAddr::from(ebss as usize),
                    MapType::Identity,
                    SegmentFlags::R | SegmentFlags::W,
                    VMAFlags::empty(),
                    None,
                    0
                )
            ))
        );
        debug!(".bss mapped @ 0x{:X} ~ 0x{:X} (identity), RW--.", sbss_with_stack as usize, sbss_with_stack as usize);
        
        verbose!("Mapping rest physical memory as identical...");
        layout.add_segment(
            Arc::new(Mutex::new(
                Segment::new(
                    VirtAddr::from(ekernel as usize), 
                    VirtAddr::from(MEM_END),
                    MapType::Identity,
                    SegmentFlags::R | SegmentFlags::W,
                    VMAFlags::empty(),
                    None,
                    0
                )
            ))
        );
        debug!("Physical memory mapped @ 0x{:X} ~ 0x{:X} (identity), RW--.", ekernel as usize, MEM_END);

        verbose!("Mapping MMIO...");
        for pair in MMIO {
            layout.add_segment(
                Arc::new(Mutex::new(
                    Segment::new(
                        (*pair).0.into(),
                        ((*pair).0 + (*pair).1).into(),
                        MapType::Identity,
                        SegmentFlags::R | SegmentFlags::W,
                        VMAFlags::empty(),
                        None,
                        0
                    )
                ))
            );
            debug!("MMIO mapped @ 0x{:X} ~ 0x{:X} (identity), RW--.", (*pair).0, (*pair).0 + (*pair).1);
        }
        info!("Kernel memory layout initilized.");

        return layout;
    }

    /// Construct a new user memory layout
    /// # Description
    /// Construct a new user memory layout, including all elf segments, user stacks and trampoline.  
    /// Also can use bare bin file for compatbility.
    // todo: no kernel panic on user's fault -- just fail it's syscall. use a Result to wrap the return value.
    pub fn new_elf(elf_data: &[u8]) -> (Self, usize, usize, usize, Vec<AuxHeader>) {
        let mut layout = Self::new();
        layout.map_trampoline();
        let mut data_top = 0;
        if let Ok(elf) = xmas_elf::ElfFile::new(elf_data) {
            debug!("ELF parsed!");
            // debug!("header: {}", elf.header);
            // map segments
            for i in 0..elf.header.pt2.ph_count() {
                let program_header = elf.program_header(i).unwrap();
                if program_header.get_type().unwrap() == xmas_elf::program::Type::Load {
                    let start = VirtAddr::from(program_header.virtual_addr() as usize);
                    let stop = VirtAddr::from((program_header.virtual_addr() + program_header.mem_size()) as usize);
                    let mut segment_flags = SegmentFlags::U;
                    if program_header.flags().is_read() {
                        segment_flags |= SegmentFlags::R;
                    }
                    if program_header.flags().is_write() {
                        segment_flags |= SegmentFlags::W;
                    }
                    if program_header.flags().is_execute() {
                        segment_flags |= SegmentFlags::X;
                    }
                    let segment = Segment::new(start, stop, MapType::Framed, segment_flags, VMAFlags::empty(), None, 0);
                    let ph_end = program_header.offset() + program_header.file_size();
                    layout.add_segment_with_source(
                        segment, 
                        &elf.input[
                        program_header.offset() as usize
                        ..
                        ph_end as usize
                        ]);
                    verbose!("App segment mapped: {:0x}<->{:0x} ==> {:?}<->{:?}, with flags={:?}", program_header.offset() as usize, ph_end as usize, start, stop, segment_flags);
                    
                    if data_top < stop.0 {
                        data_top = stop.0
                    }
                }
            }
            verbose!("Data Segment top should be at {:x}", data_top);
            // map trapcontext
            layout.add_segment(
                Arc::new(Mutex::new(
                    Segment::new(
                        TRAP_CONTEXT.into(),
                        TRAMPOLINE.into(),
                        MapType::Framed,
                        SegmentFlags::R | SegmentFlags::W,
                        VMAFlags::empty(),
                        None,
                        0
                    )
                ))
            );
            verbose!("Trapcontext mapped.");
            // map guard page
            let guard_page_high_end = VirtAddr::from(TRAP_CONTEXT);
            let guard_page_low_end = guard_page_high_end - PAGE_SIZE;
            layout.add_segment(
                Arc::new(Mutex::new(
                    Segment::new(
                        guard_page_low_end,
                        guard_page_high_end, 
                        MapType::Framed, 
                        SegmentFlags::R |SegmentFlags::W,
                        VMAFlags::empty(),
                        None,
                        0
                    )
                ))
            );
            verbose!("GuardPage mapped.");
            // map user stacks
            let stack_high_end = guard_page_low_end;
            let stack_low_end = stack_high_end - USER_STACK_SIZE;
            layout.add_segment(
                Arc::new(Mutex::new(
                    Segment::new(
                        stack_low_end, 
                        stack_high_end,
                        MapType::Framed, 
                        SegmentFlags::R |SegmentFlags::W |SegmentFlags::U,
                        VMAFlags::empty(),
                        None,
                        0
                    )
                ))
            );
            verbose!("UserStack mapped.");
            debug!(".text address: {}", elf.find_section_by_name(".text").unwrap().address());
            debug!("ph_entry_size: {}", elf.header.pt2.ph_entry_size());
            debug!("ph_count:      {}", elf.header.pt2.ph_count());
            // let ph_head_addr = (elf.find_section_by_name(".text").unwrap().address() as usize ) - (elf.header.pt2.ph_entry_size() as usize) * (elf.header.pt2.ph_count() as usize);
            // let ph_head_addr = 0;
            let mut auxv: Vec<AuxHeader> = Vec::new();
            auxv.push(AuxHeader{aux_type: AuxType::SYSINFO_EHDR,value: 0}); // no vdso here...
            auxv.push(AuxHeader{aux_type: AuxType::NULL28,      value: 0});
            auxv.push(AuxHeader{aux_type: AuxType::NULL29,      value: 0});
            auxv.push(AuxHeader{aux_type: AuxType::NULL2a,      value: 0});
            auxv.push(AuxHeader{aux_type: AuxType::NULL2b,      value: 0});
            auxv.push(AuxHeader{aux_type: AuxType::NULL2c,      value: 0});
            auxv.push(AuxHeader{aux_type: AuxType::NULL2d,      value: 0});
            auxv.push(AuxHeader{aux_type: AuxType::HWCAP,       value: 0 as usize});
            auxv.push(AuxHeader{aux_type: AuxType::PAGESZ,      value: PAGE_SIZE as usize});
            auxv.push(AuxHeader{aux_type: AuxType::CLKTCK,      value: 100 as usize});
            //HACK: is this correct?
            auxv.push(AuxHeader{aux_type: AuxType::PHDR,        value: elf.program_header(0).unwrap().virtual_addr() as usize + elf.header.pt2.ph_offset() as usize});
            auxv.push(AuxHeader{aux_type: AuxType::PHENT,       value: elf.header.pt2.ph_entry_size() as usize}); // ELF64 header 64bytes
            auxv.push(AuxHeader{aux_type: AuxType::PHNUM,       value: elf.header.pt2.ph_count() as usize});
            auxv.push(AuxHeader{aux_type: AuxType::BASE,        value: 0 as usize});
            auxv.push(AuxHeader{aux_type: AuxType::FLAGS,       value: 0 as usize});
            auxv.push(AuxHeader{aux_type: AuxType::ENTRY,       value: elf.header.pt2.entry_point() as usize});
            auxv.push(AuxHeader{aux_type: AuxType::UID,         value: 0 as usize});
            auxv.push(AuxHeader{aux_type: AuxType::EUID,        value: 0 as usize});
            auxv.push(AuxHeader{aux_type: AuxType::GID,         value: 0 as usize});
            auxv.push(AuxHeader{aux_type: AuxType::EGID,        value: 0 as usize});
            auxv.push(AuxHeader{aux_type: AuxType::SECURE,      value: 0 as usize});
    
            return (layout, data_top as usize, stack_high_end.0, elf.header.pt2.entry_point() as usize, auxv);
        }
        panic!("Invlid elf format.");
    }

    /// Map the trampoline code in the Memory layout
    /// # Description
    /// Map the trampoline code in the Memory layout. Trampoline should be in every memory layouts.
    fn map_trampoline(&mut self) {
        extern "C"
        {
            fn strampoline();
        }
        self.pagetable.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X
        );
        verbose!("Trampoline mapped {:?} <=> {:?}, R-X-", VirtAddr::from(TRAMPOLINE), PhysAddr::from(strampoline as usize));
    }

    /// Translate a vpn to a pte
    /// # Description
    /// Translate the virtual page number to page table entry, or None if not mapped.
    /// # Return
    /// The page table entry of the vpn
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        return self.pagetable.translate(vpn);
    }

    /// Get the SATP value of the pagetable in the layout.
    pub fn get_satp(&self) -> usize {
        return self.pagetable.get_satp();
    }

    /// Drop a segment in the memory layout
    /// # Description
    /// Drop a segment in the memory layout identified by the first virtual page number, and free all the corresponding physical pages.
    pub fn drop_segment(&mut self, start: VirtPageNum) {
        for (idx, m_segment) in self.segments.iter().enumerate() {
            let mut segment = m_segment.lock();
            if segment.range.get_start() == start {
                segment.unmap_pages(&mut self.pagetable);
                drop(segment);
                self.segments.remove(idx);
                return
            }
        }
    }

    /// Drop all segments in the layout.
    pub fn drop_all(&mut self) {
        self.segments.clear();
    }

    /// Tranlate a chunk of user memory into kernel space
    /// # Description
    /// Tranlate a user buffer into kernel space. Note that due to paging, the result is not continuous.
    pub fn get_user_data(&self, mut start: VirtAddr, len: usize) -> Vec<&'static mut [u8]> {
        let end = start + len;
        let mut pages = Vec::new();
        while start < end {
            let mut vpn = start.to_vpn();
            let ppn = self.translate(vpn).unwrap().ppn();
            vpn.step();
            let copy_end = min(VirtAddr::from(vpn), end);    // page end or buf end
            pages.push(&mut ppn.page_ptr()[
                start.page_offset()
                ..
                if copy_end.page_offset() == 0 { PAGE_SIZE } else { copy_end.page_offset() }
            ]);
            start = copy_end;
        }
    
        return pages;
    }

    /// Get a c-style string from the user space.
    /// # Description
    /// Get a c-style string from the user space, that is, read until a `b'\0'` is encountered.  
    /// Note that this function returns a clone of the original string.
    /// # Return
    /// A clone of the original c-style string in the user space, in a vector of bytes.
    pub fn get_user_cstr(&self, start: VirtAddr) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        let mut vpn = start.to_vpn();
        let mut iter: usize = start.page_offset();
        'outer: loop {
            let ppn = self.translate(vpn).unwrap().ppn();
            while iter < PAGE_SIZE {
                bytes.push(ppn.page_ptr()[iter]);
                if ppn.page_ptr()[iter] == 0 {
                    break 'outer;
                }
                iter += 1;
            }
            vpn.step();
            iter = 0;
        }
        // bytes.push(0);
        return bytes;
    }

    /// Get a UserBuffer in user space
    /// # Description
    /// Get a UserBuffer in user space. Modify to UserBuffer will modify the corresponding user space memory.
    /// # Return
    /// The userbuffer of corresponding area
    pub fn get_user_buffer(&self, start: VirtAddr, len: usize) -> UserBuffer {
        return UserBuffer::new(self.get_user_data(start, len));
    }

    /// Write a object into user space.
    /// # Description
    /// Write a object into user space. Can cross page boundry
    /// # Example
    /// ```
    /// let to_write: usize = 123456;
    /// current_process.unwrap().get_inner_locked().layout.write_user_data(0x10000.into(), to_write);
    /// ```
    pub fn write_user_data<T>(&self, start: VirtAddr, obj: &T) {
        let mut buf = UserBuffer::new(self.get_user_data(start, size_of::<T>()));
        buf.write(0, obj);
    }

    /// Get an object from the user space.
    /// # Description
    /// Get an object from the user space. Note that this function returns a clone of the original object,  
    /// meaning that modifying that object will not change the user memory.
    /// # Return
    /// A clone of the original object in the user space
    pub fn read_user_data<T: Copy>(&self, start: VirtAddr) -> T {
        let buf =UserBuffer::new(self.get_user_data(start, size_of::<T>()));
        buf.read(0)
    }

    /// Add a VMA segment to the layout
    pub fn add_vma(&mut self, file: Arc<dyn File>, start: VirtAddr, flag: VMAFlags, offset: usize, length: usize) -> Result<VirtAddr, &'static str> {
        if start.0 == 0 {
            return self.add_vma_anywhere(file.clone(), flag, offset, length);
        }
        verbose!("Mapping VMA: [{:?}, {:?}), length = {}", VirtPageNum::from(start), (start + length).to_vpn_ceil(), length);
        let inner = file.clone().to_common_file().unwrap();
        let start_vpn = start.to_vpn();
        let stop_vpn = (min(start + inner.poll().size as usize, start + length)).to_vpn_ceil();
        // check overlap
        for m_seg in self.segments.iter() {
            let seg = m_seg.lock();
            if seg.range.get_start() <= start_vpn && start_vpn < seg.range.get_end() {
                error!("Overlapped mmap segment");
                return Err("Overlapped mmap segment");
            } else if seg.range.get_start() < stop_vpn && stop_vpn < seg.range.get_end() {
                error!("Overlapped mmap segment");
                return Err("Overlapped mmap segment");
            }
        }
        let segment = Segment::new(
            start_vpn.into(), 
            stop_vpn.into(), 
            MapType::VMA, 
            SegmentFlags::empty(), 
            flag, 
            Some(file.clone()),
            offset
        );
        self.add_segment(Arc::new(Mutex::new(segment)));
        Ok(start)
    }

    // TODO: This can be optimized.
    pub fn get_continuous_space(&self, len: usize) -> Option<VirtPageNum> {
        'outer: for i in 0..0xffff_ffff_ff00_0___ {
            let stop_vpn: VirtPageNum = VirtPageNum::from(0xffff_ffff_ff00_0___) - i;
            let start_vpn: VirtPageNum = stop_vpn - len / PAGE_SIZE;
            
            // check overlap
            for m_seg in self.segments.iter() {
                let seg = m_seg.lock();
                if seg.range.get_start() - 1 <= start_vpn && start_vpn < seg.range.get_end() + 1 {
                    continue 'outer;
                } else if seg.range.get_start() - 1 < stop_vpn && stop_vpn < seg.range.get_end() + 1 {
                    continue 'outer;
                }
            }
            return Some(start_vpn); 
        }
        None
    }

    /// Add a VMA segment anywhere
    pub fn add_vma_anywhere(&mut self, file: Arc<dyn File>, flag: VMAFlags, offset: usize, len: usize) -> Result<VirtAddr, &'static str> {
        let start_addr: VirtAddr = self.get_continuous_space(file.poll().size as usize).ok_or("No space left")?.into();
        self.add_vma(file, start_addr, flag, offset, len)
    }

    pub fn lazy_copy_vma(&mut self, address: VirtAddr, access_flag: VMAFlags) -> Result<(), &'static str> {
        for m_seg in self.segments.iter_mut() {
            let mut seg = m_seg.lock();
            if seg.map_type == MapType::VMA && seg.range.get_start() <= address.to_vpn() && address.to_vpn() < seg.range.get_end() {
                if !(access_flag & seg.vma_flags).is_empty() {
                    verbose!("lazy copy triggered for {:?}", address);
                    return seg.map_lazy_vma(&mut self.pagetable, address);
                }
            }
        }
        Err("No such vma segment!")
    }

    pub fn drop_vma(&mut self, drop_start: VirtPageNum, drop_end: VirtPageNum) -> Result<(), &'static str> {
        verbose!("munmapping [{:?}, {:?})", drop_start, drop_end);
        let mut hit_idx = 0;
        let mut found = false;
        for (idx, m_seg) in self.segments.iter().enumerate() {
            let seg = m_seg.lock();
            let seg_start = seg.range.get_start();
            let seg_end = seg.range.get_end();
            let start_ok = seg_start <= drop_start && drop_start < seg_end;
            let end_ok = seg_start < drop_end && drop_start <= seg_end;            
            if start_ok && end_ok {
                hit_idx = idx;
                found = true;
                break;
            } else if start_ok || end_ok {
                return Err("Bad Drop VMA Region.");
            }
        }
        if !found {
            return Err("No such VMA Region");
        }
        let seg = self.segments[hit_idx].lock();
        let file = seg.file.clone().unwrap();
        let offset = seg.offset;
        let seg_start = seg.range.get_start();
        let seg_end = seg.range.get_end();
        let flags = seg.vma_flags;
        drop(seg);  // release lock
        self.drop_segment(seg_start);
        
        // map leading part
        if seg_start < drop_start {
            if let Err(msg) = self.add_vma(file.clone(), seg_start.into(), flags, offset, (drop_start - seg_start) * PAGE_SIZE) {
                return Err(msg);
            }
        }

        // map trailing part
        if drop_end < seg_end {
            if let Err(msg) = self.add_vma(file.clone(), drop_end.into(), flags, offset + (drop_end - seg_start) * PAGE_SIZE, (seg_end - drop_end) * PAGE_SIZE) {
                return Err(msg);
            }
        }
        Ok(())
    }
}

/// A kernel memory map test
pub fn remap_test() {
    extern "C" {
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
    }

    verbose!("Testing kernel memory layout...");
    let kernel_space = KERNEL_MEM_LAYOUT.lock();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert_eq!(
        kernel_space.pagetable.translate(mid_text.to_vpn()).unwrap().writable(),
        false
    );
    assert_eq!(
        kernel_space.pagetable.translate(mid_rodata.to_vpn()).unwrap().writable(),
        false,
    );
    assert_eq!(
        kernel_space.pagetable.translate(mid_data.to_vpn()).unwrap().executable(),
        false,
    );
    debug!("remap_test passed!");
}
