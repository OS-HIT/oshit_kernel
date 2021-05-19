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
pub const PIP_BUF_MAX       : usize = 65536;    // same as linux

#[cfg(feature = "board_k210")]
pub const CLOCK_FREQ: u64 = 403000000 / 62;

#[cfg(feature = "board_qemu")]
pub const CLOCK_FREQ: u64 = 12500000;

pub const SYSNAME       : &[u8] = b"OSHIT Kernel (Pre-Alpha)\0";
pub const NODENAME      : &[u8] = b"Network currently unsupported\0";
pub const RELEASE       : &[u8] = b"0.0.1-alpha\0";   // Semantic Versioning
pub const MACHINE       : &[u8] = b"UNKNOWN MACHINE\0";
pub const DOMAINNAME    : &[u8] = b"UNKNOWN DOMAIN NAME\0";

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

#[cfg(feature = "board_qemu")]
pub const MMIO: &[(usize, usize)] = &[
    (0x10000000, 0x10000),
];

pub const logo: &str = r#"
 ██████╗ ███████╗      ██╗  ██╗██╗████████╗
██╔═══██╗██╔════╝      ██║  ██║██║╚══██╔══╝
██║   ██║███████╗█████╗███████║██║   ██║   
██║   ██║╚════██║╚════╝██╔══██║██║   ██║   
╚██████╔╝███████║      ██║  ██║██║   ██║   
 ╚═════╝ ╚══════╝      ╚═╝  ╚═╝╚═╝   ╚═╝  
"#;