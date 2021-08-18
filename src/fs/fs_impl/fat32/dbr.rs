//! Dos Boot Record parser

use core::str::from_utf8;

/// bytes to u32
/// # Description
/// Read u32 from byte slice in little endian without causing LoadMisalign
fn b2u32(b: &[u8; 4]) -> u32 {
        b[0] as u32 
        | ((b[1] as u32) << 8)
        | ((b[2] as u32) << 16)
        | ((b[3] as u32) << 24)
}

/// bytes to u16
/// # Description
/// Read u16 from byte slice in little endian withou causing LoadMisalign
fn b2u16(b: &[u8; 2]) -> u16 {
        b[0] as u16 
        | ((b[1] as u16) << 8)
}


/// Raw Dos Boot Record
/// # Description
/// A Struct that has the same layout as DBR on Block Device
/// Reading fields from it directly may cause LoadMisalign.
/// This's why most field are presented as byte array
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

/// Simplified version of DBR
/// # Simplified DBR, containing only the info needed for file operations.
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
        /// Build DBR from RAW_DBR
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
                // let sec_cnt = b2u32(&raw.sec_cnt) as u32;
                let sec_cnt = 0x0fff_0000;
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

        /// Print DBR
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
}







