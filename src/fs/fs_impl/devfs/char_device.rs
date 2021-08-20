use super::{CharDeviceFile, DeviceFile};
use super::super::super::File;
use super::super::super::Path;
use alloc::string::ToString;
use alloc::string::String;
use lazy_static::*;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;
use crate::fs::{CommonFile, DirFile};
use crate::fs::file::{FileStatus, FileType};
use crate::memory::VirtAddr;
use crate::process::current_process;
use crate::sbi::{get_byte, get_byte_non_block_with_echo};
use crate::sbi::put_byte;
use core::cell::RefCell;
use core::usize;
use core::convert::{TryFrom, TryInto};
use bitflags::*;
use crate::process::ErrNo;

lazy_static! {
	pub static ref TTY0: Arc<SBITTY> = Arc::new(SBITTY::new());
}

const LF: u8 = b'\n';

pub struct SBITTY {
	buffer_size: usize,
	inner: Mutex<TTYInner>
}

struct TTYInner {
	read_buffer: VecDeque<u8>,
	write_buffer: VecDeque<u8>,
}

impl SBITTY {
	pub fn new() -> Self {
		Self {
			buffer_size: 4096,
			inner: Mutex::new(
				TTYInner {
					read_buffer: VecDeque::new(),
					write_buffer: VecDeque::new(),
				}
			)
		}
	}
}

impl Drop for SBITTY {
    fn drop(&mut self) {
        // do nothing
    }
}

impl File for SBITTY {
    fn seek(&self, offset: isize, op: crate::fs::SeekOp) -> Result<(), ErrNo> {
        Err(ErrNo::PermissionDenied)
    }

	// TODO: implement smarter flush timing, and some how intergrate this.
    fn read(&self, buffer: &mut [u8]) -> Result<usize, ErrNo> {
		for idx in 0..buffer.len() {
            let mut b = get_byte();
            if b == b'\r' {
                b = b'\n';
            }
			buffer[idx] = b;
            put_byte(b);
            // verbose!("{}, {}", b, b as char);
			if buffer[idx] == b'\n' {
                // verbose!("Done!");
				return Ok(idx);
			}
		}
		Ok(buffer.len())
    }

    fn read_user_buffer(&self, mut buffer: crate::memory::UserBuffer) -> Result<usize, ErrNo> {
		for idx in 0..buffer.len() {
            let mut b = get_byte();
            if b == b'\r' {
                b = b'\n';
            }
			buffer[idx] = b;
            put_byte(b);
            // verbose!("{}, {}", b, b as char);
			if buffer[idx] == b'\n' {
                // verbose!("Done!");
				return Ok(idx);
			}
		}
		Ok(buffer.len())
    }

	// TODO: implement smarter flush timing
    fn write(&self, buffer: &[u8]) -> Result<usize, ErrNo> {
        let mut offset = 0;
		while offset < buffer.len() {
			self.flush();
			let mut inner_locked = self.inner.lock();
			while inner_locked.write_buffer.len() < self.buffer_size as usize && offset < buffer.len() {
				inner_locked.write_buffer.push_back(buffer[offset]);
				offset += 1;
			}
		}
		self.flush();
		Ok(offset)
    }

    fn write_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<usize, ErrNo> {
        let mut offset = 0;
		while offset < buffer.len() {
			self.flush();
			let mut inner_locked = self.inner.lock();
			while inner_locked.write_buffer.len() < self.buffer_size as usize && offset < buffer.len() {
				inner_locked.write_buffer.push_back(buffer[offset]);
				offset += 1;
			}
		}
		self.flush();
		Ok(offset)
    }

