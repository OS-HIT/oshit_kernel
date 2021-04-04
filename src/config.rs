pub const USER_STACK_SIZE   : usize = 4096 * 2;
pub const KERNEL_STACK_SIZE : usize = 4096 * 2;
pub const KERNEL_HEAP_SIZE  : usize = 0x200000; // 2MB
pub const MAX_APP_NUM       : usize = 4;
pub const APP_BASE_ADDRESS  : usize = 0x80400000;
pub const APP_SIZE_LIMIT    : usize = 0x20000;
pub const PAGE_OFFSET       : usize = 12;
pub const PAGE_SIZE         : usize = 1 << PAGE_OFFSET;
pub const MEM_END           : usize = 0x80800000;   // ref: https://github.com/laanwj/k210-sdk-stuff/blob/master/doc/memory_map.md


#[cfg(feature = "board_k210")]
pub const CLOCK_FREQ: usize = 403000000 / 62;

#[cfg(feature = "board_qemu")]
pub const CLOCK_FREQ: usize = 12500000;