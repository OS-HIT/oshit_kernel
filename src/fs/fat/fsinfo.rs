#![allow(unused)]
use super::super::block_cache::get_block_cache;

use super::dbr::DBR_INST;

pub struct FSINFO {
        pub free_clst: u32,
        pub next_clst: u32,
}

impl FSINFO {
        pub const EXT_FLAG: u32 = 0x41615252;
        pub const FSINFO_SIGN: u32 = 0x61417272;
        pub const FCLST_OFFSET: usize = 0x1E8;
        pub const NCLST_OFFSET: usize = 0x1EC;

        pub fn set_fclst(&mut self, fclst: u32) -> Result<(), &str> {
                let cache = get_block_cache(1);
                if fclst > DBR_INST.cluster_cnt() {
                        return Err("set_fclst: invalid fclst");
                }
                self.free_clst = fclst;
                *cache.lock().get_mut::<u32>(FSINFO::FCLST_OFFSET) = fclst;
                return Ok(());
        }

        pub fn set_nclst(&mut self, nclst: u32) -> Result<(), &str> {
                let cache = get_block_cache(1);
                // if nclst > DBR_INST.cluster_cnt() || nclst < 2{
                //         return Err("set_nclst: invalid nclst");
                // }
                self.next_clst = nclst;
                *cache.lock().get_mut::<u32>(FSINFO::NCLST_OFFSET) = nclst;
                return Ok(());
        }
}

pub fn get_fsinfo() -> FSINFO {
        DBR_INST.get_fsinfo()
}

#[inline]
pub fn print_fsinfo() {
        println!("-------FSINFO------");
        let fi = get_fsinfo();
        println!("next free cluster: {}", fi.next_clst);
        println!("free cluster cnt: {}", fi.free_clst);
        println!();
}
