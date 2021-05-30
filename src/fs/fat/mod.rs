/*
        FAT structure on disk:

        ||<------------------------------------------------------------Disk-------------------------------------------------------->||
        ||----------------------------Reserved--------------------------------------|-----FAT-----|---------------DATA--------------||               
        || MBR(optional) | Blank (optional) | DBR(512) | FSINFO(512) | Blank/Backup | FAT1 | FAT2 | Root Dir |   other file data    || 
*/

use core::mem::size_of;
use alloc::vec::Vec;
use alloc::string::String;
use lazy_static::*;

use super::block_cache::get_block_cache;
use super::block_cache::flush_all;
use super::dirent::DirEntry;
use super::dirent::DirEntryExt;
use super::dirent::DIRENT_P_CLST;
use super::path::Path;
use alloc::borrow::ToOwned;

mod dbr;
pub mod fsinfo;
pub mod fat;

use dbr::DBR_INST;
use dbr::get_cluster_cache;
use fat::get_file_chain;

lazy_static! {
        pub static ref CLUSTER_SIZE: u32 = DBR_INST.clst_size;
        pub static ref ROOT_DIR: u32 = DBR_INST.root;
        
        static ref CLUSTER_CNT: u32 = DBR_INST.clst_cnt; 
        // static ref ROOT_DIR: Vec<u32> = fat::FAT_INST.get_clusters(DBR_INST.root);
}


#[allow(unused)]
pub fn print_vec(vec : Vec<u32>) {
        for i in vec.iter() {
                print!("{} ", i);
        }
        println!();
}

// DBR functions
#[inline]
#[allow(unused)]
pub fn print_dbr() {
        dbr::print_dbr();
}

#[inline]
pub fn read_cluster(cluster: u32, offset: u32, buf: &mut [u8]) ->Result<u32,&str> {
        dbr::read_cluster(cluster, offset, buf)
}

#[inline]
pub fn write_cluster(cluster: u32, offset: u32, buf: &[u8]) -> Result<u32, &str> {
        dbr::write_cluster(cluster, offset, buf)
}

// FAT functions
#[inline]
pub fn append_chain(end: u32) -> Result<u32, &'static str> {
        fat::append_chain(end)
}

#[inline]
pub fn truncat_chain(start: u32) -> Result<(), ()> {
        fat::truncat_file_chain(start)
}

pub fn read_chain(chain: &Vec<u32>, offset: u32, buf: &mut [u8]) -> Result<u32, &'static str> {
        let mut chain_i = (offset / *CLUSTER_SIZE) as usize;
        if chain_i >= chain.len(){
                return Err("read_chain: invalid offset");
        }

        let offset = offset % *CLUSTER_SIZE;
        let len = buf.len(); 
        let mut buf = buf;
        let mut read: usize = 0;
        while read < len {
                let r = read_cluster(chain[chain_i], offset, buf).unwrap() as usize;
                read += r;
                buf = &mut buf[r..];
                chain_i += 1;
                if chain_i >= chain.len() {
                        return Ok(read as u32);
                }
        }
        return Err("error");
}

pub fn write_chain(chain: &Vec<u32>, offset: u32, buf: &mut [u8]) -> Result<u32, &'static str> {
        let mut chain_i = (offset / *CLUSTER_SIZE) as usize;
        if chain_i >= chain.len(){
                return Err("read_chain_c: invalid offset");
        }

        let offset = offset % *CLUSTER_SIZE;
        let len = buf.len(); 
        let mut buf = buf;
        let mut read: usize = 0;
        while read < len {
                let r = write_cluster(chain[chain_i], offset, buf).unwrap() as usize;
                read += r;
                buf = &mut buf[r..];
                chain_i += 1;
                if chain_i >= chain.len() {
                        return Ok(read as u32);
                }
        }
        return Err("error");
}