    fn to_common_file<'a>(self: Arc<Self>) -> Option<Arc<dyn CommonFile + 'a>> where Self: 'a {
        None
    }

    fn to_dir_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DirFile + 'a>> where Self: 'a {
        None
    }

    fn to_device_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DeviceFile + 'a>> where Self: 'a {
        Some(self)
    }

    fn poll(&self) -> crate::fs::file::FileStatus {
        FileStatus {
			readable: 	true,
            writeable: 	true,
            size: 		0,
            name: 		"tty0".to_string(),
            ftype: 		FileType::CharDev,
            inode: 		0,
            dev_no: 	0,
            mode: 		0,	// TODO: check impl
            block_sz: 	0,
            blocks: 	0,
            uid: 		0,
            gid: 		0,
            atime_sec: 	0,
            atime_nsec:	0,
            mtime_sec: 	0,
            mtime_nsec:	0,
            ctime_sec: 	0,
            ctime_nsec:	0,
		}
    }

    fn rename(&self, new_name: &str) -> Result<(), ErrNo> {
        Err(ErrNo::PermissionDenied)
    }

    fn get_vfs(&self) -> Result<Arc<(dyn crate::fs::VirtualFileSystem + 'static)>, ErrNo> {
        Ok(super::DEV_FS.clone())
    }

    fn get_path(&self) -> Path {
        let path = vec![String::from("tty0")];
        return Path {path, must_dir: false, is_abs: true}; 
    }

    fn get_cursor(&self) -> Result<usize, ErrNo> {
        Err(ErrNo::IllegalSeek)
    }
}

macro_rules! EnumWithTryFrom {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl core::convert::TryFrom<u64> for $name {
            type Error = ();

            fn try_from(v: u64) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as u64 => Ok($name::$vname),)*
                    _ => Err(()),
                }
            }
        }
    }
}

EnumWithTryFrom!{
    #[repr(u64)]
    #[derive(Debug)]
    enum IOCTLOperation {
        TCGETS          = 0x5401,
        TCSETS          = 0x5402,
        TCSETSW         = 0x5403,
        TCSETSF         = 0x5404,
        TCGETA          = 0x5405,
        TCSETA          = 0x5406,
        TCSETAW         = 0x5407,
        TCSETAF         = 0x5408,
        TCSBRK          = 0x5409,
        TCXONC          = 0x540A,
        TCFLSH          = 0x540B,
        TIOCEXCL        = 0x540C,
        TIOCNXCL        = 0x540D,
        TIOCSCTTY       = 0x540E,
        TIOCGPGRP       = 0x540F,
        TIOCSPGRP       = 0x5410,
        TIOCOUTQ        = 0x5411,
        TIOCSTI         = 0x5412,
        TIOCGWINSZ      = 0x5413,
        TIOCSWINSZ      = 0x5414,
        TIOCMGET        = 0x5415,
        TIOCMBIS        = 0x5416,
        TIOCMBIC        = 0x5417,
        TIOCMSET        = 0x5418,
        TIOCGSOFTCAR    = 0x5419,
        TIOCSSOFTCAR    = 0x541A,
        TIOCINQ         = 0x541B,
        TIOCLINUX       = 0x541C,
        TIOCCONS        = 0x541D,
        TIOCGSERIAL     = 0x541E,
        TIOCSSERIAL     = 0x541F,
        TIOCPKT         = 0x5420,
        FIONBIO         = 0x5421,
        TIOCNOTTY       = 0x5422,
        TIOCSETD        = 0x5423,
        TIOCGETD        = 0x5424,
        TCSBRKP         = 0x5425,  /* Needed for POSIX tcsendbreak() */
        TIOCSBRK        = 0x5427,  /* BSD compatibility */
        TIOCCBRK        = 0x5428,  /* BSD compatibility */
        TIOCGSID        = 0x5429,  /* Return the session ID of FD */
        TCGETS2         = 0x542A,
        TCSETS2         = 0x542B,
        TCSETSW2        = 0x542C,
        TCSETSF2        = 0x542D,
        TIOCGRS485      = 0x542E,
        TIOCSRS485      = 0x542F,
        TIOCGPTN        = 0x5430,  /* Get Pty Number (of pty-mux device) */
        TIOCSPTLCK      = 0x5431,  /* Lock/unlock Pty */
        TIOCGDEV        = 0x5432,  /* Get primary device node of /dev/console */
        TCSETX          = 0x5433,
        TCSETXF         = 0x5434,
        TCSETXW         = 0x5435,
        TIOCSIG         = 0x5436,  /* pty: generate signal */
        TIOCVHANGUP     = 0x5437,
        TIOCGPKT        = 0x5438,  /* Get packet mode state */
        TIOCGPTLCK      = 0x5439,  /* Get Pty lock state */
        TIOCGEXCL       = 0x5440,  /* Get exclusive mode state */
        TIOCGPTPEER     = 0x5441,  /* Safely open the slave */
        TIOCGISO7816    = 0x5442,
        TIOCSISO7816    = 0x5443,
        FIONCLEX        = 0x5450,
        FIOCLEX         = 0x5451,
        FIOASYNC        = 0x5452,
        TIOCSERCONFIG   = 0x5453,
        TIOCSERGWILD    = 0x5454,
        TIOCSERSWILD    = 0x5455,
        TIOCGLCKTRMIOS  = 0x5456,
        TIOCSLCKTRMIOS  = 0x5457,
        TIOCSERGSTRUCT  = 0x5458,  /* For debugging only */
        TIOCSERGETLSR   = 0x5459,  /* Get line status register */
        TIOCSERGETMULTI = 0x545A,  /* Get multiport config */
        TIOCSERSETMULTI = 0x545B,  /* Set multiport config */
        TIOCMIWAIT      = 0x545C,  /* wait for a change on serial input line(s) */
        TIOCGICOUNT     = 0x545D,  /* read serial port inline interrupt counts */
    }
}


