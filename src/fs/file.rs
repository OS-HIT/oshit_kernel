
use core::{convert::TryInto, mem::size_of};

use alloc::vec::Vec;

use super::dirent::DirEntry;
use super::dirent::DIRENT_P_CLST;

use super::fat::CLUSTER_SIZE;
use super::fat::ROOT_DIR;
use super::fat::read_cluster;
use super::fat::write_cluster;
use super::fat::find_entry;
use super::fat::find_entry_from;
use super::fat::update_entry;
use super::fat::new_entry;
use super::fat::new_entry_at;
use super::fat::delete_entry;
use super::fat::read_dirent;
use super::fat::flush;
use super::fat::append_chain;

use super::path::Path;
use super::path::PathFormatError;
use super::path::parse_path;
use super::path::parse_path_r;
use super::path::to_string;
use super::path::get_name;
use super::path::get_ext;
use super::fat::fat::get_file_chain;
use super::fat::fat::clear_file_chain;
use super::fat::fat::truncat_file_chain;
use super::fat::fat::get_free_cluster;

#[derive(Clone)]
pub struct FILE {
        pub path: Path,
        pub ftype: FTYPE,
        pub fchain: Vec<u32>,
        pub fsize: u32,
        pub cursor: u32,
        pub flag: u32,
}

#[derive(PartialEq)]
#[derive(Clone, Copy, Debug)]
pub enum FTYPE {
        TDir,
        TFile,
        TStdIn,
        TStdOut,
        TStdErr,
        TFree,
}

pub enum FSEEK {
        SET(i32),
        CUR(i32),
        END(i32),
}

impl Drop for FILE {
    fn drop(&mut self) {
        self.close_file();      // TODO: Ask Shi Jvlao for this
    }
}

impl FILE {
        // pub const FLAG_READ: u32= 1;
        // pub const FLAG_WRITE: u32 = 2;

        pub const FMOD_READ: u32 = 1;
        pub const FMOD_WRITE: u32 = 2;
        pub const FMOD_CREATE: u32 = 4;
        pub const FMOD_APPEND: u32 = 8;

        #[inline]
        fn implemented(mode: u32) -> bool {
                return (mode == FILE::FMOD_READ) 
                        || (mode == FILE::FMOD_WRITE)
                        || (mode == FILE::FMOD_WRITE | FILE::FMOD_CREATE)
                        || (mode == FILE::FMOD_READ | FILE::FMOD_WRITE)
                        || (mode == FILE::FMOD_APPEND)
                        // || (mode == FILE::FMOD_APPEND | FILE::FMOD_CREATE)
        }

        #[inline]
        fn read_allowed(&self) -> bool {
                return self.flag & FILE::FMOD_READ != 0;
        }

        #[inline]
        fn write_allowed(&self) -> bool {
                return self.flag & FILE::FMOD_WRITE != 0;
        }

        #[inline]
        fn append_allowed(&self) -> bool {
                return self.flag & FILE::FMOD_APPEND != 0;
        }

        #[inline]
        fn do_create(&self) -> bool {
                return self.flag & FILE::FMOD_CREATE != 0;
        }
        
