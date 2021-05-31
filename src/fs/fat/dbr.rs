use core::str::from_utf8;
use core::mem::size_of;
use lazy_static::*;

use alloc::string::String;

use super::mbr::MBR_INST;

use super::super::block_cache::get_block_cache;
use super::super::block_cache::clear_block_cache;
use super::super::block_cache::BLOCK_SZ;

use super::fsinfo::FSINFO;
use super::fat::FAT;

fn b2u32(b: &[u8; 4]) -> u32 {
        b[0] as u32 
        | ((b[1] as u32) << 8)
        | ((b[2] as u32) << 16)
        | ((b[3] as u32) << 24)
}

fn b2u16(b: &[u8; 2]) -> u16 {
        b[0] as u16 
        | ((b[1] as u16) << 8)
}

#[derive(Clone, Copy)]
#[repr(C, packed(1))]
pub struct RAW_DBR {
        // offset: 00
        pub jump: [u8; 3],  // jump instruction
        pub name: [u8; 8],  // 
        pub sec_len: [u8; 2],   // sector_length in bytes
        pub clst_len: u8,   // cluster length in sectors
        pub rsv_sec: [u8; 2],   // reserved sector count(sectors before FAT)
        // offset: 10
        pub fat_cnt: u8,    // FAT count
        pub zero0: [u8; 2],     // zero field for FAT32
        pub zero1: [u8; 2],     // zero field for FAT32
        pub medium: u8,     // 
        pub zero2: u16,     // zero field for FAT32
        pub sec_p_track: [u8; 2],// sector count on a single track(only valid for certain medium)
        pub header: [u8; 2],    // header count(only valid for certain medium)
        pub hidden: [u8; 4],    // hidden sector count(MBR) before DBR
        // offset: 20
        pub sec_cnt: [u8; 4],   // total sector count of filesystem
        pub fat_sec: [u8; 4],   // sector count of a single FAT 
        pub flag: [u8; 2],      //
        pub version: [u8; 2],   // version #
        pub root: [u8; 4],      // cluster # for root directory
        // offset: 30
        pub fsinfo: [u8; 2],    // sector # for FSINFO(always 0x01)
        pub boot: [u8; 2],      // backup DBR sector(always 6)
        pub reserved: [u8; 12],
        // offset: 40
        pub unknown: [u8; 3], // what the hell is this
        pub vol: [u8; 4],     // volumn # (random value)
        pub vol_name: [u8; 11], //
        pub fat32: [u8; 8], // "FAT32"
        pub text: [u8; 420],// booting instructions
        pub sign: [u8; 2],  // 0x55 0xAA
}

impl RAW_DBR {
        pub fn get_dbr(partition: usize) -> RAW_DBR {
                let cache = get_block_cache(MBR_INST.par_tab[partition].start as usize);
                let dbr = *cache.lock().get_ref::<RAW_DBR>(0);
                if dbr.sign[0] != 0x55 || dbr.sign[1] != 0xAA {
                        panic!("get_dbr: Invalid dbr");
                }
                dbr
        }
}

pub struct DBR {
        pub vol: u32,
        pub vol_name:  [u8; 11],
        pub name: [u8; 8],
        pub fat32: [u8; 8],
        pub version: u16,

        pub fat_cnt: u32,
        pub fat_sec: u32,       // fat size in sectors
        pub fat_len: u32,       // fat size in bytes

        pub sec_len: u32,       
        pub sec_cnt: u32,
        pub rsv_sec: u32,
        pub data_sec_base: u32,

        pub clst_sec: u32,      // cluster size in sectors
        pub clst_size: u32,     // cluster size in bytes
        pub clst_cnt: u32,      // total cluster count in disk

        pub root: u32,
        pub boot: u32,
}

impl DBR {
        pub fn new() -> DBR {
                let mut partition:isize = -1;
                for i in 0..4 {
                        if MBR_INST.par_tab[i].id != 0 {
                                partition = i as isize;
                        }
                }
                if partition == -1 {
                        panic!("no fat partition found");
                }
                DBR::from_raw(RAW_DBR::get_dbr(partition as usize), MBR_INST.par_tab[partition as usize].start)
        }

