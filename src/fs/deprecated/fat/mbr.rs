
use super::super::block_cache::get_block_cache;
use lazy_static::*;


pub struct Partition {
        pub id: u8,
        pub start: u32,
        pub len: u32,
}

pub struct MBR {
        pub par_tab: [Partition; 4],
}

fn b2u32(b: &[u8; 4]) -> u32 {
        b[0] as u32 
        | ((b[1] as u32) << 8)
        | ((b[2] as u32) << 16)
        | ((b[3] as u32) << 24)
}

impl MBR {
        pub fn get_mbr() -> MBR {
                let cache = get_block_cache(0);
                let id = *cache.lock().get_ref::<u8>(0x1c2);
                let start = *cache.lock().get_ref::<[u8;4]>(0x1c6);
                let start = b2u32(&start);
                let len = *cache.lock().get_ref::<[u8;4]>(0x1ca);
                let len = b2u32(&len);
                let part0 = Partition {id, start, len,};

                let id = *cache.lock().get_ref::<u8>(0x1d2);
                let start = *cache.lock().get_ref::<[u8;4]>(0x1d6);
                let start = b2u32(&start);
                let len = *cache.lock().get_ref::<[u8;4]>(0x1da);
                let len = b2u32(&len);
                let part1 = Partition {id, start, len,};

                let id = *cache.lock().get_ref::<u8>(0x1e2);
                let start = *cache.lock().get_ref::<[u8;4]>(0x1e6);
                let start = b2u32(&start);
                let len = *cache.lock().get_ref::<[u8;4]>(0x1ea);
                let len = b2u32(&len);
                let part2 = Partition {id, start, len,};

                let id = *cache.lock().get_ref::<u8>(0x1f2);
                let start = *cache.lock().get_ref::<[u8;4]>(0x1f6);
                let start = b2u32(&start);
                let len = *cache.lock().get_ref::<[u8;4]>(0x1fa);
                let len = b2u32(&len);
                let part3 = Partition {id, start, len,};
                MBR {
                        par_tab: [part0, part1, part2, part3],
                }
        }

        pub fn print(&self) {
                print!("-----MBR------\n");
                for i in 0..4 {
                        println!("{}: id {:#02X} start {} len {}", i, self.par_tab[i].id, self.par_tab[i].start, self.par_tab[i].len);
                }
                println!();
        }
}

lazy_static! {
        pub static ref MBR_INST: MBR = MBR::get_mbr();
}