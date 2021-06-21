//! Definition of DirEntry (dirent)
use core::str::from_utf8;
use alloc::vec::Vec;
use alloc::string::String;
use lazy_static::*;
use core::mem::size_of;

use super::fat::CLUSTER_SIZE;


/// DirEntry (dirent) FAT32 Version
#[derive(Clone, Copy)]
#[repr(C, packed(1))]
pub struct DirEntry {
        /// file name
        pub name: [u8; 8],
        /// extension name
        pub ext: [u8; 3],
        /// file attribute
        pub attr: u8,
        pub reserved: u8,
        /// created time minisec part
        pub created_minisec: u8,
        /// created time sec part
        pub created_sec: u16,
        /// created date
        pub created_date: u16,
        /// last accessed time
        pub accessed_sec: u16,
        /// start cluster (high 16 bits)
        pub start_h: u16,
        /// last modified time
        pub mod_sec: u16,
        /// last modified date
        pub mod_date: u16,
        /// start cluster (low 16 bits)
        pub start_l: u16,
        /// file size
        pub size: u32,
}

lazy_static! {
        /// dirent count in a single cluster
        pub static ref DIRENT_P_CLST: u32 = *CLUSTER_SIZE / size_of::<DirEntry>() as u32;
}

/// dirent for long file name
#[derive(Clone, Copy)]
#[repr(C, packed(1))]
pub struct DirEntryExt {
        /// extra attribute for long file name
        pub ext_attr: u8,
        /// file name part 0
        pub name0: [u8; 10],
        /// file attribute, always 0x0f
        pub attr: u8,
        pub reserved: u8,
        /// check sum
        pub chksum: u8,
        /// file name part 1
        pub name1: [u8; 12],
        /// not in use field
        pub start: u16,
        /// file name part 2
        pub name2: [u8; 4],
}

impl DirEntryExt {
        /// Flag indicate end of long file name
        const EXT_END: u8 = 0x40;

        /// Build a group of long file name entries for the spcecified name
        /// # Description
        /// check sum is also required
        pub fn new(name: &String, chksum: u8) -> Vec<DirEntryExt> {
                let mut result = Vec::<DirEntryExt>::new();
                let mut name:Vec<u16> = name.encode_utf16().collect();
                while name[name.len() - 1] == 0 {
                        name.pop();
                }
                if name.len() % 13 != 0 {
                        name.push(0);
                }
                while name.len() % 13 != 0 {
                        name.push(0xffff);
                }
                let cnt = name.len() / 13;
                let mut i = 0;
                while i < cnt {
                        let mut name0 = [0u8; 10];
                        let base = i*13;
                        for j in 0..5 {
                                name0[2*j] = (name[base + j] & 0xff) as u8;
                                name0[2*j+1] = (name[base + j] >> 8) as u8;
                        }
                        let mut name1 = [0u8; 12];
                        let base = base + 5;
                        for j in 0..6 {
                                name1[2*j] = (name[base + j] & 0xff) as u8;
                                name1[2*j+1] = (name[base + j] >> 8) as u8;
                        }
                        let mut name2 = [0u8;4];
                        let base = base + 6;
                        for j in 0..2 {
                                name2[2*j] = (name[base + j] & 0xff) as u8;
                                name2[2*j+1] = (name[base + j] >> 8) as u8;
                        }
                        let dex = DirEntryExt{
                                ext_attr: (i+1) as u8,
                                name0,
                                attr: DirEntry::ATTR_LFN,
                                reserved: 0,
                                chksum,
                                name1,
                                start: 0,
                                name2,
                        };
                        result.push(dex);
                        i += 1;
                }
                let last = cnt -1;
                result[last].ext_attr |= DirEntryExt::EXT_END;
                // debug!("DirEntryExt::new return with len{} {:x}", result.len(), result[last].ext_attr);
                return result;
        }

        /// Check if the entry is the last one of long file name entries
        #[inline]
        pub fn is_end(&self) -> bool {
                return self.ext_attr & DirEntryExt::EXT_END == DirEntryExt::EXT_END;
        }

        /// Get index 
        #[inline]
        pub fn get_index(&self) -> u8 {
                return self.ext_attr & !DirEntryExt::EXT_END;
        }

