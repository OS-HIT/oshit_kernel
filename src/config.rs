//! Config file for the kernel. 
//! Changing some value might have unexpected consequences, proceed with causion!

/// Default kernel stack size for each process
pub const KERNEL_STACK_SIZE : usize = 4096 * 32;

/// Default user stack size. Will be override by `sys_clone()` arguments
pub const USER_STACK_SIZE   : usize = 4096 * 32;

/// Kernel heap size, used in dynamic memory allocation (like vec and String)
pub const KERNEL_HEAP_SIZE  : usize = 0x100000;

/// Bits reperensenting page offset
pub const PAGE_OFFSET       : usize = 12;

/// 4KiB per page
pub const PAGE_SIZE         : usize = 1 << PAGE_OFFSET;

/// This is where the physical memory ends.
/// ref: [k210-sdk-stuff/memory_map.md](https://github.com/laanwj/k210-sdk-stuff/blob/master/doc/memory_map.md)
// pub const MEM_END           : usize = 0x80800000;  
pub const MEM_END           : usize = 0x90000000;  

/// Position of Trampoline, which is a piece of code use for context switching when we switch priviledge levels (`ecall`/`sret`)
#[no_mangle]
#[link_section = ".trampoline"]
pub static TRAMPOLINE        : usize = usize::MAX - PAGE_SIZE + 1;

/// Position of TrapContext, which is just below the trampoline and takes up an entire page.
pub static TRAP_CONTEXT      : usize = TRAMPOLINE - PAGE_SIZE;

/// Max pipe ring buffer size. Same as linux.
pub const PIP_BUF_MAX       : usize = 65536;

/// Clock freqency on k210
#[cfg(feature = "board_k210")]
pub const CLOCK_FREQ: u64 = 403000000 / 62;

/// Clock frequency on qemu
#[cfg(feature = "board_qemu")]
pub const CLOCK_FREQ: u64 = 12500000;

/// UName constants, name of our OS
pub const SYSNAME       : &[u8] = b"OSHIT Kernel (Pre-Alpha)\0";
/// UName constants
pub const NODENAME      : &[u8] = b"Network currently unsupported\0";
/// UName constants, OS version
pub const RELEASE       : &[u8] = b"0.0.2-alpha\0";
/// UName constants
pub const MACHINE       : &[u8] = b"UNKNOWN MACHINE\0";
/// UName constants
pub const DOMAINNAME    : &[u8] = b"UNKNOWN DOMAIN NAME\0";
/// Length of each field in `struct uname`
pub const UTSNAME_LEN   : usize = 65;

/// Device memory mapped IO for K210
#[cfg(feature = "board_k210")]
pub const MMIO: &[(usize, usize)] = &[
    (0x0C00_0000, 0x3000),      /* PLIC      */
    (0x0C20_0000, 0x1000),      /* PLIC      */
    (0x3800_0000, 0x1000),      /* UARTHS    */
    (0x3800_1000, 0x1000),      /* GPIOHS    */
    (0x5020_0000, 0x1000),      /* GPIO      */
    (0x5024_0000, 0x1000),      /* SPI_SLAVE */
    (0x502B_0000, 0x1000),      /* FPIOA     */
    (0x502D_0000, 0x1000),      /* TIMER0    */
    (0x502E_0000, 0x1000),      /* TIMER1    */
    (0x502F_0000, 0x1000),      /* TIMER2    */
    (0x5044_0000, 0x1000),      /* SYSCTL    */
    (0x5200_0000, 0x1000),      /* SPI0      */
    (0x5300_0000, 0x1000),      /* SPI1      */
    (0x5400_0000, 0x1000),      /* SPI2      */
];

/// Device memory mapped IO for qemu
#[cfg(feature = "board_qemu")]
pub const MMIO: &[(usize, usize)] = &[
    (0x10000000, 0x10000),
];

/// An ASCII art logo
pub const LOGO: &str = r#"
 ██████╗ ███████╗      ██╗  ██╗██╗████████╗
██╔═══██╗██╔════╝      ██║  ██║██║╚══██╔══╝
██║   ██║███████╗█████╗███████║██║   ██║   
██║   ██║╚════██║╚════╝██╔══██║██║   ██║   
╚██████╔╝███████║      ██║  ██║██║   ██║   
 ╚═════╝ ╚══════╝      ╚═╝  ╚═╝╚═╝   ╚═╝  
"#;

pub const PLATFROM: &[u8; 8] = b"RISC-V64";