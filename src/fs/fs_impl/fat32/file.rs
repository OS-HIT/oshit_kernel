//! File struct of Fat32
use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;

use super::Fat32FS;
use super::inode::Inode;
use super::super::super::parse_path;
use super::super::super::Path;
use super::super::super::to_string;
use super::dirent::write_dirent_group;
// use super::super::super::file::SeekOp;
use crate::fs::SeekOp;
use crate::fs::file::FileType;
use crate::process::ErrNo;

/// File Access Mode: Read allowed
pub const READ: usize = 1;
/// File Access Mode: Write allowed
pub const WRITE: usize = 2;
/// File Access Mode: Create when missing
pub const CREATE: usize = 4;
/// File Access Mode: Opening directory
pub const DIR: usize = 8;
/// File Access Mode: Don't follow symbolic links
pub const NO_FOLLOW: usize = 16;
/// File Access Mode: Set file size to 0 when open
pub const TRUNCATE: usize = 32;
// const APPEND: usize = 4;

/// File struct of Fat32
pub struct FileInner{
        inode: Inode,
        cursor: usize,
        mode: usize,
}

macro_rules! has {
        ($x:expr, $y:expr) => {
                {
                        $x & $y != 0
                }
        };
}

impl FileInner {
        /// Create a file struct for "inode" with mode "mode"
        pub fn new(mut inode: Inode, mode:usize) -> FileInner {
                if has!(mode, TRUNCATE) {
                        inode.set_size(0);
                }
                FileInner {
                        inode,
                        cursor: 0,
                        mode,
                }
        }      

        /// If the file is a symbolic link
        #[inline]
        pub fn is_link(&self) -> bool {
                self.inode.is_link()
        }

        /// If the file is a directory
        #[inline]
        pub fn is_dir(&self) -> bool {
                self.inode.is_dir()
        }

        /// Print meta data of the file
        pub fn print(&self) {
                self.inode.print();
        }

        /// Set bits in the attribute byte of the directory entry of the file
        pub fn set_attr(&mut self, attr: u8) {
                self.inode.group.entry.attr |= attr;
        }

        /// Reset bits in the attribute byte of the directory entry of the file
        pub fn reset_attr(&mut self, attr: u8) {
                self.inode.group.entry.attr &= !attr;
        }

        /// Get the attribute byte of the directory entry of the file
        pub fn get_attr(&self) -> u8 {
                return self.inode.group.entry.attr;
        }

        /// Get the path of the file in the file system
        pub fn get_path(&self) -> Path {
                let mut p  = self.inode.path.clone();
                if (self.inode.name.len() > 0) {
                        p.path.push(self.inode.name.clone());
                        p.must_dir = self.inode.is_dir();
                }
                return p;
        }

        /// Get the file system that holds the file
        pub fn get_fs(&self) -> Arc<Fat32FS> {
                return self.inode.chain.fs.clone();
        }

        /// Set file cursor
        /// # Note
        /// Setting cursor for a directory file is not allowed 
        pub fn seek(&mut self, offset: isize, op: SeekOp) -> Result<(), ErrNo> {
                if self.inode.is_dir() {
                        return Err(ErrNo::IllegalSeek);
                }
                let new_cur = match op {
                        SeekOp::CUR => self.cursor as isize + offset,
                        SeekOp::END => self.inode.get_size() as isize + offset,
                        SeekOp::SET => offset,
                };
                if new_cur < 0 && new_cur > self.inode.get_size() as isize {
                        return Err(ErrNo::InvalidArgument);
                }
                self.cursor = new_cur as usize;
                return Ok(());
        }

        /// Get file cursor
        /// # Note
        /// No cursor for a directory file
        pub fn get_cursor(&self) -> Result<usize, ErrNo> {
                if self.inode.is_dir() {
                        return Err(ErrNo::IllegalSeek);
                }
                return Ok(self.cursor);
        }

