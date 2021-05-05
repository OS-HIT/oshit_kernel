use core::str::from_utf8;
use core::mem::size_of;
use lazy_static::*;

use super::super::block_cache::get_block_cache;
use super::super::block_cache::BLOCK_SZ;

use super::fsinfo::FSINFO;
use super::fat::FAT;

#[derive(Clone, Copy)]
#[repr(C, packed(1))]
pub struct DBR {
        pub jump: [u8; 3],  // jump instruction
        pub name: [u8; 8],  // 
        pub sec_len: u16,   // sector_length in bytes
        pub clst_len: u8,   // cluster length in sectors
        pub rsv_sec: u16,   // reserved sector count(sectors before FAT)
        pub fat_cnt: u8,    // FAT count
        pub zero0: u16,     // zero field for FAT32
        pub zero1: u16,     // zero field for FAT32
        pub medium: u8,     // 
        pub zero2: u16,     // zero field for FAT32
        pub sec_p_track: u16,// sector count on a single track(only valid for certain medium)
        pub header: u16,    // header count(only valid for certain medium)
        pub hidden: u32,    // hidden sector count(MBR) before DBR
        pub sec_cnt: u32,   // total sector count of filesystem
        pub fat_sec: u32,   // sector count of a single FAT 
        pub flag: u16,      //
        pub version: u16,   // version #
        pub root: u32,      // cluster # for root directory
        pub fsinfo: u16,    // sector # for FSINFO(always 0x01)
        pub boot: u16,      // backup DBR sector(always 6)
        pub reserved: [u8; 12],
        pub unknown: [u8; 3], // what the hell is this
        pub vol: u32,       // volumn # (random value)
        pub vol_name: [u8; 11], //
        pub fat32: [u8; 8], // "FAT32"
        pub text: [u8; 420],// booting instructions
        pub sign: [u8; 2],  // 0x55 0xAA
}

impl DBR {
        pub fn get_dbr() -> DBR {
                let cache = get_block_cache(0);
                let dbr = *cache.lock().get_ref::<DBR>(0);
                if dbr.sign[0] != 0x55 || dbr.sign[1] != 0xAA {
                        panic!("get_dbr: Invalid dbr");
                }
                dbr
        }

        #[inline]
        pub fn data_sec_base(&self) -> u32 {
                self.rsv_sec as u32 + self.fat_cnt as u32 * self.fat_sec
        }

        #[inline]
        pub fn cluster_cnt(&self) -> u32 {
                (self.sec_cnt - self.data_sec_base()) / self.clst_len as u32
        }

        #[inline]
        pub fn cluster_size(&self) -> u32 {
                self.clst_len as u32 * self.sec_len as u32
        }

        pub unsafe fn print(&self) {
                println!("------DBR---------");
                println!("{} Version {}", from_utf8(&self.fat32).unwrap(), self.version );
                println!("DBR(and MBR) length:{}", self.rsv_sec * self.sec_len);
                println!("vol:\t{:#X}\t{}", self.vol, from_utf8(&self.vol_name).unwrap());
                println!("builder:\t{}", from_utf8(&self.name).unwrap());
                println!("medium:\t\t{:#X}", self.medium);
                println!("sector length:\t{}", self.sec_len);
                println!("sector count:\t{}", self.sec_cnt);
                println!("cluster length:\t{}", self.cluster_size());
                println!("cluster count:\t{}", self.cluster_cnt());
                println!("FAT count:\t{}", self.fat_cnt);
                println!("FAT length:\t{}", self.fat_sec * self.sec_len as u32);
                println!("backup sector:\t{}\n", self.boot);
        }

        pub fn get_fsinfo(&self) -> FSINFO {
                let cache = get_block_cache(1);
                if *cache.lock().get_ref::<u32>(0) != FSINFO::EXT_FLAG {
                        panic!("get_fsinfo: invalid extension flag");
                } 
                if *cache.lock().get_ref::<u32>(0x1E4) != FSINFO::FSINFO_SIGN {
                        panic!("get_fsinfo: invalid fsinfo signature");
                }
                if *cache.lock().get_ref::<u16>(0x1FE) != 0xAA55 {
                        panic!("get_fsinfo: invalid block signature");
                }
                let free_clst = *cache.lock().get_ref::<u32>(FSINFO::FCLST_OFFSET);
                let next_clst = *cache.lock().get_ref::<u32>(FSINFO::NCLST_OFFSET);
                FSINFO {
                        free_clst,
                        next_clst,
                }
        }

