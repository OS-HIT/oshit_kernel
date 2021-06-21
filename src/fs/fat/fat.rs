//! Modular that provide services on FAT 
use core::mem::size_of;

use alloc::vec::Vec;

use lazy_static::*;
use super::dbr::DBR_INST;
use super::dbr::clear_cluster;
use super::CLUSTER_CNT;
use super::super::block_cache::get_block_cache;

/// Meta-information of FAT
pub struct FAT {
        /// Start offset in sector of FAT
        pub start: u32,
        /// End offset in sector of FAT
        pub end: u32,
        /// Indicates how many u32 is in the FAT
        pub len: u32,
        /// Indecates how many u32 is in a sector
        pub clen: u32,
}


/// Type of cluster
#[derive(PartialEq, Debug)]
enum CLUSTER {
        /// A free cluster
        Free,
        /// A cluster being used temporary
        Temp,
        /// A data cluster
        Data,
        /// Reserved 
        Rsv, // reserved
        /// A bad cluster
        Bad, 
        /// A data cluster at the end of a file chain
        Eoc, // End of chain
}

impl FAT {
        /// Get cluster type
        /// # Description
        /// Given the item in FAT that curresponding to a cluster, it
        /// returns the type of the cluster 
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

        /// Get item in FAT
        /// # Description
        /// Given a cluster number, it 
        /// returns the item in FAT that corresponding to the cluster
        /// # Exception
        /// Returns None on invalid cluster number 
        fn get_next(&self, clst_num: u32) -> Option<u32> {
                if clst_num >= self.len {
                        return None;
                } 
                let block_id = clst_num / self.clen + self.start;
                let offset = clst_num % self.clen * size_of::<u32>() as u32;
                // debug!("get_next: getting block cache");
                let next = *get_block_cache(block_id as usize).lock().get_ref::<u32>(offset as usize);
                Some(next)
        }

        /// Write item in FAT
        /// # Description
        /// Given a cluster number, it writes the 'next' value to 
        /// the item in FAT corresponding to the cluster
        /// # Exception
        /// Returns Err on invalid cluster number
        fn write_next(&self, clst_num: u32, next: u32) -> Result<(),()> {
                if clst_num == 72 {
                        error!("CAUTION: writting {:X} to 72", next);
                }
                if clst_num >= self.len {
                        return Err(());
                }
                let block_id = clst_num / self.clen + self.start;
                let offset = clst_num % self.clen * size_of::<u32>() as u32;
                *get_block_cache(block_id as usize).lock().get_mut::<u32>(offset as usize) = next;
                return Ok(());
        }

        /// Get file chain
        /// # Description 
        /// Get the file chain that starts from the specified cluster.  
        /// The starting cluster don't have the be the head of the chain.  
        /// # Exception 
        /// Returns empty vector when the specifed cluster is invalid or not a data cluster
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
                                debug!("{:?}", t);
                                false
                        }
                } { 
                        t = FAT::get_type(self.get_next(cluster).unwrap()) 
                }
                return vec
        }

        /// Free a file chain
        /// # Description 
        /// Frees a file chain that starts from specified cluster.  
        /// Given cluster not being the header of the chain may cause the chain end without End of Chain cluster  
        /// # Panic
        /// It panics when non-data cluster is found in the chain
        pub fn clear_file_chain(&self, start: u32) -> Result<(),()> {
                if start == 0 {
                        return Ok(());
                }
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

        /// Get a free cluster
        /// # Description
        /// Search the entire FAT to find a free cluster.    
        /// Mark the cluster as End of Chain cluster before returning it.  
        /// # Exception
        /// Returns Err when no free cluster is found.  
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
                        clear_cluster(new).unwrap();
                        return Ok(new);
                } else {
                        return Err("get_free_cluster: no free cluster found");
                }
        }

        /// Append a cluster to a file chain
        /// # Description
        /// Given the ending cluster of a file chain, it finds a free cluster
        /// and append it to the file chain.  
        /// # Exception
        /// Returns error when given cluster is not an End of Chain cluster,
        /// or no free cluster is found. 
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
}

lazy_static! {
        /// singleton first FAT (assuming only one filesystem exists)
        pub static ref FAT_INST: FAT = DBR_INST.get_fat1();
        /// singleton second FAT (assuming only one filesystem exists)
        pub static ref FAT_INST_2: FAT = DBR_INST.get_fat2();
}


/// Print a few items in FAT
#[allow(unused)]
pub fn print_fat() {
        for i in 0..32 {
                print!("{:08X} ", FAT_INST.get_next(i).unwrap());
        }
        println!();
}

/// Wrapper for get_clusters of singleton FAT
pub fn get_file_chain(start: u32) -> Vec<u32> {
        FAT_INST.get_clusters(start)
}

/// Wrapper for clear_file_chain of FAT
/// # Description
/// It updates both FAT
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

/// Truncat a file chain
/// # Description
/// Given the cluster that the chain needs to be truncat to,
/// it truncats the file chain in both FAT
pub fn truncat_file_chain(start: u32) -> Result<(), ()> {
        debug!("truncat_file_called");
        match FAT_INST.clear_file_chain(start) {
                Ok(()) => {
                        match FAT_INST_2.clear_file_chain(start) {
                                Ok(()) => {
                                        FAT_INST.write_next(start, 0x0FFF_FFFF).unwrap();
                                        FAT_INST_2.write_next(start, 0x0FFF_FFFF).unwrap();
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

/// Wrapper for get_free_cluster of FAT
/// # Description
/// Returns a free cluster and mark it as End of chain in both FAT
pub fn get_free_cluster() -> Result<u32, &'static str> {
        match FAT_INST.get_free_cluster() {
                Ok(new) => {
                        FAT_INST_2.write_next(new, 0x0FFF_FFFF).unwrap();
                        clear_cluster(new);
                        return Ok(new);
                },
                Err(msg) => {
                        return Err(msg);
                }
        }
}


/// Wrapper for append_chain of FAT
/// # Description
/// This function modifies both FAT 
pub fn append_chain(end: u32) -> Result<u32, &'static str> {
        match FAT_INST.append_cluster(end) {
                Ok(new) => {
                        FAT_INST_2.write_next(end, new).unwrap();
                        FAT_INST_2.write_next(new, 0x0FFF_FFFF).unwrap();
                        return Ok(new);
                } ,
                Err(msg) => {
                        return Err(msg);
                }
        }
}