        /// Fill the buffer with contents of the file. 
        /// #Note 
        /// Reading starts from the file cursor, and set cursor to the byte next
        /// to the last read byte.
        pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ErrNo> {
                let mut buffer = buffer;
                if self.inode.is_dir() {
                        return Err(ErrNo::IsADirectory);
                }
                if !has!(self.mode, READ) {
                        return Err(ErrNo::BadFileDescriptor);
                }
                let left = self.inode.get_size() - self.cursor;
                if left < buffer.len() {
                        buffer = &mut buffer[0..left];
                }
                match self.inode.chain.read(self.cursor, buffer) {
                        Ok(r) => return {
                                self.cursor += r;
                                Ok(r)
                        },
                        Err(errno) => return Err(errno),
                }
        }

        /// Write contents of the buffer to the file
        /// # Note 
        /// Writing starts from the file cursor, and set cursor to the byte next
        /// to the last written byte.
        pub fn write(&mut self, buffer: &[u8]) -> Result<usize, ErrNo> {
                if self.inode.is_dir() {
                        return Err(ErrNo::IsADirectory);
                }
                if !has!(self.mode, WRITE) {
                        return Err(ErrNo::BadFileDescriptor);
                }
                match self.inode.chain.write(self.cursor, buffer) {
                        Ok(w) => {
                                self.cursor += w;
                                if self.inode.get_size() < self.cursor {
                                        self.inode.set_size(self.cursor as u32);
                                }
                                return Ok(w);
                        },
                        Err(errno) => return Err(errno),
                }
        }

        /// Open a file from file "self". "self" must be a directory.
        pub fn open(&mut self, mut path: Path, mode:usize) -> Result<FileInner, ErrNo> {
                // let fs = self.inode.chain.fs.clone();
                if !self.inode.is_dir() {
                        return Err(ErrNo::NotADirectory);
                }
                if self.inode.is_fake() {
                        return Err(ErrNo::Fat32FakeInode);
                }
                if path.is_abs && self.inode.name.len() != 0 {
                        return Err(ErrNo::InvalidArgument);
                }
                if !path.is_abs && self.inode.name.len() == 0{
                        return Err(ErrNo::InvalidArgument);
                }
                let dir_flag = mode & DIR != 0;
                if path.path.len() == 0 {
                        return Err(ErrNo::InvalidArgument);
                }
                if path.must_dir && !dir_flag {
                        return Err(ErrNo::IsADirectory);
                }
                let name = path.path.pop().unwrap();
                if path.path.len() == 0 {
                        match open_d(&mut self.inode, &name, mode, dir_flag, mode & NO_FOLLOW != 0) {
                                Ok(f) => return Ok(f),
                                Err(errno) => return Err(errno),
                        };
                } else {
                        path.must_dir = true;
                        match self.inode.find_inode_path(&path){
                                Ok(mut parent) => {
                                        match open_d(&mut parent, &name, mode, dir_flag, mode & NO_FOLLOW != 0) {
                                                Ok(f) => return Ok(f),
                                                Err(msg) => return Err(msg),
                                        };
                                }
                                Err(_) => return Err(ErrNo::NoSuchFileOrDirectory),
                        };
                }
        }

        /// Create a directory file at file "self". "self" must be a directory.
        pub fn mkdir(&mut self, mut path: Path) -> Result<FileInner, ErrNo> {
                if !self.inode.is_dir() {
                        return Err(ErrNo::NotADirectory);
                }
                if self.inode.is_fake() {
                        return Err(ErrNo::Fat32FakeInode);
                }
                if path.is_abs && self.inode.name.len() != 0 {
                        return Err(ErrNo::InvalidArgument);
                }
                if !path.is_abs && self.inode.name.len() == 0{
                        return Err(ErrNo::InvalidArgument);
                }
                if path.path.len() == 0 {
                        return Err(ErrNo::FileExists);
                }
                let name = path.path.pop().unwrap();
                if path.path.len() > 0 {
                        path.must_dir = true;
                        let mut parent = self.inode.find_inode_path(&path)?;
                        match parent.find_inode(&name) {
                                Ok(_) => return Err(ErrNo::FileExists),
                                Err(_) => {},
                        }
                        let inode = parent.new_dir(&name, 0)?;
                        return Ok(FileInner{
                                inode,
                                cursor: 0,
                                mode: 0,
                        });
                } else {
                        match self.inode.find_inode(&name) {
                                Ok(_) => return Err(ErrNo::FileExists),
                                Err(_) => {},
                        }
                        let inode = self.inode.new_dir(&name, 0)?;
                        return Ok(FileInner{
                                inode,
                                cursor: 0,
                                mode: 0,
                        });
                }
        }

