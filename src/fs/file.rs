
use core::mem::size_of;

use alloc::vec::Vec;
use alloc::string::ToString;

use alloc::string::String;

use super::dirent::DirEntry;

use super::fat::CLUSTER_SIZE;
use super::fat::read_cluster;
use super::fat::write_cluster;
use super::fat::find_entry;
use super::fat::update_entry;
use super::fat::flush;
use super::fat::append_chain;

use super::fat::fat::clear_file_chain;
use super::fat::fat::truncat_file_chain;
use super::fat::fat::get_free_cluster;

pub struct FILE {
        path: String,
        ftype: FTYPE,
        fchain: Vec<u32>,
        fsize: u32,
        offset: u32,
        flag: u32,
}

#[derive(PartialEq)]
pub enum FTYPE {
        TDir,
        TFile,
}

impl FILE {
        // pub const FLAG_READ: u32= 1;
        // pub const FLAG_WRITE: u32 = 2;

        pub const FMOD_READ: u32 = 1;
        pub const FMOD_WRITE: u32 = 2;
        pub const FMOD_CREATE: u32 = 4;
        pub const FMOD_APPEND: u32 = 8;

        const FMOD_IMPL: u32 = FILE::FMOD_READ | FILE::FMOD_WRITE;

        #[inline]
        fn implemented(mode: u32) -> bool {
                return mode & FILE::FMOD_IMPL == mode;
        }

        fn read_allowed(&self) -> bool {
                return self.flag & FILE::FMOD_READ != 0;
        }

        fn write_allowed(&self) -> bool {
                return self.flag & FILE::FMOD_WRITE != 0;
        }
        
        
        pub fn open_file(path: &str, mode: u32) -> Result<FILE, &str> {
                if !FILE::implemented(mode) {
                        return Err("open_dir: Not implemented yet");
                }
                match find_entry(path) {
                        Ok(entry) => {

                                if ! entry.is_file() {
                                        return Err("Not a file");
                                } 
                                let fsize = if mode & FILE::FMOD_WRITE != 0 && mode & FILE::FMOD_READ == 0 {
                                        0
                                } else {
                                        entry.size
                                };
                                return Ok(
                                FILE {
                                        path: path.to_string(),
                                        ftype: FTYPE::TFile,
                                        fchain: entry.get_chain(),
                                        offset: 0,
                                        fsize,
                                        flag: mode,
                                })

                        },
                        Err(msg) => {
                                return Err(msg);
                        }
                }
        }

        pub fn delete_file(path: &str) -> Result<(), &str> {
                match find_entry(path) {
                        Ok(entry) => {
                                if ! entry.is_file() {
                                        return Err("Not a file");
                                }
                                clear_file_chain(entry.get_start()).unwrap();
                                super::fat::delete_entry(path).unwrap();
                                return Ok(());
                        },
                        Err(msg) => {
                                return Err(msg);
                        }
                }
        }

        pub fn open_dir(path: &str, mode: u32) -> Result<FILE, &str> {
                if mode != FILE::FMOD_READ {
                        return Err("open_dir: Not implemented yet");
                }
                match find_entry (path) {
                        Ok(entry) => {
                                if ! entry.is_dir() {
                                        return Err("open_dir: Not a directory");
                                }
                                return Ok(
                                FILE {
                                        path: path.to_string(),
                                        ftype: FTYPE::TDir,
                                        fchain: entry.get_chain(),
                                        offset: 0,
                                        fsize: entry.size,
                                        flag: mode,       
                                })
                        },
                        Err(msg) => {
                                return Err(msg);
                        }
                }
        }

        #[inline]
        fn get_cur_cluster(&self) -> Result<u32, &str> {
                // if self.offset > self.fsize {
                //         return Err("FILE::get_cur_cluster: invaid offset");
                // }
                let idx = self.offset / *CLUSTER_SIZE;
                if idx >= self.fchain.len() as u32 {
                        return Err("FILE::get_cur_cluster: invalid offset");
                }
                return Ok(self.fchain[idx as usize]);
        }

