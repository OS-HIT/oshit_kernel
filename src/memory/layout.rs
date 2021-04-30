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
};
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

lazy_static! {
    pub static ref KERNEL_MEM_LAYOUT: Arc<Mutex<MemLayout>> = Arc::new(Mutex::new(MemLayout::new_kernel()));
}

#[derive(PartialEq, Debug)]
pub enum MapType {
    Identity,   // identity mapping
    Framed,     // new frame
    // vma,     // TODO: VMA here when working on mmap() syscall?
}

bitflags! {
    pub struct SegmentFlags: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

pub struct Segment {
    range   : VPNRange,
    frames  : BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    flags   : SegmentFlags,
}

impl Segment {
    pub fn new(start: VirtAddr, stop: VirtAddr, map_type: MapType, flags: SegmentFlags) -> Self {
        Self {
            range   : VPNRange::new(start.to_vpn(), stop.to_vpn_ceil()),
            frames  : BTreeMap::new(),
            map_type,
            flags
        }
    }

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

    #[allow(dead_code)]
    pub fn unmap_page(&mut self, pagetable: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.frames.remove(&vpn);
        }
        pagetable.unmap(vpn);
    }

    pub fn map_pages(&mut self, pagetable: &mut PageTable) {
        for vpn in self.range {
            self.map_page(pagetable, vpn);
        }
    }

    #[allow(dead_code)]
    pub fn unmap_pages(&mut self, pagetable: &mut PageTable) {
        for vpn in self.range {
            self.unmap_page(pagetable, vpn);
        }
    }

    pub fn write(&mut self, pagetable: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed, "Error: cannot write to identity mapped segment.");
        assert!(data.len() <= (self.range.get_end() - self.range.get_start()) * PAGE_SIZE, "Data too long to be written into segment.");
        let mut data_i: usize = 0;
        let mut vpn_i = self.range.get_start();
        let len = data.len();
        while data_i < len {
            let src = &data[data_i..min(data_i + PAGE_SIZE, len)];
            let dst: &mut [u8];
            if let Some(ppn) = pagetable.translate(vpn_i) {
                dst = &mut ppn.ppn().page_ptr()[..src.len()];
            } else {
                panic!("{:?} hasn't been mapped.", vpn_i);
            }
            dst.copy_from_slice(src);
            vpn_i.step();
            data_i += PAGE_SIZE;
        }
    }

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

pub struct MemLayout {
    pagetable   : PageTable,
    segments    : Vec<Segment>,
}

impl MemLayout {
    pub fn new() -> Self {
        Self {
            pagetable   : PageTable::new(),
            segments    : Vec::new(),
        }
    }

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
    
    pub fn activate(&self) {
        verbose!("Kernel switching to virtual memory space...");
        let satp = self.pagetable.get_satp();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
        if satp::read().mode() != satp::Mode::Sv39 {
            error!("Failed switch to SV39!");
            warning!("This seems to be a known issue with k210 + rustsbi.");
        } else {
            info!("Kernel virtual memory layout has been activated.");
        }
    }

    pub fn add_segment(&mut self, mut segment: Segment) {
        segment.map_pages(&mut self.pagetable);
        self.segments.push(segment);
    }

    pub fn add_segment_with_source(&mut self, mut segment: Segment, data: &[u8]) {
        segment.map_pages(&mut self.pagetable);
        segment.write(&mut self.pagetable, data);
        self.segments.push(segment);
    }

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
        info!("Kernel memory layout initilized.");

        return layout;
    }

    pub fn new_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut layout = Self::new();
        layout.map_trampoline();
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        assert_eq!(elf.header.pt1.magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let mut end_vpn = VirtPageNum::from(0);
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
                end_vpn = segment.range.get_end();
                layout.add_segment_with_source(
                    segment, 
                    &elf.input[
                        program_header.offset() as usize
                        ..
                        (program_header.offset() + program_header.file_size()) as usize
                    ]);
            }
        }
        // map user stacks
        let stack_bottom = VirtAddr::from(end_vpn) + PAGE_SIZE;
        layout.add_segment(
            Segment::new(
                stack_bottom, 
                stack_bottom + USER_STACK_SIZE, 
                MapType::Framed, 
                SegmentFlags::R |SegmentFlags::W |SegmentFlags::U
            )
        );
        // map trapcontext
        layout.add_segment(
            Segment::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                SegmentFlags::R | SegmentFlags::W,
            )
        );

        return (layout, stack_bottom.0 + USER_STACK_SIZE, elf.header.pt2.entry_point() as usize);
    }

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
        debug!("Trampoline mapped {:?} <=> {:?}, R-X-", VirtAddr::from(TRAMPOLINE), PhysAddr::from(strampoline as usize))
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        return self.pagetable.translate(vpn);
    }

    pub fn get_satp(&self) -> usize {
        return self.pagetable.get_satp();
    }

    pub fn drop_segment(&mut self, start: VirtPageNum) {
        if let Some((idx, segment)) = self.segments
        .iter_mut().enumerate()
        .find(|(_, seg)| seg.range.get_start() == start) {
            segment.unmap_pages(&mut self.pagetable);
            self.segments.remove(idx);
        }
    }
}

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