        /// Create a regular file at file "self". "self" must be a directory.
        pub fn mkfile(&mut self, mut path: Path) -> Result<FileInner, ErrNo> {
                if !self.inode.is_dir() {
                        return Err(ErrNo::NotADirectory);
                }
                if self.inode.is_fake() {
                        return Err(ErrNo::Fat32FakeInode);
                }
                if path.is_abs && self.inode.name.len() != 0 {
                        return Err(ErrNo::InvalidArgument);
                }
                if !path.is_abs && self.inode.name.len() == 0{
                        return Err(ErrNo::InvalidArgument);
                }
                if path.path.len() == 0 {
                        return Err(ErrNo::InvalidArgument);
                }
                let name = path.path.pop().unwrap();
                if path.path.len() > 0 {
                        path.must_dir = true;
                        let mut parent = self.inode.find_inode_path(&path)?;
                        match parent.find_inode(&name) {
                                Ok(_) => return Err(ErrNo::FileExists),
                                Err(_) => {},
                        }
                        let inode = parent.new_dir(&name, 0)?;
                        return Ok(FileInner{
                                inode,
                                cursor: 0,
                                mode: 0,
                        });
                } else {
                        let inode = self.inode.new_file(&name, 0)?;
                        return Ok(FileInner{
                                inode,
                                cursor: 0,
                                mode: 0,
                        });
                }
        }

        /// Delete a regular file or empty directory file at file "self". "self" must be a directory.
        pub fn remove(&mut self, mut path: Path) -> Result<(), ErrNo> {
                if !self.inode.is_dir() {
                        return Err(ErrNo::NotADirectory);
                }
                if self.inode.is_fake() {
                        return Err(ErrNo::Fat32FakeInode);
                }
                if path.is_abs && self.inode.name.len() != 0 {
                        return Err(ErrNo::InvalidArgument);
                }
                if !path.is_abs && self.inode.name.len() == 0{
                        return Err(ErrNo::InvalidArgument);
                }
                if path.path.len() == 0 {
                        return Err(ErrNo::InvalidArgument);
                }
                let name = path.path.pop().unwrap();
                if path.path.len() > 0 {
                        path.must_dir = true;
                        let mut parent = match self.inode.find_inode_path(&path){
                                Ok(inode) => inode,
                                Err(_) => return Err(ErrNo::NoSuchFileOrDirectory),
                        };
                        return parent.delete_inode(&name);
                } else {
                        return self.inode.delete_inode(&name);
                }
        }

