//! QEMU virtio driver wrapper
use virtio_drivers::{VirtIOBlk, VirtIOHeader};
use crate::memory::{FrameTracker, PageTable, PhysAddr, PhysPageNum, VirtAddr, alloc_continuous, alloc_frame, free_frame, kernel_satp};
use crate::sbi::get_time_ms;
use crate::utils::StepByOne;
use super::BlockDevice;
use spin::Mutex;
use alloc::vec::Vec;
use lazy_static::*;

/// MMIO address for QEMU device
#[allow(unused)]
const VIRTIO0: usize = 0x10001000;

pub struct VirtIOBlock(Mutex<VirtIOBlk<'static>>);

lazy_static! {
    static ref QUEUE_FRAMES: Mutex<Vec<FrameTracker>> = Mutex::new(Vec::new());
}

const ZEROS: [u8;512] = [0u8; 512];
impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0.lock().read_block(block_id, buf).expect("Error when reading VirtIOBlk");
        
        unsafe { asm!("fence.i"); }
        for i in 0..512 {
            let b = buf[i];
            unsafe { 
                asm!(
                    "add x0, x0, {0}",
                    in(reg) b
                );
            }
        }
        unsafe { asm!("fence.i"); }
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0.lock().write_block(block_id, buf).expect("Error when writing VirtIOBlk");
    }
    fn clear_block(&self, block_id: usize) {
        self.0.lock().write_block(block_id, &ZEROS).unwrap();
    }
    fn block_cnt(&self) -> u64 {
        0
    }
}

impl VirtIOBlock {
    #[allow(unused)]
    pub fn new() -> Self {
        Self(Mutex::new(VirtIOBlk::new(
            unsafe { &mut *(VIRTIO0 as *mut VirtIOHeader) }
        ).unwrap()))
    }
}

#[no_mangle]
pub extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
    // let mut ppn_base = PhysPageNum(0);
    // for i in 0..pages {
    //     let frame = alloc_frame().unwrap();
    //     if i == 0 { ppn_base = frame.ppn; }
    //     assert_eq!(frame.ppn.0, ppn_base.0 + i);
    //     QUEUE_FRAMES.lock().push(frame);
    // }
    // ppn_base.into()
    let mut allocated = alloc_continuous(pages);
    let init_ppn = allocated[0].ppn;
    QUEUE_FRAMES.lock().append(&mut allocated);
    init_ppn.into()
}

#[no_mangle]
pub extern "C" fn virtio_dma_dealloc(pa: PhysAddr, pages: usize) -> i32 {
    // not dropping queue??? mulit drop???
    let mut ppn_base: PhysPageNum = pa.into();
    for _ in 0..pages {
        free_frame(ppn_base);
        ppn_base.step();
    }
    0
}

#[no_mangle]
pub extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    VirtAddr(paddr.0)
}

#[no_mangle]
pub extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
    PageTable::from_satp(kernel_satp()).translate_va(vaddr).unwrap()
}