        pub fn from_raw(raw: RAW_DBR, start_sector: u32) -> Self {
                let mut fat32 = [0u8; 8];
                for i in 0..fat32.len() {
                        fat32[i] = raw.fat32[i];
                }
                let mut vol_name = [0u8; 11];
                for i in 0..vol_name.len() {
                        vol_name[i] = raw.vol_name[i];
                }
                let mut name = [0u8; 8];
                for i in 0..name.len() {
                        name[i] = raw.name[i];
                }

                
                let sec_len = b2u16(&raw.sec_len) as u32;
                let sec_cnt = b2u32(&raw.sec_cnt) as u32;
                let rsv_sec = b2u16(&raw.rsv_sec) as u32 + start_sector;
                
                let fat_sec = b2u32(&raw.fat_sec);
                let fat_cnt = raw.fat_cnt as u32;
                let fat_len = fat_sec * sec_len;
                
                let data_sec_base = rsv_sec + fat_cnt * fat_sec;

                DBR {
                        vol: b2u32(&raw.vol),
                        vol_name,
                        name,
                        fat32,
                        version: b2u16(&raw.version),

                        sec_len,
                        sec_cnt,
                        rsv_sec,
                        data_sec_base,

                        clst_sec: raw.clst_len as u32,
                        clst_size: raw.clst_len as u32 * sec_len,
                        clst_cnt: (sec_cnt - data_sec_base) / raw.clst_len as u32, 

                        fat_cnt,
                        fat_sec,
                        fat_len,

                        root: b2u32(&raw.root),
                        boot: b2u16(&raw.boot) as u32,
                }       
        }

        pub fn print(&self) {
                println!("------DBR---------");
                println!("{} Version {}", from_utf8(&self.fat32).unwrap(), self.version );
                println!("vol:\t{:#X}\t{}", self.vol, from_utf8(&self.vol_name).unwrap());
                println!("builder:\t{}", from_utf8(&self.name).unwrap());
                println!("DBR(and MBR) length:{}", self.rsv_sec * self.sec_len);
                println!("sector length:\t{}", self.sec_len);
                println!("sector count:\t{}", self.sec_cnt);
                println!("cluster length:\t{}", self.clst_size);
                println!("cluster count:\t{}", self.clst_cnt);
                println!("FAT count:\t{}", self.fat_cnt);
                println!("FAT length:\t{}", self.fat_len);
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
                let clen  = self.sec_len / size_of::<u32>() as u32;
                let fat_len = self.fat_len / size_of::<u32>() as u32;
                return FAT{ 
                        start: block_id, 
                        end: block_id + self.fat_sec, 
                        len: fat_len,
                        clen,
                };
        }

        pub fn get_fat2(&self) -> FAT {
                let block_id = self.rsv_sec as u32 + self.fat_sec;
                let clen  = self.sec_len / size_of::<u32>() as u32;
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
        pub static ref DBR_INST: DBR = DBR::new();
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
        if cluster > DBR_INST.clst_cnt || offset > DBR_INST.clst_size {
                return None;
        }
        let mut sector: u32 = (*DBR_INST).data_sec_base + (*DBR_INST).clst_sec * cluster;
        sector += offset / (*DBR_INST).sec_len;
        return Some(sector);
}

pub fn read_cluster(cluster: u32, offset: u32, buf: &mut [u8]) ->Result<u32,&str> {
        if cluster >= DBR_INST.clst_cnt {
                return Err("read_cluster: Invalid cluster");
        }
        if offset >= DBR_INST.clst_size {
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
                if offset >= DBR_INST.clst_size {
                        return Ok(read);
                } 
        }
        return Ok(buf.len() as u32);
}

pub fn write_cluster(cluster: u32, offset: u32, buf: &[u8]) -> Result<u32, &str> {
        if cluster >= DBR_INST.clst_cnt {
                return Err("read_cluster: Invalid cluster");
        }
        if offset >= DBR_INST.clst_size {
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
                if offset >= DBR_INST.clst_size {
                        return Ok(write);
                } 
        }
        return Ok(buf.len() as u32);
}

pub fn clear_cluster(cluster:u32) -> Result<(), &'static str> {
        if cluster >= DBR_INST.clst_cnt {
                return Err("clear_cluster: Invalid cluster");
        } 
        if let Some(block) = get_cluster_cache(cluster, 0) {
                for i in 0..(*super::CLUSTER_SIZE / BLOCK_SZ as u32) {
                        clear_block_cache((block+i) as usize);
                }
        }
        return Ok(());
}