        pub fn get_fat1(&self) -> FAT {
                let block_id = self.rsv_sec as u32;
                let clen  = self.sec_len as u32 / size_of::<u32>() as u32;
                let fat_len = self.fat_sec * clen;
                return FAT{ 
                        start: block_id, 
                        end: block_id + self.fat_sec, 
                        len: fat_len,
                        clen: clen,
                };
        }

        pub fn get_fat2(&self) -> FAT {
                let block_id = self.rsv_sec as u32 + self.fat_sec;
                let clen  = self.sec_len as u32 / size_of::<u32>() as u32;
                let fat_len = self.fat_sec * clen;
                return FAT{ 
                        start: block_id, 
                        end: block_id + self.fat_sec, 
                        len: fat_len,
                        clen: clen,
                };
        }
}

lazy_static! {
        pub static ref DBR_INST: DBR = DBR::get_dbr(); 
}

#[allow(unused)]
pub fn print_dbr() {
        unsafe{
                DBR_INST.print();
        }
}

pub fn get_cluster_cache(cluster: u32, offset: u32) -> Option<u32> {
        if cluster < DBR_INST.root {
                return None;
        }
        let cluster = cluster - DBR_INST.root;
        if cluster > DBR_INST.cluster_cnt() || offset > DBR_INST.cluster_size() {
                return None;
        }
        let mut sector: u32 = (*DBR_INST).data_sec_base() + (*DBR_INST).clst_len as u32 * cluster;
        sector += offset / (*DBR_INST).sec_len as u32;
        return Some(sector);
}

pub fn read_cluster(cluster: u32, offset: u32, buf: &mut [u8]) ->Result<u32,&str> {
        if cluster >= DBR_INST.cluster_cnt() {
                return Err("read_cluster: Invalid cluster");
        }
        if offset >= DBR_INST.cluster_size() {
                return Err("read_cluster: Invalid Offset");
        }
        
        let mut len = buf.len();
        let mut read:u32 = 0;
        let mut offset = offset;
        while len > 0 {
                let block = get_cluster_cache(cluster, offset).unwrap();
                let off = offset as usize % BLOCK_SZ;
                let cache = get_block_cache(block as usize).clone();
                let rlen = BLOCK_SZ as u32 - (offset % BLOCK_SZ as u32);
                let rlen = if rlen > len as u32 {len as u32} else {rlen};
                for i in 0..rlen as usize {
                        buf[read as usize + i] = *cache.lock().get_ref::<u8>(off + i);
                }
                len -= rlen as usize;
                offset += rlen;
                read += rlen;
                if offset >= DBR_INST.cluster_size() {
                        return Ok(read);
                } 
        }
        return Ok(buf.len() as u32);
}

pub fn write_cluster(cluster: u32, offset: u32, buf: &[u8]) -> Result<u32, &str> {
        if cluster >= DBR_INST.cluster_cnt() {
                return Err("read_cluster: Invalid cluster");
        }
        if offset >= DBR_INST.cluster_size() {
                return Err("read_cluster: Invalid Offset");
        }

        let mut len = buf.len();
        let mut write: u32 = 0;
        let mut offset = offset;
        while len > 0 {
                let block = get_cluster_cache(cluster, offset).unwrap();
                let off = offset as usize % BLOCK_SZ;
                let cache = get_block_cache(block as usize).clone();
                let wlen = BLOCK_SZ as u32 - (offset % BLOCK_SZ as u32);
                let wlen = if wlen > len as u32 {len as u32} else {wlen};
                for i in 0..wlen as usize {
                        *cache.lock().get_mut::<u8>(off + i) = buf[write as usize + i];
                }
                len -= wlen as usize;
                offset += wlen;
                write += wlen;
                if offset >= DBR_INST.cluster_size() {
                        return Ok(write);
                } 
        }
        return Ok(buf.len() as u32);
}