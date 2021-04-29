pub const KERNEL_STACK_SIZE : usize = 4096 * 2;
pub const USER_STACK_SIZE   : usize = 4096 * 2;
pub const KERNEL_HEAP_SIZE  : usize = 0x20000;
pub const MAX_APP_NUM       : usize = 4;
pub const APP_BASE_ADDRESS  : usize = 0x80400000;
pub const APP_SIZE_LIMIT    : usize = 0x20000;
pub const PAGE_OFFSET       : usize = 12;
pub const PAGE_SIZE         : usize = 1 << PAGE_OFFSET;
pub const MEM_END           : usize = 0x80800000;   // ref: https://github.com/laanwj/k210-sdk-stuff/blob/master/doc/memory_map.md
pub const TRAMPOLINE        : usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT      : usize = TRAMPOLINE - PAGE_SIZE;

#[cfg(feature = "board_k210")]
pub const CLOCK_FREQ: usize = 403000000 / 62;

#[cfg(feature = "board_qemu")]
pub const CLOCK_FREQ: usize = 12500000;

pub const SYSNAME       : &[u8] = b"OSHIT Kernel (Pre-Alpha)\0";
pub const NODENAME      : &[u8] = b"Network currently unsupported\0";
pub const RELEASE       : &[u8] = b"0.0.1-alpha\0";   // Semantic Versioning
// NOTE: following line will be found and modified by build.rs.
// DONT CHANGE THIS LINE MANUALLY!!!!
pub const VERSION : &[u8] = b"Thu, 29 Apr 2021 06:01:19 +0000\0";
pub const MACHINE       : &[u8] = b"UNKNOWN MACHINE\0";
pub const DOMAINNAME    : &[u8] = b"UNKNOWN DOMAIN NAME\0";













































































































