//! Directory entry
use core::str::from_utf8;
use core::mem::size_of;
use alloc::vec::Vec;
use alloc::string::String;
use super::chain::Chain;

use crate::process::ErrNo;

/// Directory Entry in raw
#[derive(Clone, Copy)]
#[repr(C, packed(1))]
pub struct DirEntryRaw {
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

impl DirEntryRaw {
        const ATTR_RDWR:u8 = 0x00;
        const ATTR_RDONLY:u8 = 0x01;
        const ATTR_HIDDEN:u8 = 0x02;
        const ATTR_SYS: u8 = 0x04;
        const ATTR_VOL: u8 = 0x08;
        pub const ATTR_SUBDIR: u8 = 0x10;
        pub const ATTR_FILE: u8 = 0x20;
        pub const ATTR_SYM: u8 = 0x80;
        const ATTR_LFN: u8 = 0x0f;

        /// Create a blank directory entry
        pub fn blank() -> DirEntryRaw {
                DirEntryRaw {
                        name: [0u8; 8],
                        ext: [0u8; 3],
                        attr: 0,
                        reserved: 0,
                        created_minisec: 0,
                        created_sec: 0,
                        created_date: 0,
                        accessed_sec: 0,
                        start_h: 0,
                        mod_sec: 0,
                        mod_date: 0,
                        start_l: 0,
                        size: 0,
                }
        }

        /// If the entry is deleted
        #[inline]
        pub fn is_deleted(&self) -> bool {
                return self.name[0] == 0xE5;
        }

        /// If the entry is an extension entry for long file name implement
        #[inline]
        pub fn is_ext(&self) -> bool {
                return self.attr & DirEntryRaw::ATTR_LFN == DirEntryRaw::ATTR_LFN;
        }

        /// If the entry is a symbolic link
        /// # Note
        /// To implement symbolic link in Fat32, we used a reserved bit in attribute
        /// byte to indicate whether is a symbolic link or not.
        #[inline]
        pub fn is_link(&self) -> bool {
                return self.attr & DirEntryRaw::ATTR_SYM == DirEntryRaw::ATTR_SYM;
        }

        /// If the entry is an entry for a directory
        #[inline]
        pub fn is_dir(&self) -> bool {
                return self.attr & DirEntryRaw::ATTR_SUBDIR == DirEntryRaw::ATTR_SUBDIR;
        }

        /// If the entry is an entry for a regular file
        #[inline]
        pub fn is_file(&self) -> bool {
                return self.attr & DirEntryRaw::ATTR_FILE == DirEntryRaw::ATTR_FILE;
        }

        /// If the read-only bit in attribute is set
        #[inline]
        pub fn is_read_only(&self) -> bool {
                return self.attr & DirEntryRaw::ATTR_RDONLY == DirEntryRaw::ATTR_RDONLY;
        }

        /// If the volumn bit in attribute is set
        #[inline]
        pub fn is_vol(&self) -> bool {
                return self.attr == DirEntryRaw::ATTR_VOL;
        }

        /// Get the start cluster of the file chain
        pub fn get_start(&self) -> u32 {
                let mut start = self.start_h as u32;
                start <<= 16;
                start |= self.start_l as u32;
                return start; 
        }

        /// Set the start cluster of the file chain
        pub fn set_start(&mut self, start: u32) {
                self.start_h = (start >> 16) as u16;
                self.start_l = (start & 0xff) as u16;
        }