        /// Check if the entry is a long file name entry
        #[inline]
        pub fn is_ext(&self) -> bool {
                return self.attr == DirEntry::ATTR_LFN;
        }

        /// Get the part of filename that the entry holds
        pub fn get_name(&self) -> Vec::<u8> {
                let mut name = Vec::with_capacity(26);
                for b in &self.name0 {
                        if *b == 0xFF {
                                return name;
                        } else {
                                name.push(*b);
                        } 
                }
                for b in &self.name1 {
                        if *b == 0xFF {
                                return name;
                        } else {
                                name.push(*b);
                        } 
                }
                for b in &self.name2 {
                        if *b == 0xFF {
                                return name;
                        } else {
                                name.push(*b);
                        } 
                }
                return name;
        }
}

impl DirEntry {
        /// not in use
        const ATTR_RDWR:u8 = 0x00;
        /// Read only attribute flag
        const ATTR_RDONLY:u8 = 0x01;
        /// Hidden attribute flag
        const ATTR_HIDDEN:u8 = 0x02;
        /// System file attribute flag
        const ATTR_SYM: u8 = 0x04;
        /// Volume file attribute flag
        const ATTR_VOL: u8 = 0x08;
        /// Directory attribute flag
        const ATTR_SUBDIR: u8 = 0x10;
        /// Regular file attribute flag
        const ATTR_FILE: u8 = 0x20;
        /// Long file name attribute flag
        const ATTR_LFN: u8 = 0x0f;

        /// Get Regular file attribute flag
        #[inline]
        pub fn attr_file() -> u8 {
                return DirEntry::ATTR_FILE;
        }

        /// Get Directory attribute flag
        #[inline]
        pub fn attr_dir() -> u8 {
                return DirEntry::ATTR_SUBDIR;
        }

        /// Check whether the dirent is marked as deleted
        #[inline]
        pub fn deleted(&self) -> bool {
                return self.name[0] == 0xE5;
        }

        /// Check whether the dirent is long file name entry
        #[inline]
        pub fn is_ext(&self) -> bool {
                return self.attr & DirEntry::ATTR_LFN == DirEntry::ATTR_LFN;
        }

        /// Check whether the dirent is a directory
        #[inline]
        pub fn is_dir(&self) -> bool {
                return self.attr & DirEntry::ATTR_SUBDIR == DirEntry::ATTR_SUBDIR;
        }

        /// Check whether the dirent is file
        #[inline]
        pub fn is_file(&self) -> bool {
                return self.attr & DirEntry::ATTR_FILE == DirEntry::ATTR_FILE;
        }

        /// Check whether the dirent is read only
        #[inline]
        pub fn is_read_only(&self) -> bool {
                return self.attr & DirEntry::ATTR_RDONLY == DirEntry::ATTR_RDONLY;
        }

        /// Check whether the dirent is volumn file
        #[inline]
        pub fn is_vol(&self) -> bool {
                return self.attr == DirEntry::ATTR_VOL;
        }

        /// Get starting cluster of file chain of the dirent
        pub fn get_start(&self) -> u32 {
                let mut start = self.start_h as u32;
                start <<= 16;
                start |= self.start_l as u32;
                return start; 
        }

        /// Set starting cluster of file chain of the dirent
        pub fn set_start(&mut self, start: u32) {
                self.start_h = (start >> 16) as u16;
                self.start_l = (start & 0xff) as u16;
        }

        /// Set file chain of the dirent
        pub fn get_chain(&self) -> Vec<u32> {
                super::fat::fat::get_file_chain(self.get_start())
        }

        /// Get short name of the dirent
        pub fn get_name(&self) -> String {
                let mut name = String::new();
                name += from_utf8(&self.name).unwrap().trim();
                // println!("{}: {}", name.len(), name);
                let mut ext = String::new();
                ext += from_utf8(&self.ext).unwrap().trim();
                // debug!("ext len:{} ext: {} {} {}", ext.len(), self.ext[0], self.ext[1], self.ext[2]);
                if ext.len() > 0 {
                        name += ".";
                        name += &ext;
                }
                return name;
        }

