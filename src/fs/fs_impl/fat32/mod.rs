mod dbr;
mod fat;
mod chain;
mod dirent;
pub mod inode;
pub mod file;
pub mod wrapper;

use dbr::DBR;
use dbr::RAW_DBR;
use fat::FAT;
use fat::CLUSTER;
use dirent::DirEntryRaw;
use inode::Inode;
use file::FileInner;

use core::cell::RefCell;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use super::cache_mgr::BlockCacheManager;
use super::cache_mgr::BLOCK_SZ;

use super::BlockDeviceFile;

use core::mem::size_of;


struct Fat32FSInner {
        mgr: BlockCacheManager,
}

pub struct Fat32FS {
        inner: RefCell<Fat32FSInner>,
        dbr: DBR,
        fat1: FAT,
        fat2: FAT,
        de_p_clst: usize,
}

unsafe impl Sync for Fat32FS {}

fn get_fat(dbr: &DBR, which: usize) -> FAT {
        let block_id = match which {
                1 => dbr.rsv_sec as u32,
                2 => dbr.rsv_sec as u32 + dbr.fat_sec,
                _ => panic!("Invalid fat #"),
        };
        let clen  = dbr.sec_len / size_of::<u32>() as u32;
        let fat_len = dbr.fat_len / size_of::<u32>() as u32;
        return FAT{ 
                start: block_id, 
                end: block_id + dbr.fat_sec, 
                len: fat_len,
                clen,
        };
}

impl Fat32FS {
        pub const name: &'static str = "Fat32FS (Powered by OSHIT)";

        pub fn openFat32(device: Arc<Mutex<dyn BlockDeviceFile>>) -> Fat32FS {
                let mut mgr = BlockCacheManager::new(device);
                let raw_dbr = mgr.get_block_cache(0).lock().get_ref::<RAW_DBR>(0).clone();
                if raw_dbr.sign[0] != 0x55 || raw_dbr.sign[1] != 0xAA {
                        panic!("get_dbr: Invalid dbr");
                }
                let dbr = DBR::from_raw(raw_dbr, 0);
                dbr.print();
                let fat1 = get_fat(&dbr, 1);
                let fat2 = get_fat(&dbr, 2);
                let de_p_clst = dbr.clst_size as usize / size_of::<DirEntryRaw>();
                let inner = RefCell::new(Fat32FSInner { mgr });
                Fat32FS {inner, dbr, fat1, fat2, de_p_clst}
        }

        pub fn cluster_size(&self) -> usize {
                return self.dbr.clst_size as usize;
        }

        pub fn get_cluster_cache(&self, cluster: u32, offset: usize) -> Option<u32> {
                if cluster < self.dbr.root {
                        return None;
                }
                let cluster = cluster - self.dbr.root;
                if cluster > self.dbr.clst_cnt || offset as u32 > self.dbr.clst_size {
                        return None;
                }
                let mut sector: u32 = self.dbr.data_sec_base + self.dbr.clst_sec * cluster;
                sector += offset as u32 / self.dbr.sec_len;
                return Some(sector);
        }

        pub fn read_cluster(&self, cluster: u32, offset: usize, buf: &mut [u8]) ->Result<usize, &'static str> {
                if cluster >= self.dbr.clst_cnt {
                        return Err("read_cluster: Invalid cluster");
                }
                if offset as u32 >= self.dbr.clst_size {
                        return Err("read_cluster: Invalid Offset");
                }
                
