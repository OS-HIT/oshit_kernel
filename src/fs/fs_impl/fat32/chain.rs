//! File chain of Fat32
use super::Fat32FS;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::sync::Arc;

use crate::process::ErrNo;

/// File Chain of Fat32
#[derive(Clone)]
pub struct Chain {
        pub fs: Arc<Fat32FS>,
        pub chain: Vec<u32>,
}

impl Chain {
        const MAX_LEN:usize = 1024 * 1024;

        /// Get the file chain of root directory
        pub fn root(fs: Arc<Fat32FS>) -> Result<Chain, &'static str> {
                fs.dbr.root;
                let chain = fs.get_chain(fs.dbr.root);
                return Ok( Chain {fs: fs.clone(), chain} );
                // return Err("error when reading root");
        }
        
        /// Create a empty file chain
        pub fn new(fs: Arc<Fat32FS>, chain: Vec<u32>) -> Chain {
                Chain {fs, chain}
        }

        fn get_cluster(&self, offset: usize) -> Result<(usize,u32), ErrNo> {
                let n = offset / self.fs.cluster_size();
                if n >= self.chain.len() {
                        error!("chain.len(): {} offset: {}", self.chain.len(), offset);
                        return Err(ErrNo::Fat32InvalidOffset);
                } else {
                        return Ok((n,self.chain[n]));
                }
        }

        /// Fill the buffer with contents in file chain at "offset"
        /// # Return
        /// Number of bytes that actually read
        pub fn read(&self, offset: usize, buffer: &mut [u8]) -> Result<usize, ErrNo> {
                let (mut idx,clst) = self.get_cluster(offset)?;
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

        /// Write the contents of the buffer into the file chain at "offset"
        /// # Description
        /// Chain append will be performed when necessary. 
        /// If "offset" is bigger than the offset of the last byte in chain, space between them will be filled with 0.
        /// # Return
        /// Number of bytes that actually written
        pub fn write(&mut self, offset: usize, buffer: &[u8]) -> Result<usize, ErrNo> {
                // error!("who is calling the write?");
                let (mut idx, clst) = loop {
                        match self.get_cluster(offset) {
                                Ok(c) => break c,
                                Err(_msg) => {
                                        if self.chain.len() < Chain::MAX_LEN {
                                                let new = if self.chain.len() == 0 {
                                                        self.fs.alloc_cluster().unwrap()
                                                } else {
                                                        self.fs.append_chain(*self.chain.last().unwrap()).unwrap()
                                                };
                                                self.chain.push(new);
                                        } else {
                                                return Err(ErrNo::InvalidArgument);
                                        }
                                },
                        }
                };
                let coff = offset % self.fs.cluster_size();
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
                                        if self.chain.len() < Chain::MAX_LEN {
                                                let new = if self.chain.len() == 0 {
                                                        self.fs.alloc_cluster().unwrap()
                                                } else {
                                                        self.fs.append_chain(*self.chain.last().unwrap()).unwrap()
                                                };
                                                self.chain.push(new);
                                                write += self.fs.write_cluster(new, 0, buf).unwrap();
                                        } else {
                                                return Ok(write);
                                        }
                                }
                        }
                }
                return Ok(write);
        }

        /// Trucate chain to the specified length
        pub fn truncate(&mut self, len: usize) -> Result<(), ()> {
                if self.chain.len() > len {
                        self.fs.truncate_chain(self.chain[len-1]).unwrap();
                        self.chain.truncate(len);
                }
                return Ok(());
        }

        /// Convert the chain to string for printing
        pub fn to_string(&self, max: isize) -> String {
                if self.chain.len() == 0 {
                        return String::from("(null)");
                } else {
                        let mut s = String::new();
                        let max = if max == -1 {
                                self.chain.len()
                        } else if max as usize > self.chain.len() {
                                self.chain.len()
                        } else {
                                max as usize
                        };
                        for i in 0..max {
                                s += &self.chain[i].to_string();
                                s.push('-');
                        }
                        s.push('|');
                        return s;
                }
        }
}
