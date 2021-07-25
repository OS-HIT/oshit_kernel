//! File system implementation for oshit. Currently, only FAT32 is supported

pub mod block_cache;
pub mod fat;
mod dirent;
pub mod path;
pub mod file;
mod stdio;
mod file_with_lock;
mod pipe;

pub use file_with_lock::FileWithLock;
pub use pipe::{
        make_pipe,
        Pipe,
        PipeEnd,
        PipeFlags,
};

pub use stdio::{
        LOCKED_STDIN,
        LOCKED_STDOUT,
        LOCKED_STDERR,
        Stdin,
        Stdout,
        Stderr
};

use alloc::string::String;

pub use file::FILE;
pub use file::FTYPE;
pub use file::FSEEK;
pub use dirent::DirEntry;

// pub use stdio::{Stdin, Stdout};

// FILE::open_file(path: &str, mode: u32) -> Result<FILE, &str> 
// FILE::open_file_from(dir: &FILE, path: &str, mode: u32) -> Result<FILE, &'static str> 
// FILE::seek_file(&mut self, seek: &FSEEK) -> i32
// FILE::read_file(&mut self, buf: &mut [u8]) -> Result<u32, &str>
// FILE::write_file(&mut self, buf: &[u8]) -> Result<u32, &str>
// FILE::close_file(mut self) -> Result<(), (FILE, &'static str)>
// FILE::delete_file(path: &str) -> Result<(), &str>

// FILE::make_dir(path: &str) -> Result<(), &'static str> 
// FILE::delete_dir(path: &str) -> Result<(), &'static str>
// FILE::open_dir(path: &str, mode: u32) -> Result<FILE, &'static str>
// FILE::get_dirent(&mut self) ->Result<DirEntry, &str> 

use crate::drivers::BLOCK_DEVICE;
use crate::memory::UserBuffer;
use spin::MutexGuard;

/// Trait for VirtFile. Pipe/sd card file/block device all implements VirtFile, and is stored in the PCB
pub trait VirtFile: Send + Sync {

        /// Read from a VirtFile into a buffer in user memory space
        /// # Description
        /// Read the file from the beginning to `buf.len()` or EOF and store it in `buf`, which is a memory buffer in user memory space
        /// Note that there is no offset, for pipe like object is VirtFile too.
        /// # Examples
        /// ```
        /// let file: dyn VirtFile = STDIN;
        /// let ptr: VirtAddr = 0x00100000.into();
        /// let mut buf: UserBuffer = current_process().unwrap().get_inner_locked().layout.get_user_buffer(ptr, 1000);
        /// file.read(buf);
        /// ```
        /// # Return
        /// Returns how many bytes have been read.
        fn read(&self, buf: UserBuffer) -> isize;
        

        /// Write to a VirtFile from a buffer in user memory space
        /// # Description
        /// Write the file from the beginning to `buf.len()` with content from `buf` which is a memory buffer in user memory space
        /// Note that there is no offset, for pipe like object is VirtFile too.
        /// # Examples
        /// ```
        /// let file: dyn VirtFile = STDOUT;
        /// let ptr: VirtAddr = 0x00100000.into();
        /// let mut buf: UserBuffer = current_process().unwrap().get_inner_locked().layout.get_user_buffer(ptr, 1000);
        /// file.write(buf);
        /// ```
        /// # Return
        /// Returns how many bytes have been write.
        fn write(&self, buf: UserBuffer) -> isize;

        /// Downgrade trait object to concret type FileWithLock, held lock and expose FILE for FILE manipulation
        /// # Description
        /// Calling this function will return FILE if the concret type of trait object is in fact FileWithLock, or Err otherwise.  
        /// Note: The MutexGuard will held the lock for FileWithLock, use with caution and remember to drop in case of context switching (i.e. `suspend_switch()`)
        /// # Examples
        /// ```
        /// let proc = current_process().unwrap();
        /// let arcpcb = proc.get_inner_locked();
        /// match arcpcb.files[3].to_fs_file_locked()  {
        ///     Ok(file) => {
        ///          // FILE manipulation
        ///     },
        ///     Err(msg) => {
        ///         error!("{}", msg);
        ///     }
        /// }
        /// ```
        /// # Returns
        /// On success, return `Ok(FILE)`, otherwise `Err(err_msg)`
        fn to_fs_file_locked(&self) -> Result<MutexGuard<FILE>, &str>;
}

/// Test function for SD Card.
/// # Description
/// Test SD Card by writing and reading the SD Card.
#[allow(unused)]
pub fn sdcard_test() {
        for i in 0..10 as u8 {
                let buf = [i; 512];
                BLOCK_DEVICE.write_block(i as usize, &buf);
        }

        for i in 0..10 as u8 {
                let mut buf = [0u8; 512];
                BLOCK_DEVICE.read_block(i as usize, &mut buf);
                assert_eq!(buf[i as usize], i);
        }

        info!("sdcard test passed");
}

pub fn stat_file(path_s:& str) -> Result<DirEntry, &'static str> {
        match path::parse_path(path_s) {
                Ok(path_v) => {
                        match fat::find_entry(&path_v) {
                                Ok(dirent) => {
                                        return Ok(dirent.clone());
                                },
                                Err(msg) => {
                                        return Err(msg);
                                }
                        }
                },
                Err(err) => {
                        return Err(path::to_string(err));
                }
        }
}

// ls
pub fn list(path: &str) -> Result<(), &'static str> {
        match FILE::open_dir(path, FILE::FMOD_READ) {
                Ok(mut dir) => {
                        loop{
                                match dir.get_dirent() {
                                        Ok((dirent, name)) => {
                                                println!("{}", name);
                                                // dirent.print();
                                        },
                                        Err(_) => {
                                                return Ok(());
                                        }
                                }
                        }
                },
                Err(msg) => {
                        return Err(msg);
                }
        }
}

