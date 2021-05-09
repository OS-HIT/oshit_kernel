
use core::str::from_utf8;
use alloc::vec::Vec;
use alloc::string::String;
use lazy_static::*;
use core::mem::size_of;

use super::fat::CLUSTER_SIZE;

#[derive(Clone, Copy)]
#[repr(C, packed(1))]
pub struct DirEntry {
        pub name: [u8; 8],
        pub ext: [u8; 3],
        pub attr: u8,
        pub reserved: u8,
        pub created_minisec: u8,
        pub created_sec: u16,
        pub created_date: u16,
        pub accessed_sec: u16,
        pub start_h: u16,
        pub mod_sec: u16,
        pub mod_date: u16,
        pub start_l: u16,
        pub size: u32,
}

lazy_static! {
        pub static ref DIRENT_P_CLST: u32 = *CLUSTER_SIZE / size_of::<DirEntry>() as u32;
}

#[repr(C, packed(1))]
struct DirEntryExt {
        ext_attr: u8,
        name0: [u8; 10],
        attr: u8,
        reserved: u8,
        exam_code: u8,
        name1: [u8; 12],
        start: u16,
        name2: [u8; 4],
}

impl DirEntry {
        const ATTR_RDWR:u8 = 0x00;
        const ATTR_RDONLY:u8 = 0x01;
        const ATTR_HIDDEN:u8 = 0x02;
        const ATTR_SYM: u8 = 0x04;
        const ATTR_VOL: u8 = 0x08;
        const ATTR_SUBDIR: u8 = 0x10;
        const ATTR_FILE: u8 = 0x20;
        const ATTR_LFN: u8 = 0x0f;

        #[inline]
        pub fn attr_file() -> u8 {
                return DirEntry::ATTR_FILE;
        }

        #[inline]
        pub fn attr_dir() -> u8 {
                return DirEntry::ATTR_SUBDIR;
        }

        #[inline]
        pub fn deleted(&self) -> bool {
                return self.name[0] == 0xE5;
        }

        #[inline]
        pub fn is_ext(&self) -> bool {
                return self.attr & DirEntry::ATTR_LFN == DirEntry::ATTR_LFN;
        }

        #[inline]
        pub fn is_dir(&self) -> bool {
                return self.attr & DirEntry::ATTR_SUBDIR == DirEntry::ATTR_SUBDIR;
        }

        #[inline]
        pub fn is_file(&self) -> bool {
                return self.attr & DirEntry::ATTR_FILE == DirEntry::ATTR_FILE;
        }

        #[inline]
        pub fn is_read_only(&self) -> bool {
                return self.attr & DirEntry::ATTR_RDONLY == DirEntry::ATTR_RDONLY;
        }

        pub fn get_start(&self) -> u32 {
                let mut start = self.start_h as u32;
                start <<= 16;
                start |= self.start_l as u32;
                return start; 
        }

        pub fn set_start(&mut self, start: u32) {
                self.start_h = (start >> 16) as u16;
                self.start_l = (start & 0xff) as u16;
        }

        pub fn get_chain(&self) -> Vec<u32> {
                super::fat::fat::get_file_chain(self.get_start())
        }

        pub fn get_name(&self) -> String {
                let mut name = String::new();
                name += from_utf8(&self.name).unwrap().trim();
                // println!("{}: {}", name.len(), name);
                let ext = from_utf8(&self.ext).unwrap().trim();
                if ext.len() > 0 {
                        name += ".";
                        name += ext;
                }
                return name;
        }
        
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
                        print!("\t");
                        let chain = self.get_chain();
                        if chain.len() == 0 {
                                print!("(null)");
                        } else {
                                for i in 0..(chain.len() - 1) {
                                        print!("{}->", chain[i]);
                                }
                                print!("{}", chain[chain.len()-1]);
                        }
                }
                println!();
        }
}