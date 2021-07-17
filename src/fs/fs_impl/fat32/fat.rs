pub struct FAT {
        pub start: u32,
        pub end: u32,
        pub len: u32,
        pub clen: u32,
}

impl FAT {
        #[allow(unused)]
        pub fn print(&self) {
                println!("===========FAT==========");
                println!("start: {}", self.start);
                println!("end: {}", self.end);
                println!("len: {}", self.len);
                println!("len: {}", self.clen);
        }
}

#[derive(PartialEq, Debug)]
pub enum CLUSTER {
        Free,
        Temp,
        Data,
        Rsv, // reserved
        Bad, 
        Eoc, // End of chain
}

pub fn get_type(clst_num: u32) -> CLUSTER {
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