        /// Set name of the dirent
        /// # Description
        /// Name must be shorter then 255, store in 8 + 3 format
        pub fn set_name(&mut self, name: &String) {
                let b:Vec<u8> = name.bytes().collect();
                for i in (0..b.len()).rev() {
                        if b[i] == '.' as u8 {
                                let name_len = i;
                                let ext_len = b.len() - i - 1;
                                let mut name_ok = true;
                                if name_len > 0 && name_len <= 8 {
                                        for j in 0..name_len {
                                                self.name[j] = b[j].to_ascii_uppercase();
                                        }
                                        for j in name_len..8 {
                                                self.name[j] = ' ' as u8;
                                        }
                                } else if name_len == 8 {
                                        for j in 0..6 {
                                                self.name[j] = b[j].to_ascii_uppercase();
                                        }
                                        self.name[6] = '~' as u8;
                                        self.name[7] = '1' as u8;
                                } else {
                                        name_ok = false;
                                }
                                if name_ok {
                                        if ext_len > 0 && ext_len <= 3 {
                                                for j in 0..ext_len {
                                                        self.ext[j] = b[i+1+j].to_ascii_uppercase();
                                                }
                                                for j in ext_len..3 {
                                                        self.ext[j] = ' ' as u8;
                                                }
                                                return;
                                        } else if ext_len > 3 {
                                                for j in 0..3 {
                                                        self.ext[j] = b[i+1+j].to_ascii_uppercase();
                                                }
                                                return;
                                        } 

                                }
                        }
                }
                if b.len() <= 8 {
                        let name_len = b.len();
                        for j in 0..name_len {
                                self.name[j] = b[j].to_ascii_uppercase();
                        }
                        for j in name_len..8 {
                                self.name[j] = ' ' as u8;
                        }
                        for j in 0..3 {
                                self.ext[j] = ' ' as u8;
                        }
                } else {
                        for j in 0..6 {
                                self.name[j] = b[j].to_ascii_uppercase();
                        }
                        self.name[6] = '~' as u8;
                        self.name[7] = '1' as u8;
                        for j in 0..3 {
                                self.ext[j] = ' ' as u8;
                        }
                }
        }

        /// Calculate checksum of dirent
        pub fn chksum(&self) -> u8 {
                let mut sum:u8 = 0;
                for i in 0..8 {
                        sum = (if sum & 1 != 0 {0x80u8} else {0}).wrapping_add(sum >> 1).wrapping_add(self.name[i]);
                }
                for i in 0..3 {
                        sum = (if sum & 1 != 0 {0x80u8} else {0}).wrapping_add(sum >> 1).wrapping_add(self.ext[i]);
                }
                return sum;
        }

        /// Print raw content of dirent
        pub fn print_raw(&self) {
                print!("name:");
                for i in 0..8 {
                        print!("{} ", self.name[i]);
                }
                print!("| ");
                for i in 0..3 {
                        print!("{} ", self.ext[i]);
                }
                println!();
                println!("attr:{}", self.attr);
                println!("start_h:{}", self.start_h);
                println!("start_l:{}", self.start_l);
                println!("size: {}", self.size);
                println!("-------");
        }
        
        /// print dirent
        pub fn print(&self) {
                if self.deleted() {
                        print!("deleted: ");
                }
                if self.is_ext() {
                        print!("Entry for long file name");
                } else {
                        // print!("{}.{}\t", from_utf8(&self.name).unwrap(), from_utf8(&self.ext).unwrap());
                        print!("{:16}", self.get_name());
                        unsafe{ print!("{:#10} ", self.size) };
                        if self.attr & DirEntry::ATTR_RDONLY != 0 {
                                print!("R");
                        }
                        if self.attr & DirEntry::ATTR_HIDDEN != 0 {
                                print!("H");
                        }
                        if self.attr & DirEntry::ATTR_SYM != 0 {
                                print!("S");
                        }
                        if self.attr & DirEntry::ATTR_VOL != 0 {
                                print!("V");
                        }
                        if self.attr & DirEntry::ATTR_SUBDIR != 0 {
                                print!("D");
                        }
                        if self.attr & DirEntry::ATTR_FILE != 0 {
                                print!("F");
                        }
                        // print!("\t");
                        // let chain = self.get_chain();
                        // if chain.len() == 0 {
                        //         print!("(null)");
                        // } else {
                        //         for i in 0..(chain.len() - 1) {
                        //                 print!("{}->", chain[i]);
                        //         }
                        //         print!("{}", chain[chain.len()-1]);
                        // }
                }
                println!();
        }

        
}