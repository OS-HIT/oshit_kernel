use core::mem::size_of;

use alloc::vec::Vec;

use lazy_static::*;
use super::dbr::DBR_INST;
use super::CLUSTER_CNT;
use super::super::block_cache::get_block_cache;

pub struct FAT {
        pub start: u32,
        pub end: u32,
        pub len: u32,
        pub clen: u32,
}

#[derive(PartialEq)]
enum CLUSTER {
        Free,
        Temp,
        Data,
        Rsv, // reserved
        Bad, 
        Eoc, // End of chain
}

impl FAT {
        fn get_type(clst_num: u32) -> CLUSTER {
                let mask: u32 = 0x0FFF_FFFF;
                let tmp = clst_num & mask;
                // println!("clst_num:{:#X}", clst_num);
                if tmp == 0 {
                        return CLUSTER::Free;
                } else if tmp == 1 {
                        return CLUSTER::Temp;
                } else if tmp < 0x0FFF_FFF0 {
                        return CLUSTER::Data;
                } else if tmp >= 0x0FFF_FFF8 {
                        return CLUSTER::Eoc;
                } else if tmp < 0x0FFF_FFF7 {
                        return CLUSTER::Rsv;
                } else {
                        return CLUSTER::Bad;
                } 
        }

        fn get_next(&self, clst_num: u32) -> Option<u32> {
                if clst_num >= self.len {
                        return None;
                } 
                let block_id = clst_num / self.clen + self.start;
                let offset = clst_num % self.clen * size_of::<u32>() as u32;
                Some(*get_block_cache(block_id as usize).lock().get_ref::<u32>(offset as usize))
        }

        fn write_next(&self, clst_num: u32, next: u32) -> Result<(),()> {
                if clst_num >= self.len {
                        return Err(());
                }
                let block_id = clst_num / self.clen + self.start;
                let offset = clst_num % self.clen * size_of::<u32>() as u32;
                *get_block_cache(block_id as usize).lock().get_mut::<u32>(offset as usize) = next;
                return Ok(());
        }

        pub fn get_clusters(&self, start: u32) -> Vec<u32> {
                let mut vec = Vec::new();
                if start < 2 {
                        return vec;
                }
                let mut cluster = start;
                let mut t = FAT::get_type(self.get_next(cluster).unwrap());
                while match t {
                        CLUSTER::Data => {
                                vec.push(cluster);
                                cluster = self.get_next(cluster).unwrap();
                                true
                        },
                        CLUSTER::Eoc => {
                                vec.push(cluster);
                                false
                        }
                        _ => {
                                false
                        }
                } { 
                        t = FAT::get_type(self.get_next(cluster).unwrap()) 
                }
                return vec
        }

        pub fn clear_file_chain(&self, start: u32) -> Result<(),()> {
                let mut cur = start;
                loop {
                        let next = self.get_next(cur).unwrap();
                        match FAT::get_type(next) {
                                CLUSTER::Data => {
                                        self.write_next(cur,0).unwrap();
                                        cur = next;
                                },
                                CLUSTER::Eoc => {
                                        self.write_next(cur, 0).unwrap();
                                        return Ok(());
                                }
                                _ => {
                                        panic!("clean_file_chain: ?");
                                }
                        }
                }
        }

        pub fn get_free_cluster(&self) -> Result<u32, &str> {
                let mut new = 0;
                for i in 2..*CLUSTER_CNT {
                        if FAT::get_type(self.get_next(i).unwrap()) == CLUSTER::Free {
                                new = i;
                                break;
                        }
                }
                if new != 0 {
                        self.write_next(new, 0x0FFF_FFFF).unwrap();
                        return Ok(new);
                } else {
                        return Err("get_free_cluster: no free cluster found");
                }
        }

        pub fn append_cluster(&self, end: u32) -> Result<u32, &str> {
                if FAT::get_type(self.get_next(end).unwrap()) != CLUSTER::Eoc {
                        return Err("append_cluster: not end of chain");
                }

                if let Ok(new) = self.get_free_cluster() {
                        self.write_next(end, new).unwrap();
                        return Ok(new);
                } else {
                        return Err("append_cluster: cannot find free cluster");
                }

        }

        // pub fn write_file_chain(&self, chain: &[u32]) -> Result {
        //         for clst in chain.iter() {
        //                 if clst > self.len {
        //                         return Err();
        //                 }
        //         }
        // }
}

lazy_static! {
        pub static ref FAT_INST: FAT = DBR_INST.get_fat1();
        pub static ref FAT_INST_2: FAT = DBR_INST.get_fat2();
}

#[allow(unused)]
pub fn print_fat() {
        for i in 0..32 {
                print!("{:08X} ", FAT_INST.get_next(i).unwrap());
        }
        println!();
}

pub fn get_file_chain(start: u32) -> Vec<u32> {
        FAT_INST.get_clusters(start)
}

pub fn clear_file_chain(start: u32) -> Result<(),()> {
        match FAT_INST.clear_file_chain(start) {
                Ok(()) => {
                        match FAT_INST_2.clear_file_chain(start) {
                                Ok(()) => {
                                        return Ok(());
                                },
                                _ => {
                                        return Err(());
                                }
                        }
                },
                _ => {
                        return Err(());
                }
        }
}

pub fn truncat_file_chain(start: u32) -> Result<(), ()> {
        match FAT_INST.clear_file_chain(start) {
                Ok(()) => {
                        match FAT_INST_2.clear_file_chain(start) {
                                Ok(()) => {
                                        FAT_INST.write_next(start, 0x0).unwrap();
                                        FAT_INST_2.write_next(start, 0x0).unwrap();
                                        return Ok(());
                                },
                                _ => {
                                        return Err(());
                                }
                        }
                },
                _ => {
                        return Err(());
                }
        }
}

pub fn get_free_cluster() -> Result<u32, &'static str> {
        match FAT_INST.get_free_cluster() {
                Ok(new) => {
                        FAT_INST_2.write_next(new, 0x0FFF_FFFF).unwrap();
                        return Ok(new);
                },
                Err(msg) => {
                        return Err(msg);
                }
        }
}

pub fn append_chain(end: u32) -> Result<u32, &'static str> {
        match FAT_INST.append_cluster(end) {
                Ok(new) => {
                        FAT_INST_2.write_next(end, new);
                        FAT_INST_2.write_next(end, 0x0FFF_FFFF);
                        return Ok(new);
                } ,
                Err(msg) => {
                        return Err(msg);
                }
        }
}