pub fn read_dirent(chain: &Vec<u32>, offset: u32) -> Option<DirEntry> {
        let chain_i = offset / *DIRENT_P_CLST;
        if chain_i >= chain.len() as u32 {
                return None;
        }
        let off = offset % *DIRENT_P_CLST;
        return read_dirent_c(chain[chain_i as usize], off);
}

pub fn read_dirent_lfn(chain: &Vec<u32>, offset: u32) -> Result<Option<(DirEntry, usize, String)>,()> {
        let mut direntext = Vec::<DirEntryExt>::new();
        let mut do_push = false;
        let mut offset = offset;
        loop {
                if let Some(item) = read_dirent(chain, offset) {
                        if item.deleted() || item.is_vol() {
                                return Ok(None);
                        } 
                        if item.is_ext() {
                                unsafe {
                                        let dex = *((&item as *const _) as *const DirEntryExt);
                                        if dex.is_end() {
                                                direntext.push(*((&item as *const _) as *const DirEntryExt));
                                                do_push = true;
                                        } else if do_push {
                                                direntext.push(*((&item as *const _) as *const DirEntryExt));
                                        } else {
                                                return Ok(None);
                                        }
                                }
                                offset += 1;
                                continue;
                        }
                        if item.is_dir() || item.is_file() {
                                if direntext.len() > 0 {
                                        let cnt = direntext.len() + 1;
                                        let name = get_full_name(&mut direntext).unwrap();
                                        return Ok(Some((item, cnt, name)));
                                } else {
                                        return Ok(Some((item, 1, item.get_name())));
                                }
                        }
                        // debug!("read_dirent_lfn: I get some weird stuff:{} {}",chain[0], offset);
                        item.print_raw();
                        offset += 1;
                } else {
                        return Err(());
                }
        }
}

pub fn read_dirent_c(cluster: u32, offset: u32) -> Option<DirEntry> {
        if cluster > *CLUSTER_CNT {
                return None;
        }
        if let Some(block) = get_cluster_cache(cluster, offset * size_of::<DirEntry>() as u32) {
                let off = offset * size_of::<DirEntry>() as u32 % DBR_INST.sec_len;
                let dirent = *get_block_cache(block as usize).lock().get_ref::<DirEntry>(off as usize);
                if dirent.name[0] == 0 {
                        None
                } else {
                        Some(dirent)
                }
        } else {
                return None
        }
}

pub fn write_dirent(chain: &Vec<u32>, offset: u32, new: &DirEntry) -> Result<(), &'static str> {
        let chain_i = offset / *DIRENT_P_CLST;
        if chain_i >= chain.len() as u32 {
                return Err("write_dirent: invalid offset");
        }
        let off = offset % *DIRENT_P_CLST;
        return write_dirent_c(chain[chain_i as usize], off, new);
}

pub fn write_dirent_c(cluster:u32, offset: u32, new: &DirEntry) -> Result<(), &'static str> {
        if cluster > *CLUSTER_CNT {
                return Err("write_dirent: invalid cluster");
        }
        if let Some(block) = get_cluster_cache(cluster, offset * size_of::<DirEntry>() as u32) {
                let off = offset * size_of::<DirEntry>() as u32 % DBR_INST.sec_len;
                // println!("b:{:#010X} off:{:#010X}", block, off);
                *get_block_cache(block as usize).lock().get_mut::<DirEntry>(off as usize) = *new;
                return Ok(());       
        } else {
                return Err("write_dirent: invalid offset");
        }
}

pub fn delete_dirent(chain: &Vec<u32>, offset: u32) -> Result<(), &'static str> {
        let chain_i = offset / *DIRENT_P_CLST;
        if chain_i >= chain.len() as u32 {
                return Err("delete_dirent: invalid offset");
        }
        let off = offset % *DIRENT_P_CLST;
        return delete_dirent_c(chain[chain_i as usize], off);
}

