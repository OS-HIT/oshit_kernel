use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use bit_field::BitField;

use super::Fat32FS;
use super::inode::Inode;
use super::super::path::parse_path;
use super::dirent::write_dirent_group;
// use super::super::super::file::SeekOp;
use crate::fs::SeekOp;
use crate::fs::file::FileType;

pub const READ: usize = 1;
pub const WRITE: usize = 2;
pub const CREATE: usize = 4;
pub const DIR: usize = 8;
pub const NO_FOLLOW: usize = 16;
// const APPEND: usize = 4;

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
        pub fn new(mut inode: Inode, mode:usize) -> FileInner {
                // if has!(mode, TRUNCATE) {
                //         inode.set_size(0);
                // }
                FileInner {
                        inode,
                        cursor: 0,
                        mode,
                }
        }      

        #[inline]
        pub fn is_dir(&self) -> bool {
                self.inode.is_dir()
        }

        pub fn print(&self) {
                self.inode.print();
        }

        pub fn set_attr(&mut self, attr: u8) {
                self.inode.group.entry.attr |= attr;
        }

        pub fn reset_attr(&mut self, attr: u8) {
                self.inode.group.entry.attr &= !attr;
        }

        pub fn get_attr(&self) -> u8 {
                return self.inode.group.entry.attr;
        }

        pub fn get_path(&self) -> String {
                self.inode.path.to_string()
        }

        pub fn get_fs(&self) -> Arc<Fat32FS> {
                return self.inode.chain.fs.clone();
        }

        pub fn seek(&mut self, offset: isize, op: SeekOp) -> Result<(), &'static str> {
                if self.inode.is_dir() {
                        return Err("seek: not allowed for dir");
                }
                let new_cur = match op {
                        SeekOp::CUR => self.cursor as isize + offset,
                        SeekOp::END => self.inode.get_size() as isize + offset,
                        SeekOp::SET => offset,
                };
                if new_cur < 0 && new_cur > self.inode.get_size() as isize {
                        return Err("seek: invalid offset");
                }
                self.cursor = new_cur as usize;
                return Ok(());
        }

        pub fn get_cursor(&self) -> Result<usize, &'static str> {
                if self.inode.is_dir() {
                        return Err("get_cursor: cursor not in use for dir");
                }
                return Ok(self.cursor);
        }

        pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, &'static str> {
                let mut buffer = buffer;
                if self.inode.is_dir() {
                        return Err("read: read directory is not allowed");
                }
                if !has!(self.mode, READ) {
                        return Err("read: file not opened in read mode");
                }
                let left = self.inode.get_size() - self.cursor;
                if left == 0 {
                        return Err("read: end of file");
                }
                if left < buffer.len() {
                        buffer = &mut buffer[0..left];
                }
                match self.inode.chain.read(self.cursor, buffer) {
                        Ok(r) => return {
                                self.cursor += r;
                                Ok(r)
                        },
                        Err(_) => return Err("read: end of file"),
                }
        }

        pub fn write(&mut self, buffer: &[u8]) -> Result<usize, &'static str> {
                if self.inode.is_dir() {
                        return Err("write: write directory is not allowed");
                }
                if !has!(self.mode, WRITE) {
                        return Err("write: file not opened in write mode");
                }
                match self.inode.chain.write(self.cursor, buffer) {
                        Ok(w) => {
                                self.cursor += w;
                                if self.inode.get_size() < self.cursor {
                                        self.inode.set_size(self.cursor as u32);
                                }
                                return Ok(w);
                        },
                        Err(msg) => return Err(msg),
                }
        }

        pub fn open(&mut self, path: &str, mode:usize) -> Result<FileInner, &'static str> {
                // let fs = self.inode.chain.fs.clone();
                if !self.inode.is_dir() {
                        return Err("open: not from directory");
                }
                if self.inode.is_fake() {
                        return Err("open: from fake inode");
                }
                let mut path = match parse_path(path) {
                        Ok(p) => p,
                        Err(_) => return Err("open: path parse failed"),
                };
                if path.is_abs && self.inode.name.len() != 0 {
                        return Err("open: using abs path from non-root directory");
                }
                if !path.is_abs && self.inode.name.len() == 0{
                        return Err("open: abs path required");
                }
                let dir_flag = mode & DIR != 0;
                if path.path.len() == 0 {
                        return Err("open: empty path");
                }
                if path.must_dir && !dir_flag {
                        return Err("open: arg conflict detected, dir or not?");
                }
                let name = path.path.pop().unwrap();
                if path.path.len() == 0 {
                        match open_d(&mut self.inode, &name, mode, dir_flag, mode & NO_FOLLOW != 0) {
                                Ok(f) => return Ok(f),
                                Err(msg) => return Err(msg),
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
                                Err(_) => return Err("open: parent not found"),
                        };
                }
        }

        pub fn mkdir(&mut self, path: &str) -> Result<FileInner, &'static str> {
                if !self.inode.is_dir() {
                        return Err("mkdir: not from directory");
                }
                if self.inode.is_fake() {
                        return Err("mkdir: fake inode");
                }
                let mut path = match parse_path(path) {
                        Ok(p) => p,
                        Err(_) => return Err("mkdir: path parse failed"),
                };
                if path.is_abs && self.inode.name.len() != 0 {
                        return Err("mkdir: using abs path from non-root inode");
                }
                if !path.is_abs && self.inode.name.len() == 0{
                        return Err("mkdir: abs path required");
                }
                if path.path.len() == 0 {
                        return Err("mkdir: empty path");
                }
                let name = path.path.pop().unwrap();
                if path.path.len() > 0 {
                        path.must_dir = true;
                        let mut parent = match self.inode.find_inode_path(&path){
                                Ok(inode) => inode,
                                Err(_) => return Err("mkdir: parent not found"),
                        };
                        match parent.find_inode(&name) {
                                Ok(_) => return Err("mkdir: file/dir of the same name already exist"),
                                Err(_) => {},
                        }
                        let inode = match parent.new_dir(&name, 0) {
                                Ok(n) => n,
                                Err(msg) => return Err(msg),
                        };
                        return Ok(FileInner{
                                inode,
                                cursor: 0,
                                mode: 0,
                        });
                } else {
                        let inode = match self.inode.new_dir(&name, 0) {
                                Ok(n) => n,
                                Err(msg) => return Err(msg),
                        };
                        return Ok(FileInner{
                                inode,
                                cursor: 0,
                                mode: 0,
                        });
                }
        }

        pub fn mkfile(&mut self, path: &str) -> Result<FileInner, &'static str> {
                if !self.inode.is_dir() {
                        return Err("mkfile: not from directory");
                }
                if self.inode.is_fake() {
                        return Err("mkfile: fake inode");
                }
                let mut path = match parse_path(path) {
                        Ok(p) => p,
                        Err(_) => return Err("mkfile: path parse failed"),
                };
                if path.is_abs && self.inode.name.len() != 0 {
                        return Err("mkfile: using abs path from non-root inode");
                }
                if !path.is_abs && self.inode.name.len() == 0{
                        return Err("open: abs path required");
                }
                if path.path.len() == 0 {
                        return Err("mkfile: empty path");
                }
                let name = path.path.pop().unwrap();
                if path.path.len() > 0 {
                        path.must_dir = true;
                        let mut parent = match self.inode.find_inode_path(&path){
                                Ok(inode) => inode,
                                Err(_) => return Err("open: parent not found"),
                        };
                        match parent.find_inode(&name) {
                                Ok(_) => return Err("mkdir: file/dir of the same name already exist"),
                                Err(_) => {},
                        }
                        let inode = match parent.new_dir(&name, 0) {
                                Ok(n) => n,
                                Err(msg) => return Err(msg),
                        };
                        return Ok(FileInner{
                                inode,
                                cursor: 0,
                                mode: 0,
                        });
                } else {
                        let inode = match self.inode.new_file(&name, 0) {
                                Ok(n) => n,
                                Err(msg) => return Err(msg),
                        };
                        return Ok(FileInner{
                                inode,
                                cursor: 0,
                                mode: 0,
                        });
                }
        }

        pub fn remove(&mut self, path: &str) -> Result<(), &'static str> {
                if !self.inode.is_dir() {
                        return Err("remove: not from directory");
                }
                if self.inode.is_fake() {
                        return Err("remove: fake inode");
                }
                let mut path = match parse_path(path) {
                        Ok(p) => p,
                        Err(_) => return Err("remove: path parse failed"),
                };
                if path.is_abs && self.inode.name.len() != 0 {
                        return Err("remove: using abs path from non-root inode");
                }
                if !path.is_abs && self.inode.name.len() == 0{
                        return Err("remove: abs path required");
                }
                if path.path.len() == 0 {
                        return Err("remove: empty path");
                }
                let name = path.path.pop().unwrap();
                if path.path.len() > 0 {
                        path.must_dir = true;
                        let mut parent = match self.inode.find_inode_path(&path){
                                Ok(inode) => inode,
                                Err(_) => return Err("open: parent not found"),
                        };
                        return parent.delete_inode(&name);
                } else {
                        return self.inode.delete_inode(&name);
                }
        }

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

        pub fn rename(&mut self, new_name: &str) -> Result<(), &'static str> {
                let parent = self.inode.get_parent().unwrap();
                match parent.find_inode(new_name) {
                        Ok(_) => return Err("rename: file/dir of the same name exists"),
                        Err(_) => {},
                }
                self.inode.group.rename(new_name).unwrap();
                self.inode.name = String::from(new_name);
                return Ok(());
        }

        pub fn close(&mut self) {
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
        pub fn readable(&self) -> bool {
                !self.inode.is_dir()
        }

        pub fn writable(&self) -> bool {
                self.mode.get_bit(1) | self.mode.get_bit(2)
        }

        pub fn last_acc_time_sec(&self) -> usize {
                self.inode.group.entry.accessed_sec as usize * 86400usize
        }
        
        pub fn create_time_sec(&self) -> usize {
                self.inode.group.entry.created_date as usize * 86400usize
                + self.inode.group.entry.created_sec as usize
        }

        pub fn create_time_nsec(&self) -> usize {
                self.inode.group.entry.created_minisec as usize * 1000000usize
        }

        pub fn size(&self) -> usize {
                self.inode.get_size()
        }

        pub fn name(&self) -> String {
                self.inode.name.clone()
        }

        pub fn ftype(&self) -> FileType {
                if self.inode.is_dir() {
                        FileType::Directory
                } else {
                        FileType::Regular
                }
        }

        pub fn fmode(&self) -> usize {
                self.mode
        }
}

