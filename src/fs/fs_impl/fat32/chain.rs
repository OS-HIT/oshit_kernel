use super::Fat32FS;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;

#[derive(Clone)]
pub struct Chain {
        pub fs: Arc<Fat32FS>,
        pub chain: Vec<u32>,
}

impl Chain {
        const max_len:usize = 1024 * 1024;

        pub fn root(fs: Arc<Fat32FS>) -> Result<Chain, &'static str> {
                fs.dbr.root;
                let chain = fs.get_chain(fs.dbr.root);
                return Ok( Chain {fs: fs.clone(), chain} );
                // return Err("error when reading root");
        }
        
        pub fn new(fs: Arc<Fat32FS>, chain: Vec<u32>) -> Chain {
                Chain {fs, chain}
        }

        fn get_cluster(&self, offset: usize) -> Result<(usize,u32), &'static str> {
                let n = offset / self.fs.cluster_size();
                if n >= self.chain.len() {
                        return Err("Chain::get_cluster: invalid offset\n");
                } else {
                        return Ok((n,self.chain[n]));
                }
        }

        pub fn read(&self, offset: usize, buffer: &mut [u8]) -> Result<usize, &'static str> {
                let (mut idx,clst) = match self.get_cluster(offset) {
                        Ok(c) => c,
                        Err(msg) => return Err(msg),
                };
                let coff = offset % self.fs.cluster_size();
                let len = buffer.len();
                let mut read = self.fs.read_cluster(clst, coff, buffer).unwrap();
                while read < len {
                        let buf = &mut buffer[read..];
                        idx +=1 ;
                        match self.chain.get(idx) {
                                Some(clst) => {
                                        read += self.fs.read_cluster(*clst, 0, buf).unwrap();
                                },
                                None => {
                                        return Ok(read);
                                }
                        } 
                }
                return Ok(read);
        }

        pub fn write(&mut self, offset: usize, buffer: &[u8]) -> Result<usize, &'static str> {
                let (mut idx, clst) = loop {
                        match self.get_cluster(offset) {
                                Ok(c) => break c,
                                Err(_msg) => {
                                        if self.chain.len() < Chain::max_len {
                                                let new = self.fs.append_chain(*self.chain.last().unwrap()).unwrap();
                                                self.chain.push(new);
                                        } else {
                                                return Err("Chain::write: invalid offset\n");
                                        }
                                },
                        }
                };
                let coff = offset & self.fs.cluster_size();
                let len = buffer.len();
                let mut write = self.fs.write_cluster(clst, coff, buffer).unwrap();
                while write < len {
                        let buf = &buffer[write..];
                        idx += 1;
                        match self.chain.get(idx) {
                                Some(clst) => {
                                        write += self.fs.write_cluster(*clst, 0, buf).unwrap();
                                },
                                None => {
                                        return Ok(write);
                                }
                        }
                }
                return Ok(write);
        }

        pub fn to_string(&self) -> String {
                if self.chain.len() == 0 {
                        return String::from("(null)");
                } else {
                        let mut s = String::new();
                        for c in &self.chain {
                                s += &c.to_string();
                                s.push('-');
                        }
                        s.push('|');
                        return s;
                }
        }
}