        fn open_file_path(path: Path, mode: u32) -> Result<FILE, &'static str> {
                match find_entry(&path, false) {
                        Ok(entry) => {
                                if ! entry.is_file() {
                                        return Err("open_file:Not a file");
                                } 
                                if entry.is_read_only() && mode & FILE::FMOD_WRITE != 0 {
                                        return Err("open_file: read only file");
                                }
                                let cursor = if mode & FILE::FMOD_APPEND != 0 {
                                        entry.size
                                } else {
                                        0
                                };
                                let fsize = if mode & FILE::FMOD_WRITE != 0 && mode & FILE::FMOD_READ == 0 {
                                        0
                                } else {
                                        entry.size
                                };
                                return Ok(
                                FILE {
                                        path: path,
                                        ftype: FTYPE::TFile,
                                        fchain: entry.get_chain(),
                                        cursor,
                                        fsize,
                                        flag: mode,
                                });
                        },
                        Err(msg) => {
                                if mode & FILE::FMOD_CREATE != 0 {
                                        let mut parent = path.clone();
                                        parent.pop().unwrap();
                                        if parent.len() != 0 {
                                                if let Err(_) = find_entry(&parent, true) {
                                                        return Err("open_file: parent directory not exists");
                                                }
                                        }

                                        return Ok(
                                        FILE {
                                                path,
                                                ftype: FTYPE::TFile,
                                                fchain: Vec::new(),
                                                cursor: 0,
                                                fsize: 0,
                                                flag: mode,
                                        });
                                } else {
                                        return Err(msg);
                                }
                        }
                }
        }

        pub fn open_file_from(dir: &FILE, path: &str, mode: u32) -> Result<FILE, &'static str> {
                if !FILE::implemented(mode) {
                        return Err("open_file: Not implemented yet");
                }
                let path = match parse_path(path) {
                        Ok((path, is_dir)) => {
                                if is_dir {
                                        return Err("open_file_from: Cannot open dir");
                                }
                                path
                        },
                        Err(PathFormatError::NotAbs) => {
                                match parse_path_r(path) {
                                        Ok((path, is_dir)) => {
                                                if is_dir {
                                                        return Err("open_file_from: Cannot open dir");
                                                }
                                                if dir.ftype == FTYPE::TDir {
                                                        let path_tmp = dir.path.clone();
                                                        path_tmp.append(&mut path);
                                                        path_tmp
                                                } else {
                                                        return Err("open_file_from: Are you sure you are giving me a directory?");
                                                }
                                        },
                                        Err(err) => {
                                                return Err(to_string(err));
                                        }
                                }
                        }
                };
                FILE::open_file_path(path, mode)
        }
        
        pub fn open_file(path: &str, mode: u32) -> Result<FILE, &'static str> {
                if !FILE::implemented(mode) {
                        return Err("open_file: Not implemented yet");
                }
                let path = match parse_path(path) {
                        Ok((path, is_dir)) => {
                                if is_dir {
                                        return Err("open_file: Cannot open dir");
                                }
                                path
                        },
                        Err(error) => {
                                return Err(to_string(error));
                        }
                };
                FILE::open_file_path(path, mode)
        }

        pub fn delete_file(path: &str) -> Result<(), &str> {
                let (path, is_dir) = match parse_path(path) {
                        Ok((path, is_dir)) => {
                                (path, is_dir)
                        },
                        Err(error) => {
                                return Err(to_string(error));
                        }
                };
                if is_dir {
                        return Err("delete_file: input path is referring a directory");
                }
                match find_entry(&path, false) {
                        Ok(entry) => {
                                if ! entry.is_file() {
                                        return Err("delete_file: Not a file");
                                }
                                clear_file_chain(entry.get_start()).unwrap();
                                delete_entry(&path, false).unwrap();
                                flush();
                                return Ok(());
                        },
                        Err(msg) => {
                                return Err(msg);
                        }
                }
        }

        pub fn open_dir(path: &str, mode: u32) -> Result<FILE, &'static str> {
                if mode != FILE::FMOD_READ {
                        return Err("open_dir: Not implemented yet");
                }
                let path = match parse_path(path) {
                        Ok((path, _)) => {
                                path
                        },
                        Err(error) => {
                                return Err(to_string(error));
                        }
                };
                if path.len() == 0 {
                        return Ok(
                        FILE{
                                path,
                                ftype: FTYPE::TDir,
                                fchain: get_file_chain(*ROOT_DIR),
                                cursor: 0,
                                fsize: 0,
                                flag: mode,
                        })
                }
                match find_entry(&path, true) {
                        Ok(entry) => {
                                return Ok(
                                FILE {
                                        path: path,
                                        ftype: FTYPE::TDir,
                                        fchain: entry.get_chain(),
                                        cursor: 0,
                                        fsize: 0,
                                        flag: mode,       
                                })
                        },
                        Err(msg) => {
                                return Err(msg);
                        }
                }
        }

        pub fn make_dir(path: &str) -> Result<(), &'static str> {
                let mut path = match parse_path(path) {
                        Ok((path, _)) => path,
                        Err(error) => return Err(to_string(error)),
                };
                if path.len() == 0 {
                        return Err("make_dir: Are you trying to make root directory?");
                }
                let dir = path.pop().unwrap();
                let mut dir = vec![dir];
                let cluster: u32;
                let dirent: DirEntry;
                let mut pstart = *ROOT_DIR;
                if path.len() == 0 {
                        if let Err(_) = find_entry(&dir, false) {
                                let dir = dir.pop().unwrap();
                                let mut name = [0u8;8];
                                name.clone_from_slice(get_name(&dir).as_bytes());
                                let mut ext = [0u8;3];
                                ext.clone_from_slice(get_ext(&dir).as_bytes());
                                cluster = get_free_cluster().unwrap();
                                dirent = DirEntry{
                                        name, ext, attr: DirEntry::attr_dir(),
                                        reserved: 0, created_minisec: 0,
                                        created_sec: 0, created_date: 0,
                                        accessed_sec: 0, size: 0,
                                        start_h: (cluster >> 16) as u16,
                                        mod_sec: 0, mod_date: 0,
                                        start_l: (cluster & 0xff) as u16,
                                };
                                new_entry(&path, &dirent).unwrap();
                        } else {
                                return Err("make_dir: directory name occupied");
                        }
                } else {
                        if let Ok(mut parent) = find_entry(&path, true) {
                                if let Err(_) = find_entry_from(parent.get_start(), &dir, false) {
                                        let dir = dir.pop().unwrap();
                                        let mut name = [0u8;8];
                                        name.clone_from_slice(get_name(&dir).as_bytes());
                                        let mut ext = [0u8;3];
                                        ext.clone_from_slice(get_ext(&dir).as_bytes());
                                        cluster = get_free_cluster().unwrap();
                                        dirent = DirEntry{
                                                name, ext, attr: DirEntry::attr_dir(),
                                                reserved: 0, created_minisec: 0,
                                                created_sec: 0, created_date: 0,
                                                accessed_sec: 0, size: 2 * size_of::<DirEntry>() as u32,
                                                start_h: (cluster >> 16) as u16,
                                                mod_sec: 0, mod_date: 0,
                                                start_l: (cluster & 0xff) as u16,
                                        };
                                        if let Ok(update_size) = new_entry_at(&parent, &dirent) {
                                                if update_size {
                                                        parent.size += size_of::<DirEntry>() as u32;
                                                        update_entry(&path, true, &parent).unwrap();
                                                }
                                        } else {
                                                return Err("make_dir: failed to set new entry");
                                        }
                                        pstart = parent.get_start()
                                } else {
                                        return Err("make_dir: directory name occupied");
                                }
                        } else {
                                return Err("make_dir: parent direcotry not found");
                        };
                }

                let mut name = [' ' as u8; 8];
                name[0] = '.' as u8;
                let ext = [' ' as u8; 3];
                let mut dir_tmp = DirEntry{
                        name, ext, attr: DirEntry::attr_dir(),
                        reserved: 0, created_minisec: 0,
                        created_sec: 0, created_date: 0,
                        accessed_sec: 0, size: 0,
                        start_h: (cluster >> 16) as u16,
                        mod_sec: 0, mod_date: 0,
                        start_l: (cluster & 0xff) as u16,
                };
                new_entry_at(&dirent, &dir_tmp).unwrap();
                dir_tmp.name[1] = '.' as u8;
                if path.len() == 0 {
                        dir_tmp.set_start(*ROOT_DIR);
                } else {
                        dir_tmp.set_start(pstart);
                }
                new_entry_at(&dirent, &dir_tmp).unwrap();
                flush();
                return Ok(());
        }

        fn is_empty_dir(entry: &DirEntry) -> bool {
                let chain = entry.get_chain();
                for clst in chain {
                        for offset in 0..*DIRENT_P_CLST {
                                if let Some(dirent) = read_dirent(clst, offset) {
                                        if dirent.is_ext() || dirent.deleted() {
                                                continue;
                                        }
                                        let name = dirent.get_name();
                                        if "." == name || ".." == name {
                                                continue;
                                        }
                                        return false;
                                } else {
                                        break;
                                }
                        }
                }
                return true;
        }

        pub fn delete_dir(path: &str) -> Result<(), &'static str> {
                let mut path = match parse_path(path) {
                        Ok((path, _)) => path,
                        Err(error) => return Err(to_string(error)),
                };
                if path.len() == 0 {
                        return Err("delete_dir: deleting root directory is not allowed");
                }
                let dir = path.pop().unwrap();
                let dir = vec![dir];
                if path.len() == 0 {
                        if let Ok(entry) = find_entry(&dir, true) {
                                if FILE::is_empty_dir(&entry) {
                                        delete_entry(&dir, true).unwrap();
                                        clear_file_chain(entry.get_start()).unwrap();
                                        flush();
                                        return Ok(());
                                } else {
                                        return Err("delete_dir: cannot delete non-empty directory");
                                }
                        } else {
                                return Err("delete_dir: directory not found");
                        }
                } else {
                        if let Ok(parent) = find_entry(&path, true) {
                                if let Ok(entry) = find_entry_from(parent.get_start(), &dir, true) {
                                        if FILE::is_empty_dir(&entry) {
                                                clear_file_chain(entry.get_start()).unwrap();
                                                delete_entry(&dir, true).unwrap();
                                                flush();
                                                return Ok(());
                                        } else {
                                                return Err("delete_dir: cannot delete non-empty directory");
                                        }
                                } else {
                                        return Err("delete_dir: directory not found");
                                }
                        } else {
                                return Err("delete_dir: directory not found");
                        }
                }
        }

        #[inline]
        fn get_cur_cluster(&self) -> Result<u32, &str> {
                // if self.cursor > self.fsize {
                //         return Err("FILE::get_cur_cluster: invaid offset");
                // }
                let idx = self.cursor / *CLUSTER_SIZE;
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

                let mut buf = [0u8; size_of::<DirEntry>()];
                loop {
                        if self.fsize != 0 {
                                return Err("get_dirent: End of dir");
                        }
                        if read_cluster(self.get_cur_cluster().unwrap(), self.cursor % *CLUSTER_SIZE, &mut buf).unwrap() 
                                != size_of::<DirEntry>() as u32 {
                                return Err("get_dirent: short read");
                        }
                        
                        self.cursor += size_of::<DirEntry>() as u32;
                        if buf[0] == 0 {
                                self.fsize = 1;
                                return Err("get_dirent: End of dir");
                        }
                        let dirent: DirEntry = unsafe { core::ptr::read(buf.to_vec().as_ptr() as *const _) };
                        if !dirent.deleted() && !dirent.is_ext() {
                                return Ok(dirent.clone());
                        }
                }
        }

        pub fn seek_file(&mut self, seek: &FSEEK) -> i32 {
                match seek {
                        FSEEK::SET(offset) => {
                                let cursor = *offset;
                                if cursor < 0 {
                                        self.cursor = 0;
                                        return 0;
                                } else if cursor > self.fsize as i32 {
                                        self.cursor = self.fsize;
                                        return self.fsize as i32;
                                } else {
                                        self.cursor = *offset as u32;
                                        return *offset;
                                }
                        },
                        FSEEK::CUR(offset) => {
                                let cursor = offset + self.cursor as i32;
                                if cursor < 0 {
                                        let offset = - (self.cursor as i32);
                                        self.cursor = 0;
                                        return offset;
                                } else if cursor > self.fsize as i32 {
                                        let offset = self.fsize as i32 - self.cursor as i32;
                                        self.cursor = self.fsize;
                                        return offset;
                                } else {
                                        self.cursor = cursor as u32;
                                        return *offset;
                                }
                        },
                        FSEEK::END(offset) => {
                                let cursor = offset + self.fsize as i32;
                                if cursor < 0 {
                                        let offset = - (self.fsize as i32);
                                        self.cursor = 0;
                                        return offset;
                                } else if cursor > self.fsize as i32 {
                                        self.cursor = self.fsize;
                                        return 0;
                                } else {
                                        self.cursor = cursor as u32;
                                        return *offset;
                                }
                        },
                };
        }

        pub fn read_file(&mut self, buf: &mut [u8]) -> Result<u32, &'static str> {

                if self.ftype != FTYPE::TFile {
                        return Err("read_file: Not a regular file");
                }
                
                if !self.read_allowed() {
                        return Err("read_file: Read is not allowed");
                }

                let rest = self.fsize - self.cursor;
                let mut rbuf = buf;
                let len = if rest < rbuf.len() as u32 {
                                rbuf = &mut rbuf[..rest as usize];
                                rest        
                        } else {
                                rbuf.len() as u32
                        };

                let mut read = 0;
                while read < len {
                        let read_len = read_cluster(self.get_cur_cluster().unwrap(), self.cursor % *CLUSTER_SIZE, rbuf).unwrap();
                        self.cursor += read_len;
                        read += read_len;
                        rbuf = &mut rbuf[(read_len as usize)..];
                }
                return Ok(read);
        }

        pub fn write_file(&mut self, buf: &[u8]) -> Result<u32, &str> {
                if self.ftype != FTYPE::TFile {
                        return Err("write_file: Not a regular file");
                }
                if !self.write_allowed() {
                        if !self.append_allowed() {
                                return Err("write_file: write is not allowed for this file");
                        } else if self.cursor != self.fsize {
                                return Err("write_file: this file is append only, please set cursor to the end of file before writing");
                        }
                }
                let mut wlen = 0;
                let mut blen = buf.len() as u32;
                let mut wbuf = buf;
                while blen > 0 {
                        if let Ok(cluster) = self.get_cur_cluster() {
                                let off = self.cursor % *CLUSTER_SIZE;
                                let write_len = write_cluster(cluster, off, wbuf).unwrap();
                                wbuf = &wbuf[(write_len as usize)..];
                                blen -= write_len;
                                self.cursor += write_len;
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

                if self.fsize < self.cursor {
                        self.fsize = self.cursor;
                }
                return Ok(wlen);
        }

        pub fn close_file(&mut self) -> Result<(), (&FILE, &'static str)> {
                if self.ftype != FTYPE::TFile {
                        return Err((self, "FILE::close_file: not a file"));
                }
                if self.do_create() {
                        if let Ok(mut entry) = find_entry(&self.path, false) {
                                if self.write_allowed() || self.append_allowed() {
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
                                                        clear_file_chain(self.fchain[0]).unwrap();
                                                } else {
                                                        truncat_file_chain(self.fchain[chain_len as usize - 1]).unwrap();
                                                }
                                        }
                                        update_entry(&self.path, false, &entry).unwrap();
                                        flush();
                                } 
                        } else {
                                let file = self.path.pop().unwrap();
                                let mut name = [0u8;8];
                                name.clone_from_slice(get_name(&file).as_bytes());
                                let mut ext = [0u8;3];
                                ext.clone_from_slice(get_ext(&file).as_bytes());
                                let start_h = (self.fchain[0] >> 16) as u16;
                                let start_l = (self.fchain[0] & 0xff) as u16;
                                let entry = DirEntry{
                                        name, ext,
                                        attr: DirEntry::attr_file(), reserved: 0x0,
                                        created_minisec: 0x0, created_sec: 0x0,
                                        created_date: 0x0, accessed_sec: 0x0,
                                        start_h, mod_sec: 0x0,
                                        mod_date: 0x0, start_l, size: self.fsize,
                                };
                                new_entry(&self.path, &entry).unwrap();
                                flush();
                        }
                } else {
                        if self.write_allowed() || self.append_allowed() {
                                if let Ok(mut entry) = find_entry(&self.path, false) {
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
                                                        clear_file_chain(self.fchain[0]).unwrap();
                                                } else {
                                                        truncat_file_chain(self.fchain[chain_len as usize - 1]).unwrap();
                                                }
                                        }
                                        update_entry(&self.path, false, &entry).unwrap();
                                        flush();
                                } else {
                                        return Err((self, "file_close: file not exist, what up?"));
                                }
                        }
                }
                return Ok(());
        }
}