pub fn delete_dirent_c(cluster: u32, offset: u32) -> Result<(), &'static str> {
        if cluster > *CLUSTER_CNT {
                return Err("delete_dirent_c: invalid cluster");
        }
        if let Some(block) = get_cluster_cache(cluster, offset * size_of::<DirEntry>() as u32) {
                let off = offset * size_of::<DirEntry>() as u32 % (*DBR_INST).sec_len;
                // println!("b:{:#010X} off:{:#010X}", block, off);
                get_block_cache(block as usize).lock().get_mut::<DirEntry>(off as usize).name[0] = 0xE5;
                return Ok(());
        } else {
                return Err("delete_dirent_c: get_cluster_cache failed, invalid offset ?");
        }
}

pub fn find_entry(path: &Path) -> Result<DirEntry, &'static str> {
        find_entry_from(*ROOT_DIR, path)
}

fn get_full_name(dex: &mut Vec::<DirEntryExt>) -> Result<String, &'static str> {
        let mut name = Vec::<u8>::new();
        loop {
                if let Some(d) = dex.pop() {
                        name.append(&mut d.get_name());
                        let nlen = name.len() >> 1;
                        if d.is_end() {
                                let mut n = Vec::<u16>::with_capacity(nlen);
                                for i in 0..nlen {
                                        let tmp:u16 = name[2*i+1] as u16;
                                        let tmp:u16 = (tmp << 8) | (name[2*i] as u16);
                                        if tmp == 0 {
                                                break;
                                        }
                                        n.push(tmp);
                                }
                                let name = String::from_utf16(&n).unwrap();
                                return Ok(name);
                        }
                } else {
                        return Err("get_full_name: missing end for lfn");
                }
        }
}

pub fn find_entry_from(from: u32, path: &Path) -> Result<DirEntry, &'static str> {
        // debug!("find_entry_from:{} {}", from, path.path[0]);
        if path.path.len() == 0 {
                if path.is_abs {
                        return Err("find_entry_from: no entry for root directory");
                } else {
                        return Err("find_entry_from: current entry?");
                }
        }
        let mut dir = get_file_chain(from);
        if dir.len() == 0 {
                return Err("find_entry_from: invalid file chain");
        }
        let mut dirent: Option<DirEntry> = None;
        let mut lname: Option<String> = None;
        let mut direntext = Vec::<DirEntryExt>::new();
        let mut depth = 0;
        for fname in &path.path {
                let mut i = 0;
                let fname = fname.to_ascii_uppercase();
                // debug!("find_entry_from:{} {}", fname, dir[0]);
                loop {
                        match read_dirent_lfn(&dir, i) {
                                Ok(Some((item, cnt, name))) => {
                                        verbose!("Checking: {} <=> {}", name.to_ascii_uppercase(), fname);
                                        if name.to_ascii_uppercase() == fname {
                                                dirent = Some(item);
                                                lname = Some(name);
                                                depth += 1;
                                                dir = if item.is_dir() {item.get_chain()} else {Vec::new()};
                                                break;       
                                        }
                                        i += cnt as u32;
                                },
                                Ok(None) => {
                                        i += 1;
                                },
                                Err(_) => {
                                        // panic!("PANICCCCC");
                                        return Err("find_entry_from: file not found");
                                }
                        };
                }
        }
        if depth == path.path.len() {
                if let Some(de) = dirent {
                        if path.must_dir && !de.is_dir() {
                                return Err("find_entry_from: not a directory");
                        }
                        return Ok(de);
                } else {
                        return Err("find_entry_from: file not found");
                }
        } else {
                return Err("find_entry_from: file not found");
        }
}