        /// List all files in file "self". "self" must be a directory.
        pub fn list(&self) -> Result<Vec<FileInner>, &'static str> {
                if !self.inode.is_dir() {
                        return Err("list: not from directory");
                }
                if self.inode.is_fake() {
                        return Err("list: fake inode");
                }
                let inodes = self.inode.get_inodes().unwrap();
                let mut files = Vec::<FileInner>::new();
                for inode in inodes {
                        files.push(FileInner {
                                inode,
                                cursor: 0,
                                mode: 0,
                        })
                }
                return Ok(files);
        }

        /// Rename the file
        pub fn rename(&mut self, new_name: &str) -> Result<(), ErrNo> {
                let parent = self.inode.get_parent().unwrap();
                match parent.find_inode(new_name) {
                        Ok(_) => return Err(ErrNo::FileExists),
                        Err(_) => {},
                }
                self.inode.group.rename(new_name).unwrap();
                self.inode.name = String::from(new_name);
                return Ok(());
        }

        /// Flush file meta data
        /// # Note 
        /// close() can be called for multiple times for a file. 
        /// It does no more than flushing meta data.
        pub fn close(&mut self) {
                if self.inode.name.len() == 0 {
                        return ;
                }
                if !self.inode.is_dir() {
                        if self.inode.group.get_start() == 0 && self.inode.chain.chain.len() != 0 {
                                self.inode.group.entry.set_start(self.inode.chain.chain[0]);
                        }
                        let csize = self.inode.chain.fs.cluster_size();
                        let clen = (self.inode.get_size() + csize - 1) / csize;
                        self.inode.chain.truncate(clen).unwrap();
                }
                let mut parent = self.inode.get_parent().unwrap();
                write_dirent_group(&mut parent.chain, &mut self.inode.group).unwrap();
                self.inode.chain.fs.sync();
        }

        /// If the file is readable
        pub fn readable(&self) -> bool {
                // !self.inode.is_dir()
                has!(self.mode, READ)
        }

        /// If the file is writable
        pub fn writable(&self) -> bool {
                // self.mode.get_bit(1) | self.mode.get_bit(2)
                has!(self.mode, WRITE)
        }

        /// Get last accessed time of the file
        pub fn last_acc_time_sec(&self) -> usize {
                self.inode.group.entry.accessed_sec as usize * 86400usize
        }
        
        /// Get create time (sec) of the file
        pub fn create_time_sec(&self) -> usize {
                self.inode.group.entry.created_date as usize * 86400usize
                + self.inode.group.entry.created_sec as usize
        }

        /// Get create time (nsec) of the file
        pub fn create_time_nsec(&self) -> usize {
                self.inode.group.entry.created_minisec as usize * 1000000usize
        }

        /// Get file size
        /// # Note
        /// File size of a directory file is 0
        pub fn size(&self) -> usize {
                self.inode.get_size()
        }

        /// Get file name
        pub fn name(&self) -> String {
                self.inode.name.clone()
        }

        /// Get file type
        pub fn ftype(&self) -> FileType {
                if self.inode.is_link() {
                        FileType::Link
                } else if self.inode.is_dir() {
                        FileType::Directory
                } else {
                        FileType::Regular
                }
        }

        /// Get file mode
        pub fn fmode(&self) -> usize {
                self.mode
        }
}

fn open_d(parent: &mut Inode, name: &str, mode:usize, dir_flag: bool, no_follow: bool) -> Result<FileInner, ErrNo> {
        match parent.find_inode(&name) {
                Ok(mut inode) => {
                        if inode.is_slink() && !no_follow {
                                let size = inode.get_size();
                                if size > 512 {
                                        return Err(ErrNo::FileNameTooLong);
                                }
                                let mut buf = [0u8; 512];
                                inode.chain.read(0, &mut buf).unwrap();
                                let path = match parse_path(&core::str::from_utf8(&buf).unwrap()) {
                                        Ok(path) => path,
                                        Err(err) => return Err(ErrNo::InvalidArgument),
                                };
                                let root = Inode::root(parent.chain.fs.clone());
                                let mut root = FileInner::new(root, 0);
                                return root.open(path, mode);
                        }
                        if dir_flag && !inode.is_dir() {
                                return Err(ErrNo::NotADirectory);
                        }
                        if !dir_flag && inode.is_dir() {
                                return Err(ErrNo::IsADirectory);
                        }
                        if inode.is_fake() {
                                inode = inode.realize().unwrap();
                        }
                        return Ok(FileInner::new(inode, mode));
                },
                Err(_) => {
                        if mode & CREATE != 0 {
                                if dir_flag {
                                        match parent.new_dir(&name, 0) {
                                                Ok(inode) => {
                                                        return Ok(FileInner::new(inode, mode));
                                                },
                                                Err(errno) => {
                                                        return Err(errno);
                                                }
                                        }
                                } else {
                                        match parent.new_file(&name, 0) {
                                                Ok(inode) => {
                                                        return Ok(FileInner::new(inode, mode));
                                                },
                                                Err(errno) => {
                                                        return Err(errno);
                                                }
                                        }
                                }
                        } else {
                                return Err(ErrNo::NoSuchFileOrDirectory);
                        }
                }
        }
}