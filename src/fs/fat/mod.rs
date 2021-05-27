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
use super::dirent::DIRENT_P_CLST;
use super::path::Path;

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

pub fn read_dirent(chain: &Vec::<u32>, offset: u32) -> Option<(DirEntry, Option<String>)> {
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

pub fn write_dirent(cluster:u32, offset: u32, new: &DirEntry) -> Result<(), &'static str> {
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

pub fn delete_dirent(cluster: u32, offset: u32) -> Result<(), &'static str> {
        if cluster > *CLUSTER_CNT {
                return Err("delete_dirent: invalid cluster");
        }
        if let Some(block) = get_cluster_cache(cluster, offset * size_of::<DirEntry>() as u32) {
                let off = offset * size_of::<DirEntry>() as u32 % (*DBR_INST).sec_len;
                // println!("b:{:#010X} off:{:#010X}", block, off);
                get_block_cache(block as usize).lock().get_mut::<DirEntry>(off as usize).name[0] = 0xE5;
                return Ok(());
        } else {
                return Err("delete_dirent: get_cluster_cache failed, invalid offset ?");
        }
}

pub fn find_entry(path: &Path, is_dir: bool) -> Result<DirEntry, &'static str> {
        find_entry_from(*ROOT_DIR, path, is_dir)
}

pub fn find_entry_from(from: u32, path: &Path, is_dir: bool) -> Result<DirEntry, &'static str> {
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
        let mut depth = 0;
        for fname in path.path {
                'search: for clst in dir.iter() {
                        for i in 0..*DIRENT_P_CLST {
                                if let Some(item) = read_dirent(*clst, i) {
                                        if item.deleted() || item.is_ext() {
                                                continue;
                                        }
                                        if fname == item.get_name() {
                                                dirent = Some(item);
                                                depth += 1;
                                                dir = item.get_chain();
                                                break 'search;
                                        }
                                } else {
                                        break;
                                }
                        }
                }
        }
        if depth == path.len() {
                if let Some(de) = dirent {
                        if is_dir && !de.is_dir() {
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
        if path.len() == 0 {
                return Err("delete_entry: no entry for root directory");
        }
        let mut parent_path = path.clone();
        let file = parent_path.pop().unwrap();
        let chain = if parent_path.len() == 0 {
                get_file_chain(*ROOT_DIR)
        } else {
                let entry = find_entry(&parent_path, true).unwrap();
                get_file_chain(entry.get_start())
        };
        for clst in chain {
                for offset in 0..*DIRENT_P_CLST {
                        if let Some(dirent) = read_dirent(clst, offset) {
                                if dirent.is_ext() || dirent.deleted() {
                                        continue;
                                }
                                if cat_name(&file) == dirent.get_name() {
                                        if is_dir && !dirent.is_dir() {
                                                return Err("delete_entry: not a directory");
                                        }
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

pub fn update_entry(path: &Path, is_dir: bool, new: &DirEntry) -> Result<(), &'static str> {
        if path.len() == 0 {
                return Err("update_entry: no entry for root directory");
        }
        let mut parent_path = path.clone();
        let file = parent_path.pop().unwrap();
        let chain = if parent_path.len() == 0 {
                get_file_chain(*ROOT_DIR)
        } else {
                let entry = find_entry(&parent_path, true).unwrap();
                get_file_chain(entry.get_start())
        };
        for clst in chain {
                for offset in 0..*DIRENT_P_CLST {
                        if let Some(dirent) = read_dirent(clst, offset) {
                                if dirent.is_ext() || dirent.deleted() {
                                        continue;
                                }
                                if is_dir && !dirent.is_dir() {
                                        return Err("update_entry: not a directory");
                                }
                                if cat_name(&file) == dirent.get_name() {
                                        write_dirent(clst, offset, new).unwrap();
                                        return Ok(());
                                }
                        } else {
                                break;
                        }
                }
        }
        return Err("update_entry: entry not found");
}

#[inline]
//                                  (clst, offset, update_size)
fn get_free_entry(chain: &Vec<u32>) -> (u32, u32, bool) {
        for clst in chain {
                for offset in 0..*DIRENT_P_CLST {
                        if let Some(dirent) = read_dirent(*clst, offset) {
                                if dirent.deleted() {
                                        return (*clst, offset, false);
                                }
                        } else {
                                return (*clst, offset, true);
                        }
                }
        }
        if let Ok(clst) = append_chain(chain[chain.len() - 1]) {
                return (clst, 0, true);
        }
        return (0,0, false);
}

pub fn new_entry(parent: &Path, new: &DirEntry) -> Result<(), &'static str> {
        let mut fchain = get_file_chain(*ROOT_DIR);
        let mut entry: Option<DirEntry> = None;
        if parent.len() != 0 {
                if let Ok(ent) = find_entry(&parent, true) {
                        entry = Some(ent);
                        fchain = ent.get_chain();
                } else {
                        return Err("new_entry: parent not found");
                }
        }
        let (clst, offset, update_size) = get_free_entry(&fchain);
        if clst == 0 {
                return Err("new_entry: no space for new entry");
        }
        if update_size && parent.len() != 0{
                if let Some(mut entry) = entry {
                        entry.size += size_of::<DirEntry>() as u32;
                        update_entry(&parent, true, &entry).unwrap();
                } else {
                        return Err("new_entry: what happened to my entry?");
                }
        }
        write_dirent(clst, offset, new)
}

pub fn new_entry_at(parent: &DirEntry, new: &DirEntry) -> Result<bool, &'static str> {
        let fchain = get_file_chain(parent.get_start());
        let (clst, offset, update_size) = get_free_entry(&fchain);
        if clst == 0 {
                return Err("new_entry_at: no space for new entry");
        } 
        if let Err(msg) = write_dirent(clst, offset, new) {
                return Err(msg);
        } else {
                return Ok(update_size);
        }
}

#[allow(unused)]
pub fn ls_root() {
        let chain = get_file_chain(*ROOT_DIR);
        for cluster in chain.iter() {
                for offset in 0..*DIRENT_P_CLST {
                        if let Some(dirent) = read_dirent(*cluster, offset) {
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