fn open_d(parent: &mut Inode, name: &str, mode:usize, dir_flag: bool, no_follow: bool) -> Result<FileInner, &'static str> {
        match parent.find_inode(&name) {
                Ok(mut inode) => {
                        if inode.is_slink() && !no_follow {
                                let size = inode.get_size();
                                if size > 512 {
                                        return Err("open: link path too long");
                                }
                                let mut buf = [0u8; 512];
                                inode.chain.read(0, &mut buf).unwrap();
                                let root = Inode::root(parent.chain.fs.clone());
                                let mut root = FileInner::new(root, 0);
                                return root.open(core::str::from_utf8(&buf).unwrap(), mode);
                        }
                        if dir_flag && !inode.is_dir() {
                                return Err("open_d: not a directory");
                        }
                        if !dir_flag && inode.is_dir() {
                                return Err("open_d: is a directory");
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
                                                Err(_) => {
                                                        return Err("open_d: create dir failed");
                                                }
                                        }
                                } else {
                                        match parent.new_file(&name, 0) {
                                                Ok(inode) => {
                                                        return Ok(FileInner::new(inode, mode));
                                                },
                                                Err(_) => {
                                                        return Err("open:create file failed");
                                                }
                                        }
                                }
                        } else {
                                return Err("open: file not found;");
                        }
                }
        }
}