use buddy_system_allocator::LockedHeap;
use crate::config::KERNEL_HEAP_SIZE;
use alloc::boxed::Box;
use alloc::vec::Vec;

#[global_allocator]
static KERNEL_HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

fn heap_test() {
    verbose!("Testing kernel heap...");
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let bss_range = sbss as usize..ebss as usize;
    let a = Box::new(5);
    assert_eq!(*a, 5);
    assert!(bss_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for i in 0..500 {
        assert_eq!(v[i], i);
    }
    assert!(bss_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    verbose!("Kernel heap test passed!");
}

pub fn init_kernel_heap() {
    debug!("Initializing kernel heap space...");
    verbose!("Kernel heap start @ 0x{:0X}, length 0x{:0X}", unsafe{HEAP_SPACE.as_ptr()} as usize, KERNEL_HEAP_SIZE);
    unsafe {
        KERNEL_HEAP_ALLOCATOR.lock().init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
    heap_test();
    info!("Kernel heap initialized.");
}

#[alloc_error_handler]
pub fn on_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Kernel heap allocation error on allocating layout {:?}. OOM?", layout);
}