        /// Get short file name
        pub fn get_name(&self) -> String {
                let mut name = String::new();
                debug!("name: {:x} {:x} {:x} {:x} {:x} {:x} {:x} {:x}", 
                        self.name[0],
                        self.name[1],
                        self.name[2],
                        self.name[3],
                        self.name[4],
                        self.name[5],
                        self.name[6],
                        self.name[7]
                );
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

        /// Set short file name
        pub fn set_name(&mut self, name: &str) {
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

        /// Get check sum for extension entries
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

        /// Print some fields in the entry
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
                unsafe{
                        println!("attr:{}", self.attr);
                        println!("start_h:{}", self.start_h);
                        println!("start_l:{}", self.start_l);
                        println!("size: {}", self.size);
                        println!("-------");
                }
        }
        
        /// Print entry
        pub fn print(&self) {
                if self.is_deleted() {
                        print!("deleted: ");
                }
                if self.is_ext() {
                        print!("Entry for long file name");
                } else {
                        // print!("{}.{}\t", from_utf8(&self.name).unwrap(), from_utf8(&self.ext).unwrap());
                        print!("{:16}", self.get_name());
                        unsafe{ print!("{:#10} ", self.size) };
                        if self.attr & DirEntryRaw::ATTR_RDONLY != 0 {
                                print!("R");
                        }
                        if self.attr & DirEntryRaw::ATTR_HIDDEN != 0 {
                                print!("H");
                        }
                        if self.attr & DirEntryRaw::ATTR_SYM != 0 {
                                print!("S");
                        }
                        if self.attr & DirEntryRaw::ATTR_VOL != 0 {
                                print!("V");
                        }
                        if self.attr & DirEntryRaw::ATTR_SUBDIR != 0 {
                                print!("D");
                        }
                        if self.attr & DirEntryRaw::ATTR_FILE != 0 {
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

/// Extension entry for long file name
/// # Note
/// More than one extension entry may be needed for a long file name,
/// so the extension entries usually work in groups (Vec::<DirEntryRaw>).
#[derive(Clone, Copy)]
#[repr(C, packed(1))]
pub struct DirEntryExtRaw {
        pub ext_attr: u8,
        pub name0: [u8; 10],
        pub attr: u8,
        pub reserved: u8,
        pub chksum: u8,
        pub name1: [u8; 12],
        pub start: u16,
        pub name2: [u8; 4],
}

impl DirEntryExtRaw {
        const EXT_END: u8 = 0x40;

        /// If the entry is deleted
        pub fn is_deleted(&self) -> bool{
                return self.ext_attr == 0xE5;
        }

        /// Create a group of extension entries from a given filename
        pub fn new(name: &str, chksum: u8) -> Vec<DirEntryExtRaw> {
                let mut result = Vec::<DirEntryExtRaw>::new();
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
                        let dex = DirEntryExtRaw{
                                ext_attr: (i+1) as u8,
                                name0,
                                attr: DirEntryRaw::ATTR_LFN,
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
                result[last].ext_attr |= DirEntryExtRaw::EXT_END;
                let mut rresult = Vec::<DirEntryExtRaw>::new();
                while result.len() > 0 {
                        rresult.push(result.pop().unwrap());
                }
                // debug!("DirEntryExtRaw::new return with len{} {:x}", result.len(), result[last].ext_attr);
                return rresult;
        }

        /// If the entry the last one in a extension entry group
        #[inline]
        pub fn is_end(&self) -> bool {
                return self.ext_attr & DirEntryExtRaw::EXT_END == DirEntryExtRaw::EXT_END;
        }

        /// Get index in group of the extension entry
        #[inline]
        pub fn get_index(&self) -> u8 {
                return self.ext_attr & !DirEntryExtRaw::EXT_END;
        }

        /// If the entry is a extension entry
        #[inline]
        pub fn is_ext(&self) -> bool {
                return self.attr == DirEntryRaw::ATTR_LFN;
        }

        /// Get the part of name that the entry holds
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

/// Treat "buf" as an entry and tell if it is a extentsion entry
fn is_ext(buf: &[u8; size_of::<DirEntryRaw>()]) -> bool {
        (buf[11] & DirEntryRaw::ATTR_LFN) == DirEntryRaw::ATTR_LFN
}

/// Treat "buf" as an extension and tell if it is deleted
fn is_del(buf: &[u8; size_of::<DirEntryRaw>()]) -> bool {
        buf[0] == 0xE5
}

/// Group a entry and the group of extension entries that serve the entry.
#[derive(Clone)]
pub struct DirEntryGroup {
        exts: Vec<DirEntryExtRaw>,
        pub entry: DirEntryRaw,
        offset: usize,
        slotsize: usize,
}

impl DirEntryGroup {
        /// Create virtual entry group for root directory
        /// # Note
        /// No entry for root directory since root is not in a directory,
        /// so we need to fake one for convenience.
        pub fn root() -> DirEntryGroup {
                DirEntryGroup {
                        exts: Vec::<DirEntryExtRaw>::new(),
                        entry: DirEntryRaw {
                                name: [0u8; 8],
                                ext: [0u8; 3],
                                attr: DirEntryRaw::ATTR_SUBDIR,
                                reserved: 0u8,
                                created_minisec: 0u8,
                                created_sec: 0u16,
                                created_date: 0u16,
                                accessed_sec: 0u16,
                                start_h: 0u16,
                                mod_sec: 0u16,
                                mod_date: 0u16,
                                start_l: 0u16,
                                size: 0u32,
                        },
                        offset: 0,
                        slotsize: 0,
                }
        }

        /// Create a entry group from given infos
        pub fn new(name: &str, start: u32, attr: u8) -> DirEntryGroup {
                let mut entry = DirEntryRaw::blank();
                entry.attr = attr;
                entry.set_name(name);
                entry.set_start(start);
                let exts = DirEntryExtRaw::new(name, entry.chksum());
                return DirEntryGroup {entry, exts, offset: 0, slotsize:0 };
        }

        /// Change the filename that the entries hold
        pub fn rename(&mut self, name: &str) -> Result<(), ()> {
                self.entry.set_name(name);
                self.exts = DirEntryExtRaw::new(name, self.entry.chksum());
                return Ok(());
        }

        /// If the entry group refer to "."
        #[inline]
        pub fn is_cur(&self) -> bool {
                return self.entry.name[0] == '.' as u8 && self.entry.name[1] == ' ' as u8;
        }

        /// If the entry group refer to ".."
        #[inline]
        pub fn is_par(&self) -> bool {
                return self.entry.name[0] == '.' as u8 
                && self.entry.name[1] == '.' as u8 
                && self.entry.name[2] == ' ' as u8;
        }

        /// Get the filename that the entries hold
        pub fn get_name(&self) -> Result<String, &'static str> {
                let mut name = Vec::<u8>::new();
                if self.exts.len() > 0 {
                        for i in (0..self.exts.len()).rev() {
                                name.append(&mut self.exts[i].get_name());
                                if self.exts[i].is_end() {
                                        if i != 0 {
                                                return Err("get_name: end not end?");
                                        }
                                        let nlen = name.len() >> 1;
                                        let mut n = Vec::<u16>::with_capacity(nlen);
                                        for j in 0..nlen {
                                                let tmp:u16 = name[2*j+1] as u16;
                                                let tmp:u16 = (tmp << 8) | (name[2*j] as u16);
                                                if tmp == 0 {
                                                        break;
                                                }
                                                n.push(tmp);
                                        }
                                        let name = String::from_utf16(&n).unwrap();
                                        return Ok(name);
                                }
                        } 

                        return Err("get_name: missing end for lfn");
                } else {
                        return Ok(self.entry.get_name());
                }
        }

        /// Get the starting cluster of the file chain
        pub fn get_start(&self) -> u32{
                return self.entry.get_start();
        }
}

/// If a directory is empty
/// # Description
/// "chain" is the file chain of the directory
pub fn empty_dir(chain: &Chain) -> bool {
        let mut offset = 0;
        loop {
                match read_dirent_group(&chain, offset) {
                        Ok((group, next)) => {
                                if group.is_cur() || group.is_par() {
                                        offset = next;
                                } else {
                                        return false;
                                }
                        },
                        Err(_) => return true,
                }
        } 
}

/// Get a group from the offset in "chain"
/// # Description
/// "chain" is a file chain of a directory
/// # Return
/// On success, returns the entry group and the offset to look for next group in the chain.
/// Returns error message otherwise.
pub fn read_dirent_group(chain: &Chain, offset: usize) -> Result<(DirEntryGroup, usize), ErrNo> {
        let mut exts = Vec::<DirEntryExtRaw>::new();
        let mut buf = [0u8; size_of::<DirEntryRaw>()];
        let mut off = offset;
        let mut slotsize = 0;
        loop {
                match chain.read(off, &mut buf) {
                        Ok(rlen) => {
                                if rlen != size_of::<DirEntryRaw>() {
                                        return Err(ErrNo::Fat32EntryShortRead);
                                } 
                        },
                        Err(errno) => {
                                return Err(errno);
                        }
                }
                slotsize += 1;
                off += size_of::<DirEntryExtRaw>();
                if is_del(&buf) {
                        continue;
                }
                if !is_ext(&buf) {
                        break; 
                }
                unsafe {
                        let ext = *((&buf as *const _) as *const DirEntryExtRaw).clone();
                        exts.push(ext);
                }
        }
        
        if buf[0] == 0 {
                return Err(ErrNo::Fat32NoMoreEntry);
        }
        unsafe {
                let entry = *((&buf as *const _) as *const DirEntryRaw).clone();
                return Ok((DirEntryGroup {
                                exts, 
                                entry,
                                offset,
                                slotsize,
                        }, off));
        }
}

/// Write a group into "chain"
/// # Description
/// "chain" is a file chain of a directory
/// write_dirent_group will try to update the entries in chain first.
/// If update failed (for example, filename gets longer or group not exist in the chain),
/// it wirte new entried at the end of the chain, and delete the old ones (if there are). 
pub fn write_dirent_group (chain: &mut Chain, group: &mut DirEntryGroup) -> Result<(),()> {
        if group.slotsize == 0 {
                let mut offset = 0;
                let mut slotsize = 0;
                loop {
                        let mut b = [0u8];
                        match chain.read(offset, &mut b) {
                                Ok(_rlen) => if b[0] == 0 {break}
                                            else if b[0] == 0xE5 {slotsize += 1}
                                            else {slotsize = 0},
                                Err(_msg) => break,
                        }
                        offset += size_of::<DirEntryRaw>();
                }
                group.offset = offset;
                for ext in &group.exts {
                        unsafe {
                                // let buf = core::slice::from_raw_parts((ext as *const DirEntryExtRaw) as *const u8, size_of::<DirEntryExtRaw>());
                                let buf = &*(ext as *const _ as *const [u8; size_of::<DirEntryExtRaw>()]);
                                chain.write(offset, buf).unwrap();
                        } 
                        offset += size_of::<DirEntryExtRaw>();
                }
                unsafe {
                        let buf = &*((&group.entry as *const _) as *const [u8; size_of::<DirEntryRaw>()]);
                        chain.write(offset, buf).unwrap();
                }
                group.slotsize = group.exts.len() + 1 + slotsize;
                return Ok(());
        } else if group.slotsize < group.exts.len() + 1 {
                let offset = group.offset;
                group.slotsize = 0;
                match write_dirent_group(chain, group) {
                        Ok(()) => {
                                delete_dirent_group(chain, offset).unwrap();
                                return Ok(());
                        },
                        Err(msg) => {
                                return Err(msg);
                        }
                }
        } else {
                let mut offset = group.offset + (group.slotsize - group.exts.len() - 1) * size_of::<DirEntryExtRaw>();
                for ext in &group.exts {
                        unsafe {
                                let buf = &*((ext as *const _) as *const [u8; size_of::<DirEntryExtRaw>()]);
                                chain.write(offset, buf).unwrap();
                        } 
                        offset += size_of::<DirEntryExtRaw>();
                }
                unsafe {
                        let buf = &*((&group.entry as *const _) as *const [u8; size_of::<DirEntryRaw>()]);
                        chain.write(offset, buf).unwrap();
                }
                return Ok(());        
        }
}

/// Mark the entries in chain as deleted
pub fn delete_dirent_group(chain: &mut Chain, offset: usize) -> Result<(), ErrNo>{
        let mut buf = [0u8; size_of::<DirEntryRaw>()];
        let mut off = offset;
        loop {
                match chain.read(off, &mut buf) {
                        Ok(rlen) => {
                                if rlen != size_of::<DirEntryRaw>() {
                                        return Err(ErrNo::Fat32EntryShortRead);
                                } 
                        },
                        Err(errno) => {
                                return Err(errno);
                        }
                }
                if buf[0] == 0 {
                        return Err(ErrNo::NoSuchFileOrDirectory);
                }        
                if is_del(&buf) {
                        off += size_of::<DirEntryExtRaw>();
                        // return Ok(());
                        continue;
                }
                buf[0] = 0xE5;
                chain.write(off, &buf).unwrap();
                off += size_of::<DirEntryExtRaw>();
                if !is_ext(&buf) {
                        return Ok(());    
                }
        }
}