#[derive(Clone, Copy, Debug)]
struct WinSize {
    row: u16,
    col: u16,
    x_pixel: u16,
    y_pixel: u16,
}

bitflags! {
    pub struct TTYIFlag: u32 {
        const  IGNBRK   = 0o0000001;
        const  BRKINT   = 0o0000002;
        const  IGNPAR   = 0o0000004;
        const  PARMRK   = 0o0000010;
        const  INPCK    = 0o0000020;
        const  ISTRIP   = 0o0000040;
        const  INLCR    = 0o0000100;
        const  IGNCR    = 0o0000200;
        const  ICRNL    = 0o0000400;
        const  IUCLC    = 0o0001000;
        const  IXON     = 0o0002000;
        const  IXANY    = 0o0004000;
        const  IXOFF    = 0o0010000;
        const  IMAXBEL  = 0o0020000;
        const  IUTF8    = 0o0040000;
    }
}

impl DeviceFile for SBITTY {
    fn ioctl(&self, op: u64, argp: VirtAddr) -> Result<u64, ErrNo> {
		// TODO: Check tty's ioctl
        let op: IOCTLOperation = IOCTLOperation::try_from(op).map_err(|_| ErrNo::PermissionDenied)?;
        match op {
            IOCTLOperation::TIOCGWINSZ => {
                let size = WinSize {
                    row: 80,
                    col: 25,
                    x_pixel: 800,
                    y_pixel: 600,
                };
                current_process().unwrap().get_inner_locked().layout.write_user_data(argp, &size);
                Ok(0)
            },
            _ => {
                error!("tty caught ioctl for op={:?}, argp={:?}", op, argp);
                Err(ErrNo::NotSuchDevice)
            }
        }
    }

    fn to_char_dev<'a>(self: Arc<Self>) -> Option<Arc<dyn CharDeviceFile + 'a>> where Self: 'a  {
        Some(self)
    }

    fn to_blk_dev<'a>(self: Arc<Self>) -> Option<Arc<dyn super::BlockDeviceFile + 'a>> where Self: 'a  {
        None
    }
}

impl CharDeviceFile for SBITTY {
    fn flush(&self) {
		let mut inner_locked = self.inner.lock();
		while !inner_locked.write_buffer.is_empty() {
			put_byte(inner_locked.write_buffer.pop_front().unwrap());
		}
    }
}