                let mut len = buf.len();
                let mut read = 0;
                let mut offset = offset;
                while len > 0 {
                        let block = self.get_cluster_cache(cluster, offset).unwrap();
                        let off = offset as usize % BLOCK_SZ;
                        let cache = self.inner.borrow_mut().mgr.get_block_cache(block as usize);
                        let rlen = BLOCK_SZ - (offset % BLOCK_SZ);
                        let rlen = if rlen > len {len} else {rlen};
                        for i in 0..rlen as usize {
                                buf[read as usize + i] = *cache.lock().get_ref::<u8>(off + i);
                        }
                        len -= rlen as usize;
                        offset += rlen;
                        read += rlen;
                        if offset as u32 >= self.dbr.clst_size {
                                return Ok(read);
                        } 
                }
                return Ok(buf.len());
        }

        pub fn write_cluster(&self, cluster: u32, offset: usize, buf: &[u8]) -> Result<usize, &'static str> {
                if cluster >= self.dbr.clst_cnt {
                        return Err("write_cluster: Invalid cluster");
                }
                if offset as u32 >= self.dbr.clst_size {
                        return Err("write_cluster: Invalid Offset");
                }
        
                let mut len = buf.len();
                let mut write = 0;
                let mut offset = offset;
                while len > 0 {
                        let block = self.get_cluster_cache(cluster, offset).unwrap();
                        let off = offset as usize % BLOCK_SZ;
                        let cache = self.inner.borrow_mut().mgr.get_block_cache(block as usize).clone();
                        let wlen = BLOCK_SZ - (offset % BLOCK_SZ);
                        let wlen = if wlen > len {len} else {wlen};
                        for i in 0..wlen as usize {
                                *cache.lock().get_mut::<u8>(off + i) = buf[write as usize + i];
                        }
                        len -= wlen as usize;
                        offset += wlen;
                        write += wlen;
                        if offset as u32 >= self.dbr.clst_size {
                                return Ok(write);
                        } 
                }
                return Ok(buf.len());
        }

        pub fn clear_cluster(&self, cluster:u32) -> Result<(), &'static str> {
                if cluster >= self.dbr.clst_cnt {
                        return Err("clear_cluster: Invalid cluster");
                } 
                if let Some(block) = self.get_cluster_cache(cluster, 0) {
                        for i in 0..(self.dbr.clst_size / BLOCK_SZ as u32) {
                                self.inner.borrow_mut().mgr.clear_block_cache((block+i) as usize);
                        }
                }
                return Ok(());
        }

        fn get_next_clst(&self, clst_num: u32) -> Option<u32> {
                if clst_num >= self.fat1.len {
                        return None;
                } 
                let block_id = clst_num / self.fat1.clen + self.fat1.start;
                let offset = clst_num % self.fat1.clen * size_of::<u32>() as u32;
                // debug!("get_next: getting block cache");
                let next = *self.inner.borrow_mut().mgr.get_block_cache(block_id as usize).lock().get_ref::<u32>(offset as usize);
                Some(next)
        }

        fn write_next_clst(&self, clst_num: u32, next: u32) -> Result<(),()> {
                if clst_num >= self.fat1.len {
                        return Err(());
                }
                let block_id = clst_num / self.fat1.clen + self.fat1.start;
                let offset = clst_num % self.fat1.clen * size_of::<u32>() as u32;
                *self.inner.borrow_mut().mgr.get_block_cache(block_id as usize).lock().get_mut::<u32>(offset as usize) = next;
                let block_id = block_id + self.dbr.fat_sec;
                *self.inner.borrow_mut().mgr.get_block_cache(block_id as usize).lock().get_mut::<u32>(offset as usize) = next;
                return Ok(());
        }

        pub fn alloc_cluster(&self) -> Result<u32, &'static str> {
                let mut new = 0;
                for i in 2..self.dbr.clst_cnt {
                        if fat::get_type(self.get_next_clst(i).unwrap()) == CLUSTER::Free {
                                new = i;
                                break;
                        }
                }
                if new != 0 {
                        self.write_next_clst(new, 0x0FFF_FFFF).unwrap();
                        self.clear_cluster(new).unwrap();
                        return Ok(new);
                } else {
                        return Err("get_free_cluster: no free cluster found");
                }
        }

        pub fn get_chain(&self, start: u32) -> Vec<u32> {
                let mut vec = Vec::new();
                if start < 2 {
                        return vec;
                }
                let mut cluster = start;
                let mut t = fat::get_type(self.get_next_clst(cluster).unwrap());
                while match t {
                        CLUSTER::Data => {
                                vec.push(cluster);
                                cluster = self.get_next_clst(cluster).unwrap();
                                true
                        },
                        CLUSTER::Eoc => {
                                vec.push(cluster);
                                false
                        }
                        _ => {
                                // debug!("{:?}", t);
                                false
                        }
                } { 
                        t = fat::get_type(self.get_next_clst(cluster).unwrap()) 
                }
                return vec
        }

        pub fn clear_chain(&self, start: u32) -> Result<(),()> {
                if start == 0 {
                        return Ok(());
                }
                let mut cur = start;
                loop {
                        let next = self.get_next_clst(cur).unwrap();
                        match fat::get_type(next) {
                                CLUSTER::Data => {
                                        self.write_next_clst(cur,0).unwrap();
                                        cur = next;
                                },
                                CLUSTER::Eoc => {
                                        self.write_next_clst(cur, 0).unwrap();
                                        return Ok(());
                                }
                                _ => {
                                        panic!("clean_file_chain: ?");
                                }
                        }
                }
        }

        pub fn append_chain(&self, end: u32) -> Result<u32, &str> {
                let end = match fat::get_type(self.get_next_clst(end).unwrap()) {
                        CLUSTER::Eoc => end,
                        CLUSTER::Data => self.get_chain(end).pop().unwrap(),
                        _ => return Err("append_cluster: not a chain\n"),
                };

                if let Ok(new) = self.alloc_cluster() {
                        self.write_next_clst(end, new).unwrap();
                        return Ok(new);
                } else {
                        return Err("append_cluster: cannot find free cluster");
                }
        }

        pub fn truncate_chain(&self, start: u32) -> Result<(), ()> {
                match self.clear_chain(start) {
                        Ok(()) => {
                                self.write_next_clst(start, 0x0FFF_FFFF).unwrap();
                                return Ok(());
                        },
                        _ => {
                                return Err(());
                        }
                }
        }

        pub fn sync(&self) {
                self.inner.borrow_mut().mgr.flush_all();
        }
}

