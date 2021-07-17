
use core::str::from_utf8;
use core::mem::size_of;
use alloc::vec::Vec;
use alloc::string::String;
use super::chain::Chain;

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
        const ATTR_SYM: u8 = 0x04;
        const ATTR_VOL: u8 = 0x08;
        pub const ATTR_SUBDIR: u8 = 0x10;
        pub const ATTR_FILE: u8 = 0x20;
        const ATTR_LFN: u8 = 0x0f;

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

        #[inline]
        pub fn is_deleted(&self) -> bool {
                return self.name[0] == 0xE5;
        }

        #[inline]
        pub fn is_ext(&self) -> bool {
                return self.attr & DirEntryRaw::ATTR_LFN == DirEntryRaw::ATTR_LFN;
        }

        #[inline]
        pub fn is_dir(&self) -> bool {
                return self.attr & DirEntryRaw::ATTR_SUBDIR == DirEntryRaw::ATTR_SUBDIR;
        }

        #[inline]
        pub fn is_file(&self) -> bool {
                return self.attr & DirEntryRaw::ATTR_FILE == DirEntryRaw::ATTR_FILE;
        }

        #[inline]
        pub fn is_read_only(&self) -> bool {
                return self.attr & DirEntryRaw::ATTR_RDONLY == DirEntryRaw::ATTR_RDONLY;
        }

        #[inline]
        pub fn is_vol(&self) -> bool {
                return self.attr == DirEntryRaw::ATTR_VOL;
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

        pub fn is_deleted(&self) -> bool{
                return self.ext_attr == 0xE5;
        }

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
                // debug!("DirEntryExtRaw::new return with len{} {:x}", result.len(), result[last].ext_attr);
                return result;
        }

        #[inline]
        pub fn is_end(&self) -> bool {
                return self.ext_attr & DirEntryExtRaw::EXT_END == DirEntryExtRaw::EXT_END;
        }

        #[inline]
        pub fn get_index(&self) -> u8 {
                return self.ext_attr & !DirEntryExtRaw::EXT_END;
        }

        #[inline]
        pub fn is_ext(&self) -> bool {
                return self.attr == DirEntryRaw::ATTR_LFN;
        }

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

fn is_ext(buf: &[u8; size_of::<DirEntryRaw>()]) -> bool {
        (buf[11] & DirEntryRaw::ATTR_LFN) == DirEntryRaw::ATTR_LFN
}

fn is_del(buf: &[u8; size_of::<DirEntryRaw>()]) -> bool {
        buf[0] == 0xE5
}

#[derive(Clone)]
pub struct DirEntryGroup {
        exts: Vec<DirEntryExtRaw>,
        pub entry: DirEntryRaw,
}

impl DirEntryGroup {
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
                        }
                }
        }

        pub fn new(name: &str, start: u32, attr: u8) -> DirEntryGroup {
                let mut entry = DirEntryRaw::blank();
                entry.attr = attr;
                entry.set_name(name);
                entry.set_start(start);
                let exts = DirEntryExtRaw::new(name, entry.chksum());
                return DirEntryGroup {entry, exts};
        }

        #[inline]
        pub fn is_cur(&self) -> bool {
                return self.entry.name[0] == '.' as u8 && self.entry.name[1] == ' ' as u8;
        }

        #[inline]
        pub fn is_par(&self) -> bool {
                return self.entry.name[0] == '.' as u8 
                && self.entry.name[1] == '.' as u8 
                && self.entry.name[2] == ' ' as u8;
        }

        pub fn get_name(&self) -> Result<String, &'static str> {
                let mut name = Vec::<u8>::new();
                if self.exts.len() > 0 {
                        for i in (0..self.exts.len()).rev() {
                                name.append(&mut self.exts[i].get_name());
                                if self.exts[i].is_end() {
                                        if i != 0 {
                                                return Err("get_full_name: end not end?");
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

                        return Err("get_full_name: missing end for lfn");
                } else {
                        return Ok(self.entry.get_name());
                }
        }

        pub fn get_start(&self) -> u32{
                return self.entry.get_start();
        }
}

pub fn read_dirent_group(chain: &Chain, offset: usize) -> Result<(DirEntryGroup, usize), &'static str> {
        let mut exts = Vec::<DirEntryExtRaw>::new();
        let mut buf = [0u8; size_of::<DirEntryRaw>()];
        let mut off = offset;
        loop {
                match chain.read(off, &mut buf) {
                        Ok(rlen) => {
                                if rlen != size_of::<DirEntryRaw>() {
                                        return Err("read_dirent_group: short read");
                                } 
                        },
                        Err(msg) => {
                                return Err(msg);
                        }
                }
                off += size_of::<DirEntryExtRaw>();
                if is_del(&buf) {
                        continue;
                }
                if !is_ext(&buf) {
                        break; 
                }
                unsafe {
                        let ext = *((&buf as *const _) as *const DirEntryExtRaw).clone();
                        if ext.is_deleted() {
                                continue;
                        }
                        exts.push(ext);
                }
        }
        
        if buf[0] == 0 {
                return Err("read_dirent_group: Invalid(Empty) DirEntry");
        }
        unsafe {
                let entry = *((&buf as *const _) as *const DirEntryRaw).clone();
                return Ok((DirEntryGroup {exts, entry}, off));
        }
}

pub fn write_dirent_group (chain: &mut Chain, group: &DirEntryGroup) -> Result<(),()> {
        let mut offset = 0;
        loop {
                let mut b = [0u8];
                match chain.read(offset, &mut b) {
                        Ok(rlen) => if b[0] == 0 {break},
                        Err(msg) => break,
                }
                offset += size_of::<DirEntryRaw>();
        }
        for ext in &group.exts {
                unsafe {
                        let buf = &*((&ext as *const _) as *const [u8; size_of::<DirEntryExtRaw>()]);
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