pub fn delete_entry(path: &Path, is_dir: bool) -> Result<(),&'static str> {
        if path.path.len() == 0 {
                return Err("delete_entry: no entry for root directory");
        }
        let mut parent_path = path.clone();
        let file = parent_path.path.pop().unwrap();
        parent_path.must_dir = true;
        let chain = if parent_path.path.len() == 0 {
                get_file_chain(*ROOT_DIR)
        } else {
                let entry = find_entry(&parent_path).unwrap();
                get_file_chain(entry.get_start())
        };
        let mut dex = Vec::<DirEntryExt>::new();
        let mut offset = 0;
        loop {
                if let Some(dirent) = read_dirent(&chain, offset) {
                        if dirent.deleted() {
                                continue;
                        }
                        if dirent.is_ext() {
                                unsafe {
                                        dex.push(*((&dirent as *const _) as *const DirEntryExt));
                                }
                                continue;
                        }
                        if dex.len() != 0 {
                                let flen = dex.len() as u32 + 1;
                                if file == get_full_name(&mut dex).unwrap() {
                                        if path.must_dir && !dirent.is_dir() {
                                                return Err("delete entry: not a directory");
                                        }
                                        for i in 0..flen {
                                                delete_dirent(&chain, offset + i).unwrap();
                                        }
                                }
                        } else {
                                if file == dirent.get_name() {
                                        if is_dir && !dirent.is_dir() {
                                                return Err("delete_entry: not a directory");
                                        }
                                        delete_dirent(&chain, offset).unwrap();
                                        return Ok(());
                                }
                        }
                } else {
                        return Err("delete_entry: entry not found");
                }
                offset += 1;
        }
}

fn is_free_entry(chain: &Vec<u32>, offset: u32) -> bool {
        let entry = read_dirent(chain, offset).unwrap();
        return entry.deleted() || entry.name[0] == 0;
}

pub fn update_entry(path: &Path, new: &DirEntry) -> Result<(), &'static str> {
        if path.path.len() == 0 {
                return Err("update_entry: no entry for root directory");
        }
        let mut parent_path = path.clone();
        let file = parent_path.path.pop().unwrap().to_ascii_uppercase();
        let fisdir = parent_path.must_dir;
        parent_path.must_dir = true;
        let chain = if parent_path.path.len() == 0 {
                get_file_chain(*ROOT_DIR)
        } else {
                let entry = find_entry(&parent_path).unwrap();
                get_file_chain(entry.get_start())
        };
        let mut offset = 0;
        let mut dex = Vec::<DirEntryExt>::new();
        loop {
                match read_dirent_lfn(&chain, offset) {
                        Ok(Some((dirent, cnt, name))) => {
                                if name.to_ascii_uppercase() == file {
                                        if fisdir && !dirent.is_dir() {
                                                return Err("delete_entry: not a directory");
                                        }
                                        offset += cnt as u32;
                                        offset -= 1;
                                        write_dirent(&chain, offset, new).unwrap();
                                        return Ok(());
                                } 
                                offset += cnt as u32;
                        },
                        Ok(None) => {
                                offset += 1;
                        },
                        Err(_) => {
                                return Err("update_entry: entry not found");
                        }
                }
        }
}

// #[inline]
// //                                  (clst, offset, update_size)
// fn get_free_entry(chain: &Vec<u32>) -> (u32, u32, bool) {
//         for clst in chain {
//                 for offset in 0..*DIRENT_P_CLST {
//                         if let Some(dirent) = read_dirent_c(*clst, offset) {
//                                 if dirent.deleted() {
//                                         return (*clst, offset, false);
//                                 }
//                         } else {
//                                 return (*clst, offset, true);
//                         }
//                 }
//         }
//         if let Ok(clst) = append_chain(chain[chain.len() - 1]) {
//                 return (clst, 0, true);
//         }
//         return (0,0, false);
// }
fn get_free_entry(chain: &Vec<u32>) -> u32 {
        let mut offset = 0;
        loop {
                if let None = read_dirent(chain, offset) {
                        return offset;
                }
                offset += 1;
        }
        
}

