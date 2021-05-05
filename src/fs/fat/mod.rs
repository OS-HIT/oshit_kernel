/*
        FAT structure on disk:

        ||<------------------------------------------------------------Disk-------------------------------------------------------->||
        ||----------------------------Reserved--------------------------------------|-----FAT-----|---------------DATA--------------||               
        || MBR(optional) | Blank (optional) | DBR(512) | FSINFO(512) | Blank/Backup | FAT1 | FAT2 | Root Dir |   other file data    || 
*/

use core::mem::size_of;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use lazy_static::*;

use super::block_cache::get_block_cache;
use super::block_cache::flush_all;
use super::dirent::DirEntry;

mod dbr;
pub mod fsinfo;
pub mod fat;

use dbr::DBR_INST;
use dbr::get_cluster_cache;

lazy_static! {
        pub static ref CLUSTER_SIZE: u32 = DBR_INST.cluster_size();
        
        static ref CLUSTER_CNT: u32 = DBR_INST.cluster_cnt(); 
        static ref ROOT_DIR: Vec<u32> = fat::FAT_INST.get_clusters(DBR_INST.root);
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

pub fn read_dirent(cluster: u32, offset: u32) -> Option<DirEntry> {
        if cluster > *CLUSTER_CNT {
                return None;
        }
        if let Some(block) = get_cluster_cache(cluster, offset * size_of::<DirEntry>() as u32) {
                let off = offset * size_of::<DirEntry>() as u32 % DBR_INST.sec_len as u32;
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

pub fn write_dirent(cluster:u32, offset: u32, new: &DirEntry) -> Result<(), &str> {
        if cluster > *CLUSTER_CNT {
                return Err("write_dirent: invalid cluster");
        }
        if let Some(block) = get_cluster_cache(cluster, offset * size_of::<DirEntry>() as u32) {
                let off = offset * size_of::<DirEntry>() as u32 % DBR_INST.sec_len as u32;
                // println!("b:{:#010X} off:{:#010X}", block, off);
                *get_block_cache(block as usize).lock().get_mut::<DirEntry>(off as usize) = *new;
                return Ok(());       
        } else {
                return Err("write_dirent: invalid offset");
        }
}

pub fn delete_dirent(cluster: u32, offset: u32) -> Result<(), &'static str> {
        if cluster > *CLUSTER_CNT {
                return Err("delete_dirent: invalid cluster");
        }
        if let Some(block) = get_cluster_cache(cluster, offset * size_of::<DirEntry>() as u32) {
                let off = offset * size_of::<DirEntry>() as u32 % (*DBR_INST).sec_len as u32;
                // println!("b:{:#010X} off:{:#010X}", block, off);
                get_block_cache(block as usize).lock().get_mut::<DirEntry>(off as usize).name[0] = 0xE5;
                return Ok(());
        } else {
                return Err("delete_dirent: get_cluster_cache failed, invalid offset ?");
        }
}

fn get_fname(path: &str) -> String {
        let mut len = path.len();
        if len <= 1 {
                return "".to_string();
        }
        if path.chars().nth(0).unwrap() != '/' {
                return "".to_string();
        }
        for (i, c) in path.chars().enumerate() {
                if i == 0 {
                        continue;
                } 
                if c == '/' {
                        len = i;
                        break;
                }
        }
        return path.chars().skip(1).take(len - 1).collect();
}

pub fn find_entry(path: &str) -> Result<DirEntry, &str> {
        let mut dir:Vec<u32> = ROOT_DIR.to_vec();
        let mut path = path.trim();
        let p_end = path.len() - 1;
        if path.chars().nth(0).unwrap() != '/' {
                return Err("find_entry: absolute path required");
        }
        if p_end == 0 {
                return Err("find_entry: no entry for root directory");
        }
        let is_dir = if path.chars().nth(p_end).unwrap() == '/' {
                path = &path[..p_end-1];
                true
        } else {
                false
        };
        loop {
                // println!("path: {}", path);
                let fname = get_fname(path);
                let mut start = 0;
                if fname.len() == 0 {
                        break Err("File not find\n");
                }
                for clst in dir.iter() {
                        let mut i = 0;
                        loop {
                                if let Some(item) = read_dirent(*clst, i) {
                                        i += 1;
                                        if item.deleted() || item.is_ext() {
                                                continue;
                                        }
                                        // item.print();
                                        if fname.to_uppercase() == item.get_name() {
                                                if path[1..] == fname {
                                                        if !is_dir || is_dir != item.is_dir() {
                                                                return Ok(item);
                                                        } else {
                                                                return Err("find_entry: not a directory");
                                                        }
                                                } else {
                                                        path = &path[fname.len()+1..];
                                                        start = item.get_start();
                                                        // println!("Getting round two");
                                                        break;
                                                }
                                                
                                        }
                                } else {
                                        break;
                                }
                        }
                        if start != 0 {
                                break;
                        }
                }
                if start != 0{
                        dir = fat::get_file_chain(start);
                } else {
                        break Err("File not find\n");
                }
        }
}

fn get_lname(path: &str) -> String {
        if path.len() <= 1 {
                return "".to_string();
        }
        if path.chars().nth(0).unwrap() != '/' {
                return "".to_string();
        }
        let mut len = 1;
        for (i, c) in path.chars().enumerate() {
                if c == '/' {
                        len = i + 1;
                }
        }
        return path.chars().skip(len).take(path.len() - len).collect();
}

pub fn delete_entry(path: &str) -> Result<(),&str> {
        let mut path = path.trim();
        let p_end = path.len() - 1;
        if path.chars().nth(0).unwrap() != '/' {
                return Err("delete_entry: absolute path required");
        }
        if p_end == 0 {
                return Err("delete_entry: no entry for root directory");
        }
        if path.chars().nth(p_end).unwrap() == '/' {
                path = &path[..p_end-1];
        }
        let lname = get_lname(path);
        if lname.len() == 0 {
                return Err("delete_entry: invalid path");
        }
        let chain = if lname.len() < p_end {
                let plen = p_end - lname.len();
                path = &path[..plen];
                let parent = find_entry(path).unwrap();
                fat::get_file_chain(parent.get_start())
        } else {
                ROOT_DIR.to_vec()
        };
        for clst in chain {
                let mut offset = 0;
                loop {
                        if let Some(dirent) = read_dirent(clst, offset) {
                                offset += 1;
                                if dirent.is_ext() || dirent.deleted() {
                                        continue;
                                }
                                if lname.to_uppercase() == dirent.get_name() {
                                        delete_dirent(clst,offset).unwrap();
                                        return Ok(());
                                }
                        } else {
                                break;
                        }
                }
        }
        return Err("delete_entry: entry not found");
}

pub fn update_entry(path: &str, new: &DirEntry) -> Result<(), &'static str> {
        let mut path = path.trim();
        let p_end = path.len() - 1;
        if path.chars().nth(0).unwrap() != '/' {
                return Err("update_entry: absolute path required");
        }
        if p_end == 0 {
                return Err("update_entry: no entry for root directory");
        }
        if path.chars().nth(p_end).unwrap() == '/' {
                path = &path[..p_end-1];
        }
        let lname = get_lname(path);
        if lname.len() == 0 {
                return Err("update_entry: invalid path");
        }
        let chain = if lname.len() < p_end {
                let plen = p_end - lname.len();
                path = &path[..plen];
                let parent = find_entry(path).unwrap();
                fat::get_file_chain(parent.get_start())
        } else {
                ROOT_DIR.to_vec()
        };
        for clst in chain {
                let mut offset = 0;
                loop {
                        if let Some(dirent) = read_dirent(clst, offset) {
                                offset += 1;
                                if dirent.is_ext() || dirent.deleted() {
                                        continue;
                                }
                                if lname.to_uppercase() == dirent.get_name() {
                                        println!("update_entry: {} {}", clst, offset - 1);
                                        // delete_dirent(clst,offset).unwrap();
                                        write_dirent(clst, offset-1, new).unwrap();
                                        return Ok(());
                                }
                        } else {
                                break;
                        }
                }
        }
        return Err("update_entry: entry not found");
}

pub fn ls_root() {
        for cluster in ROOT_DIR.iter() {
                let mut offset = 0;
                loop {
                        if let Some(dirent) = read_dirent(*cluster, offset) {
                                offset += 1;
                                if dirent.deleted() || dirent.is_ext() {
                                        continue;
                                } 
                                print!("ls_root:{} {} \t", cluster, offset - 1);
                                dirent.print();
                        } else {
                                break;
                        }
                }
        }
}

pub fn flush() {
        flush_all();
}