fn root_dir(fs: Arc<Fat32FS>) -> FileInner {
        return FileInner::new(Inode::root(fs), 0); 
}

pub fn open(fs: Arc<Fat32FS>, abs_path: &str, mode: usize) -> Result<FileInner, &'static str> {
        let mut root = root_dir(fs);
        return root.open(&abs_path, mode);
}

pub fn mkdir(fs: Arc<Fat32FS>, abs_path: &str) -> Result<FileInner, &'static str> {
        let mut root = root_dir(fs);
        return root.mkdir(abs_path);
}

pub fn mkfile(fs: Arc<Fat32FS>, abs_path: &str) -> Result<FileInner, &'static str> {
        let mut root = root_dir(fs);
        return root.mkfile(abs_path);
}

pub fn remove(fs: Arc<Fat32FS>, abs_path: &str) -> Result<(), &'static str> {
        let mut root = root_dir(fs);
        return root.remove(abs_path);
}

pub fn rename(fs: Arc<Fat32FS>, to_rename: &str, new_name: &str) -> Result<(), &'static str> {
        match open(fs, to_rename, 0){
                Ok(mut file) => {
                        file.rename(new_name).unwrap();
                        file.close();
                        return Ok(());
                },
                Err(_) => {
                        return Err("rename: file not found");
                }
        };
}

pub fn sym_link(fs: Arc<Fat32FS>, target_path: &str, link_path: &str) -> Result<(), &'static str> {
        match open(fs, link_path, file::WRITE | file::CREATE | file::NO_FOLLOW) {
                Ok(mut file) => {
                        file.set_attr(DirEntryRaw::ATTR_SYM);
                        file.write(target_path.as_bytes()).unwrap();
                        file.close();
                        return Ok(());
                },
                Err(msg) => {
                        return Err(msg);
                }
        }
}

pub fn print_file_tree(root: &Inode, indent: usize) {
        if root.is_dir() {
                let mut indent_s = String::new();
                for _i in 0..indent {
                        indent_s += "    ";
                }
                for inode in root.get_inodes().unwrap() {
                        print!("{}", indent_s);
                        inode.print();
                        if inode.is_dir() && !inode.is_cur() && !inode.is_par() {
                                print_file_tree(&inode, indent + 1);
                        }
                }
        }
}