pub fn new_entry(parent: &Path, new: &DirEntry, name: &String) -> Result<(), &'static str> {
        // debug!("new_entry: name {}", name);
        let mut fchain = get_file_chain(*ROOT_DIR);
        let mut entry: Option<DirEntry> = None;
        let mut parent = parent.clone();
        parent.must_dir = true;
        if parent.path.len() != 0 {
                if let Ok(ent) = find_entry(&parent) {
                        entry = Some(ent);
                        fchain = ent.get_chain();
                } else {
                        return Err("new_entry: parent not found");
                }
        }

        let mut offset = get_free_entry(&fchain);
        let mut dex = DirEntryExt::new(name, new.chksum());
        let size = size_of::<DirEntryExt>() * dex.len() + size_of::<DirEntry>();
        loop {
                if let Some(de) = dex.pop() {
                        unsafe {
                                let d = *((&de as *const _) as *const DirEntry);
                                // debug!("writing to {}", offset);
                                if let Err(_) = write_dirent(&fchain, offset, &d) {
                                        append_chain(fchain[fchain.len() -1]).unwrap();
                                        write_dirent(&fchain, offset, &d).unwrap();
                                }

                                offset += 1;
                        }
                } else {
                        break;
                }
        }
        
        if parent.path.len() != 0{
                if let Some(mut entry) = entry {
                        entry.size += size as u32;
                        update_entry(&parent, &entry).unwrap();
                } else {
                        return Err("new_entry: what happened to my entry?");
                }
        }
        // debug!("writing short to {}", offset);
        if let Err(_) = write_dirent(&fchain, offset, new) {
                append_chain(fchain[fchain.len() -1]).unwrap();
                write_dirent(&fchain, offset, new).unwrap();
        }
        return Ok(());
}

pub fn new_entry_at(parent: &DirEntry, new: &DirEntry, name: &String) -> Result<u32, &'static str> {
        // debug!("new_entry_at: name {} {}",new.get_name(), name);
        let fchain = get_file_chain(parent.get_start());
        let mut offset = get_free_entry(&fchain);
        let mut size = 0;
        if name.len() != 0 {
                let mut dex = DirEntryExt::new(name, new.chksum());
                size += size_of::<DirEntryExt>() * dex.len();
                loop {
                        if let Some(de) = dex.pop() {
                                unsafe {
                                        let d = *((&de as *const _) as *const DirEntry);
                                        if let Err(_) = write_dirent(&fchain, offset, &d) {
                                                append_chain(fchain[fchain.len() -1]).unwrap();
                                                write_dirent(&fchain, offset, &d).unwrap();
                                        }
                                        // debug!("new_entry_at: writed ext at {} {}", fchain[0], offset);
                                        offset += 1;
                                }
                        } else {
                                break;
                        }
                }
        }

        size += size_of::<DirEntry>();
        if let Err(_) = write_dirent(&fchain, offset, new) {
                append_chain(fchain[fchain.len() -1]).unwrap();
                write_dirent(&fchain, offset, new).unwrap();
        }
        // debug!("new_entry_at: writed at {} {}", fchain[0], offset);
        return Ok(size as u32);
}

#[allow(unused)]
pub fn ls_root() {
        let chain = get_file_chain(*ROOT_DIR);
        let mut offset = 0;
        let mut dex = Vec::<DirEntryExt>::new();
        loop{
                if let Some(dirent) = read_dirent(&chain, offset) {
                        if dirent.deleted() || dirent.is_vol() {
                                continue;
                        } 
                        if dirent.is_ext() { 
                                unsafe{
                                        let dirent = *((&dirent as *const _) as *const DirEntryExt);
                                        dex.push(dirent);
                                }
                        }
                        if dirent.is_dir() || dirent.is_file() {
                                println!("{}", get_full_name(&mut dex).unwrap());
                        }
                        // print!("ls_root:{} {} \t", cluster, offset - 1);
                        dirent.print();
                } else {
                        break;
                }
        }
}

pub fn flush() {
        flush_all();
}