        pub fn get_dirent(&mut self) ->Result<DirEntry, &str> {
                if self.ftype != FTYPE::TDir {
                        return Err("get_dirent: not a directory");
                }
                if !self.read_allowed() {
                        return Err("get_dirent: read not allowed");
                } 
                if self.offset >= self.fsize {
                        return Err("get_dirent: End of dir");
                }
                let mut buf = [0u8; size_of::<DirEntry>()];
                loop {
                        if read_cluster(self.get_cur_cluster().unwrap(), self.offset % *CLUSTER_SIZE, &mut buf).unwrap() 
                                != size_of::<DirEntry>() as u32 {
                                return Err("get_dirent: short read");
                        }
                        let dirent: DirEntry = unsafe { core::ptr::read(buf.to_vec().as_ptr() as *const _) };
                        if !dirent.deleted() && !dirent.is_ext() {
                                return Ok(dirent);
                        }
                        self.offset += size_of::<DirEntry>() as u32;
                        if self.offset >= self.fsize {
                                return Err("get_dirent: End of dir");
                        }
                }
        }

        pub fn read_file(&mut self, buf: &mut [u8]) -> Result<u32, &str> {
                if self.ftype == FTYPE::TDir {
                        return Err("read_file: This is a directory");
                }
                
                if !self.read_allowed() {
                        return Err("read_file: Read is not allowed");
                }

                let rest = self.fsize - self.offset;
                let mut rbuf = buf;
                let len = if rest < rbuf.len() as u32 {
                                rbuf = &mut rbuf[..rest as usize];
                                rest        
                        } else {
                                rbuf.len() as u32
                        };

                let mut read = 0;
                while read < len {
                        let read_len = read_cluster(self.get_cur_cluster().unwrap(), self.offset % *CLUSTER_SIZE, rbuf).unwrap();
                        self.offset += read_len;
                        read += read_len;
                        rbuf = &mut rbuf[(read as usize)..];
                }
                return Ok(read);
        }

        pub fn write_file(&mut self, buf: &[u8]) -> Result<u32, &str> {
                if self.ftype == FTYPE::TDir {
                        return Err("write_file: This is a directory");
                }
                if !self.write_allowed() {
                        return Err("write_file: write is not allowed for this file");
                }
                let mut wlen = 0;
                let mut blen = buf.len() as u32;
                let mut wbuf = buf;
                while blen > 0 {
                        if let Ok(cluster) = self.get_cur_cluster() {
                                let off = self.offset % *CLUSTER_SIZE;
                                let write_len = write_cluster(cluster, off, wbuf).unwrap();
                                wbuf = &wbuf[(write_len as usize)..];
                                blen -= write_len;
                                self.offset += write_len;
                                wlen += write_len;
                        } else {
                                if self.fchain.len() == 0 {
                                        if let Ok(new) = get_free_cluster() {
                                                self.fchain.push(new);
                                                continue;
                                        } else {
                                                return Err("write_file: no free cluster");
                                        }
                                } else if let Ok(new) = append_chain(self.fchain[self.fchain.len() - 1]) {
                                        self.fchain.push(new);
                                        continue;
                                } else {
                                        return Err("write_file: append cluster failed");
                                }
                                // return Err("write_file: adding clusters is not implemented");
                        }
                }

                if self.fsize < self.offset {
                        self.fsize = self.offset;
                }
                return Ok(wlen);
        }

        pub fn close_file(self) -> Result<(), (FILE, &'static str)> {
                if self.ftype != FTYPE::TFile {
                        return Err((self, "FILE::close_file: not a file"));
                }
                if self.write_allowed() {
                        let mut entry = find_entry(&self.path).unwrap();
                        entry.size = self.fsize;
                        let mut chain_len = self.fsize / *CLUSTER_SIZE;
                        if self.fsize % *CLUSTER_SIZE != 0 {
                                chain_len += 1;
                        }
                        if self.fchain.len() != 0 {
                                entry.set_start(self.fchain[0]);
                        }
                        if chain_len < self.fchain.len() as u32 {
                                if chain_len == 0 {
                                        println!("fchain[0]:{}", self.fchain[0]);
                                        clear_file_chain(self.fchain[0]).unwrap();
                                        println!("Checkpoint");
                                        // self.fchain = Vec::new();
                                } else {
                                        truncat_file_chain(self.fchain[chain_len as usize - 1]).unwrap();
                                }
                        }
                        update_entry(&self.path, &entry).unwrap();
                        flush();
                }
                return Ok(());
        }
}
