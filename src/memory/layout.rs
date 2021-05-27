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
    get_user_buffer,
    UserBuffer
};
use core::mem::size_of;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use bitflags::*;
use crate::config::*;
use core::cmp::min;
use crate::utils::StepByOne;
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
    // vma,     // TODO: VMA here when working on mmap() syscall?
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

/// Representing a continuous segment in the memroy layout.  
/// For example, data segments/text segments in the user program.
pub struct Segment {
    /// range of the Segment, [range.start()..range.end())
    range   : VPNRange,
    /// allocated physical frames, aloneside with their virtual page number.  
    /// It holds the FrameTracker so that it's not dropped.
    frames  : BTreeMap<VirtPageNum, FrameTracker>,
    /// the mapping type (identity or framed)
    map_type: MapType,
    /// the flags
    flags   : SegmentFlags,
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
    pub fn new(start: VirtAddr, stop: VirtAddr, map_type: MapType, flags: SegmentFlags) -> Self {
        verbose!("New Segment: {:?} <=> {:?}", start.to_vpn(), stop.to_vpn_ceil());
        Self {
            range   : VPNRange::new(start.to_vpn(), stop.to_vpn_ceil()),
            frames  : BTreeMap::new(),
            map_type,
            flags
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
    pub fn map_page(&mut self, pagetable: &mut PageTable, vpn: VirtPageNum) {
        if vpn < self.range.get_start() || vpn >= self.range.get_end() {
            error!("Trying to map a page that is not in this Segment.");
            return;
        }
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identity => {
                ppn = PhysPageNum(vpn.0);
            },
            MapType::Framed => {
                let frame = alloc_frame().unwrap();
                ppn = frame.ppn;
                self.frames.insert(vpn, frame);
            }
        }
        pagetable.map(vpn, ppn, PTEFlags::from_bits(self.flags.bits).unwrap());
    }
    
    pub fn adjust_end(&mut self, pagetable: &mut PageTable, new_end: VirtPageNum) {
        // We need to align the end to the 4K border of the page
        // let new_end = self.range.get_end() + (VirtAddr::from(sz)).0;
        // self.range.set_end(new_end);
        if new_end < self.range.get_end() {
            for i in new_end.0..self.range.get_end().0 {
                self.unmap_page(pagetable, i.into());
            }
        } else if new_end > self.range.get_end() {
            for i in self.range.get_end().0..new_end.0 {
                self.map_page(pagetable, i.into());
            }
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
        if self.map_type == MapType::Framed {
            self.frames.remove(&vpn);
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
            self.map_page(pagetable, vpn);
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
            range: VPNRange::new(
                src.range.get_start(),
                src.range.get_end()
            ),
            frames: BTreeMap::new(),
            map_type: src.map_type,
            flags: src.flags,
        }
    }
}

/// The memory layout, for kernel space or user space.
pub struct MemLayout {
    /// The pagetable of this memory layout.
    pagetable   : PageTable,
    /// The segments in this memory layout.
    pub segments    : Vec<Segment>,
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
    pub fn fork_from_user(src: &MemLayout) -> Self {
        let mut layout = Self::new();
        layout.map_trampoline();
        for segment in src.segments.iter() {
            let new_segment = Segment::clone_from(segment);
            layout.add_segment(new_segment);
            for vpn in segment.range {
                let src_ppn = src.translate(vpn).unwrap().ppn();
                let dst_ppn = layout.translate(vpn).unwrap().ppn();
                dst_ppn.page_ptr().copy_from_slice(src_ppn.page_ptr());
            }
        }
        return layout;
    }

    pub fn alter_segment(&mut self, old_end: VirtPageNum, new_end: VirtPageNum) {
        if let Some((idx, segment)) = self.segments.iter_mut().enumerate().find(|(_, seg)| seg.range.get_end() == old_end) {
            segment.adjust_end(&mut self.pagetable, new_end);
        } else {
            error!("No segment end with {:?}", old_end);
        }
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
    pub fn add_segment(&mut self, mut segment: Segment) {
        segment.map_pages(&mut self.pagetable);
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
        self.segments.push(segment);
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
            Segment::new(
                VirtAddr::from(stext as usize), 
                VirtAddr::from(etext as usize),
                MapType::Identity,
                SegmentFlags::R | SegmentFlags::X
            )
        );
        debug!(".text mapped @ 0x{:X} ~ 0x{:X} (identity), R-X-.", stext as usize, etext as usize);
        
        verbose!("Mapping .rodata...");
        layout.add_segment(
            Segment::new(
                VirtAddr::from(srodata as usize), 
                VirtAddr::from(erodata as usize),
                MapType::Identity,
                SegmentFlags::R | SegmentFlags::X
            )
        );
        debug!(".rodata mapped @ 0x{:X} ~ 0x{:X} (identity), R-X-.", srodata as usize, erodata as usize);
        
        verbose!("Mapping .data...");
        layout.add_segment(
            Segment::new(
                VirtAddr::from(sdata as usize), 
                VirtAddr::from(edata as usize),
                MapType::Identity,
                SegmentFlags::R
            )
        );
        debug!(".data mapped @ 0x{:X} ~ 0x{:X} (identity), R---.", sdata as usize, edata as usize);
        
        verbose!("Mapping .bss...");
        layout.add_segment(
            Segment::new(
                VirtAddr::from(sbss_with_stack as usize), 
                VirtAddr::from(ebss as usize),
                MapType::Identity,
                SegmentFlags::R | SegmentFlags::W
            )
        );
        debug!(".bss mapped @ 0x{:X} ~ 0x{:X} (identity), RW--.", sbss_with_stack as usize, sbss_with_stack as usize);
        
        verbose!("Mapping rest physical memory as identical...");
        layout.add_segment(
            Segment::new(
                VirtAddr::from(ekernel as usize), 
                VirtAddr::from(MEM_END),
                MapType::Identity,
                SegmentFlags::R | SegmentFlags::W
            )
        );
        debug!("Physical memory mapped @ 0x{:X} ~ 0x{:X} (identity), RW--.", ekernel as usize, MEM_END);

        verbose!("Mapping MMIO...");
        for pair in MMIO {
            layout.add_segment(
                Segment::new(
                    (*pair).0.into(),
                    ((*pair).0 + (*pair).1).into(),
                    MapType::Identity,
                    SegmentFlags::R | SegmentFlags::W,
                )
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
    pub fn new_elf(elf_data: &[u8]) -> (Self, usize, usize, usize) {
        let mut layout = Self::new();
        layout.map_trampoline();
        let mut end_vpn = VirtPageNum::from(0);
        let mut data_top = 0;
        let mut entry_point: usize = 0;
        if let Ok(elf) = xmas_elf::ElfFile::new(elf_data) {
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
                    let segment = Segment::new(start, stop, MapType::Framed, segment_flags);
                    let ph_end = program_header.offset() + program_header.file_size();
                    end_vpn = segment.range.get_end();
                    layout.add_segment_with_source(
                        segment, 
                        &elf.input[
                        program_header.offset() as usize
                        ..
                        ph_end as usize
                        ]);
                    verbose!("App segment mapped: {:0x} <-> {:0x}", program_header.offset() as usize, ph_end as usize);
                    
                    if data_top < ph_end {
                        data_top = ph_end
                    }
                }
            }
            verbose!("Data Segment top should be at {:x}", data_top);
            entry_point = elf.header.pt2.entry_point() as usize;
            // map trapcontext
            layout.add_segment(
                Segment::new(
                    TRAP_CONTEXT.into(),
                    TRAMPOLINE.into(),
                    MapType::Framed,
                    SegmentFlags::R | SegmentFlags::W,
                )
            );
            // map guard page
            let guard_page_high_end = VirtAddr::from(TRAP_CONTEXT);
            let guard_page_low_end = guard_page_high_end - PAGE_SIZE;
            layout.add_segment(
                Segment::new(
                    guard_page_low_end,
                    guard_page_high_end, 
                    MapType::Framed, 
                    SegmentFlags::R |SegmentFlags::W
                )
            );
            // map user stacks
            let stack_high_end = guard_page_low_end;
            let stack_low_end = stack_high_end - USER_STACK_SIZE;
            layout.add_segment(
                Segment::new(
                    stack_low_end, 
                    stack_high_end,
                    MapType::Framed, 
                    SegmentFlags::R |SegmentFlags::W |SegmentFlags::U
                )
            );

            return (layout, data_top as usize, stack_high_end.0, elf.header.pt2.entry_point() as usize);
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
        verbose!("Trampoline mapped {:?} <=> {:?}, R-X-", VirtAddr::from(TRAMPOLINE), PhysAddr::from(strampoline as usize))
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
        if let Some((idx, segment)) = self.segments
        .iter_mut().enumerate()
        .find(|(_, seg)| seg.range.get_start() == start) {
            segment.unmap_pages(&mut self.pagetable);
            self.segments.remove(idx);
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
        bytes.push(0);
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