pub fn list_tree(path: &str, level: u32) -> Result<(), &'static str> {
        const INDENT: &str = "|   ";
        let path = String::from(path);
        match FILE::open_dir(&path, FILE::FMOD_READ) {
                Ok(mut dir) => {
                        loop{
                                match dir.get_dirent() {
                                        Ok((dirent, name)) => {
                                                if dirent.get_name() == "." || dirent.get_name() == ".." {
                                                        continue;
                                                }
                                                for _i in 0..level{
                                                        print!("{}", INDENT);
                                                }
                                                // dirent.print();
                                                println!("{:11}--- {}", dirent.get_name(), name);
                                                if dirent.is_dir() {
                                                        let mut subdir = path.clone();
                                                        subdir += &name;
                                                        subdir.push('/');
                                                        // debug!("calling list_tree: {}", subdir);
                                                        if let Err(msg) = list_tree(&subdir, level + 1) {
                                                                return Err(msg);
                                                        }
                                                }
                                        },
                                        Err(_) => {
                                                // debug!("finished");
                                                return Ok(());
                                        }
                                }
                        }
                },
                Err(msg) => {
                        return Err(msg);
                }
        }
}

/// A test for File System
/// # Description
/// Test File System by creating/reading/writing Files and Folders
#[allow(unused)]
pub fn fs_test() {
        debug!("writing to test.txt");
        let mut file = FILE::open_file("/proc0", FILE::FMOD_WRITE).unwrap();
        let mut rbuf = [0u8; 512];
        match file.read_file(&mut rbuf) {
            Ok(len) => {
                error!("我们太弱小了，没有力量（哭腔");
                debug!("len: {}", len);
            },
            Err(msg) => {
                info!("{}", msg);
            }
        };
        
        let buf = r#"
        Goodbye
        "#.as_bytes();
        assert!(file.write_file(buf).unwrap() == buf.len() as u32);
        if let Err((_, msg)) = file.close_file() {
            error!("{}", msg);
        }
        
        debug!("appending to test.txt");
        let mut file = FILE::open_file("/proc0", FILE::FMOD_APPEND).unwrap();
        assert!(file.write_file(buf).unwrap() == buf.len() as u32);
        if let Err((_, msg)) = file.close_file() {
            error!("{}", msg);
        }
        
        debug!("reading test.txt");
        let mut file = FILE::open_file("/proc0", FILE::FMOD_READ).unwrap();
        let len = file.read_file(&mut rbuf).unwrap();
        let buf = &rbuf[0..len as usize];
        println!("{}", core::str::from_utf8(buf).unwrap());
        if let Err((_, msg)) = file.close_file() {
            error!("{}", msg);
        }
        
        debug!("creating new file");
        let mut file = FILE::open_file("/newfile", FILE::FMOD_CREATE | FILE::FMOD_WRITE).unwrap();
        let buf2 = "Hello, world".as_bytes();
        assert!(file.write_file(&buf2).unwrap() == buf2.len() as u32);
        if let Err((_, msg)) = file.close_file() {
            error!("{}", msg);
        }
        
        debug!("reading new file");
        let mut file = FILE::open_file("/newfile", FILE::FMOD_READ).unwrap();
        let len = file.read_file(&mut rbuf).unwrap();
        let buf = &rbuf[0..len as usize];
        println!("{}", core::str::from_utf8(buf).unwrap());
        if let Err((_, msg)) = file.close_file() {
            error!("{}", msg);
        }
        
        fat::ls_root();
        
        debug!("delete new file");
        FILE::delete_file("/newfile").unwrap();
        
        fat::ls_root();
        
        debug!("make dir new_dir");
        FILE::make_dir("/new_dir").unwrap();
        
        fat::ls_root();
        
        debug!("create file in new dir");
        let mut file = FILE::open_file("/new_dir/file", FILE::FMOD_CREATE | FILE::FMOD_WRITE).unwrap();
        assert!(file.write_file(&buf2).unwrap() == buf2.len() as u32);
        if let Err((_, msg)) = file.close_file() {
            error!("{}", msg);
        }
        
        debug!("list dir");
        let mut dir = FILE::open_dir("/new_dir", FILE::FMOD_READ).unwrap();
        loop{
            match dir.get_dirent() {
                Ok((dirent, name)) => {
                //     dirent.print();
                        println!("{}", name);
                },
                Err(msg) => {
                    debug!("{}", msg);
                    break;
                }
            }
        }
        
        debug!("read file in new dir");
        let mut file = FILE::open_file("/new_dir/file", FILE::FMOD_READ).unwrap();
        let len = file.read_file(&mut rbuf).unwrap();
        let buf = &rbuf[0..len as usize];
        println!("{}", core::str::from_utf8(buf).unwrap());
        if let Err((_, msg)) = file.close_file() {
            error!("{}", msg);
        }
        
        debug!("delete non-empty dir");
        match FILE::delete_dir("/new_dir") {
            Ok(()) => {
                error!("Not OK at all!");
            },
            Err(msg) => {
                debug!("{}", msg);
            }
        }
        
        debug!("delete empty dir");
        FILE::delete_file("/new_dir/file").unwrap();
        let mut dir = FILE::open_dir("/new_dir", FILE::FMOD_READ).unwrap();
        loop{
            match dir.get_dirent() {
                Ok((dirent, name)) => {
                //     dirent.print();
                        println!("{}", name);
                },
                Err(msg) => {
                    debug!("{}", msg);
                    break;
                }
            }
        }
        FILE::delete_dir("/new_dir").unwrap();
        info!("test passed");
}
