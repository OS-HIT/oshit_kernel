/// *******************
/// * Physical Memory *
/// *******************
/// ===== 0x80000000 =====
/// SBI
/// ===== 0x80020000 =====  <- kernel entry point, 0x80020000 for qemu + opensbi
/// .text
/// =====  srodata   =====
/// .rodata
/// =====    data    =====
/// .data
/// =====   edata    =====
/// .bss                    <- kernel crt  
/// =====   ekernel  =====  <- check symbol in linker_*.id
/// All managed by frame allocator
/// ===== 0x80800000 =====  <- max memory, limited by k210. // TODO make it dynamic
/// 
/// ****************************************************
/// * Kernel Virtual Memory (SV39, PPN in [0, 7fffff]) *
/// ****************************************************
/// ===== 0x7F FFFF ===== 
/// 4 KiB trampoline, liner maped, rx
/// ===== 0x7F FFFE =====   <- Frame alloccator manage end
/// KSTACK_SIZE, kernel stack 0, read & write
/// =====================
/// 1, guard page
/// =====================
/// KSTACK_SIZE, kernel stack 1, read & write
/// =====================
/// 1, guard page
/// =====================
/// ...
/// =====  mem_end  =====   <- Frame alloccator manage start
/// kernel usable physical frames, managed by heap allocator, i.e. kernel heap space, identical mapping
/// =====  ekernel  =====   
/// .bss    rw, identical mapping
/// =====   edata   =====
/// .data   rw, identical mapping
/// =====  erodata  =====
/// .rodata r, identical mapping
/// =====   etext   =====
/// .text   rx, identical mapping
/// ===== base_addr =====
/// UNUSED  // TODO optimize kerenl memory layout, there are 2(or 24) pages unused
/// ===== 0x00 0000 ======
/// 
/// **************************************************
/// * User Virtual Memory (SV39, PPN in [0, 7fffff]) *
/// **************************************************
/// ===== 0x7F FFFF ===== 
/// 4 KiB trampoline, liner maped, rx
/// ===== 0x7F FFFE =====   <- Frame alloccator manage end
/// Trap context, rw    // TODO really? 4KiB of trap context?
/// ===== 0x7F FFFD =====
/// AVAILABLE
/// =====================
/// user stack, urw
/// =====================
/// .bss
/// =====================
/// .data
/// =====================
/// .rodata
/// =====================
/// .text
/// ===== 0x00 0000 =====   <- Frame allocator manage start



mod addresses;

pub use addresses::{
    VirtAddr,
    PhysAddr,
    VirtPageNum,
    PhysPageNum,
};
