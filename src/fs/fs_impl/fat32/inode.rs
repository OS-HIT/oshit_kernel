use super::super::super::path::Path;
use super::Fat32FS;
use super::chain::Chain;
use super::dirent::DirEntryRaw;
use super::dirent::DirEntryGroup;
use super::dirent::read_dirent_group;
use super::dirent::write_dirent_group;
use super::dirent::empty_dir;
use super::dirent::delete_dirent_group;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;

#[derive(Clone)]
pub struct Inode {
        pub name: String,
        pub path: Path,
        pub group: DirEntryGroup,
        pub chain: Chain,
}

impl Inode {
        pub fn root(fs: Arc<Fat32FS>) -> Inode {
                Inode {
                        chain: Chain::root(fs).unwrap(),
                        path: Path::root(),
                        group: DirEntryGroup::root(),
                        name: String::from(""),
                }
        }

        #[inline]
        pub fn is_link(&self) -> bool {
                return self.group.entry.is_link();
        }

        #[inline]
        pub fn is_dir(&self) -> bool {
                return self.group.entry.is_dir();
        }

        #[inline]
        pub fn is_cur(&self) -> bool {
                return self.group.is_cur();
        }

        #[inline]
        pub fn is_par(&self) -> bool {
                return self.group.is_par();
        }

        #[inline]
        pub fn is_slink(&self) -> bool {
                return self.group.entry.attr & DirEntryRaw::ATTR_SYM != 0;
        }

        pub fn get_size(&self) -> usize {
                return self.group.entry.size as usize;
        }

        pub fn set_size(&mut self, size: u32) {
                self.group.entry.size = size;
        }

        #[inline]
        pub fn is_fake(&self) -> bool {
                return self.is_cur() || self.is_par();
        }

        pub fn print(&self) {
                print!("name: {:16}", &self.name);
                print!("parent: {:32}", &self.path.to_string());
                println!("chain: {}", &self.chain.to_string());
                // print!("start: {}\n", &self.chain.chain[0]);
        }

        pub fn get_inodes(&self) -> Result<Vec<Inode>, &'static str> {
                if !self.group.entry.is_dir() {
                        return Err("get_inodes: not a directory");
                }
                let mut offset = 0;
                let mut inodes = Vec::<Inode>::new();
                loop {
                        match read_dirent_group(&self.chain, offset) {
                                Ok((group, next)) => {
                                        let c = Chain::new(self.chain.fs.clone(), self.chain.fs.get_chain(group.get_start()));
                                        let mut path = self.path.clone();
                                        if self.name.len() > 0 {
                                                path.push(self.name.clone(), true).unwrap();
                                        }
                                        inodes.push(
                                                Inode {
                                                        name: group.get_name().unwrap(),
                                                        path,
                                                        group: group,
                                                        chain: c,
                                                }
                                        );
                                        offset = next;
                                },
                                Err(_) => return Ok(inodes),
                        }

                }
        }

        pub fn find_inode(&self, name: &str) -> Result<Inode, &'static str> {
                if !self.group.entry.is_dir() {
                        return Err("get_inodes: not a directory");
                }
                let mut offset = 0;
                loop {
                        match read_dirent_group(&self.chain, offset) {
                                Ok((group, next)) => {
                                        let iname = group.get_name().unwrap();
                                        if name.eq(&iname) {
                                                let c = Chain::new(self.chain.fs.clone(), self.chain.fs.get_chain(group.get_start()));
                                                let mut p = self.path.clone();
                                                if self.name.len() > 0 {
                                                        p.push(self.name.clone(), true).unwrap();
                                                }
                                                return Ok(Inode {
                                                        name: group.get_name().unwrap(),
                                                        group: group,
                                                        path: p,
                                                        chain: c,
                                                });
                                        }
                                        offset = next;
                                },
                                Err(_) => return Err("find_inode: inode not found"),
                        }

                }
        }

        pub fn find_inode_path(&self, path: &Path) -> Result<Inode, &'static str> {
                if !self.is_dir() {
                        return Err("find_inode_path: not a directory");
                }
                if path.path.len() == 0 {
                        return Err("find_inode_path: empty path");
                }
                let mut cur = self;
                let mut i: Inode = self.clone();
                for p in &path.path {
                        i = match cur.find_inode(p) {
                                Ok(inode) => inode,
                                Err(msg) => return Err(msg),
                        };
                        cur = &i;
                }
                if path.must_dir && !cur.is_dir() {
                        return Err("find_inode_path: target not directory");
                }
                return Ok(i);
        }

        pub fn get_parent(&self) -> Result<Inode, &'static str> {
                let root = Inode::root(self.chain.fs.clone());
                if self.path.path.len() == 0 {
                        return Ok(root);
                } else {
                        return root.find_inode_path(&self.path);
                }
        }
        
        pub fn realize(&mut self) -> Result<Inode, &'static str> {
                if !self.is_cur() || !self.is_par() {
                        return Err("realize: not fake inode");
                }
                self.path.purge().map_err(|err| -> &str {"Path format error"})?;
                return Ok(Inode::root(self.chain.fs.clone()).find_inode_path(&self.path).unwrap());
        }

        pub fn new(&mut self, name: &str, chain: Chain, attr:u8) -> Result<Inode, &'static str> {
                if !self.is_dir() {
                        return Err("new: cannot new from none dir inode");
                }
                if self.is_fake() {
                        return Err("new: cannont new from fake inode");
                }
                let start = if chain.chain.len() == 0 {
                        0u32
                } else {
                        chain.chain[0]
                };
                let mut group = DirEntryGroup::new(name, start, attr);
                write_dirent_group(&mut self.chain, &mut group).unwrap();
                let mut path = self.path.clone();
                if self.name.len() > 0 {
                        path.push(self.name.clone(), true).unwrap();
                }
                let new = Inode {name: String::from(name), path, group, chain};
                return Ok(new);
        }

        // pub fn new_path(&mut self, path: &str, chain: Chain, attr: u8) -> Result<Inode, &'static str> {
                
        // }

        pub fn new_dir(&mut self, name: &str, attr:u8) -> Result<Inode, &'static str> {
                let attr = attr | DirEntryRaw::ATTR_SUBDIR;
                let mut chain = Vec::new();
                chain.push(self.chain.fs.alloc_cluster().unwrap());
                let chain = Chain::new(self.chain.fs.clone(), chain);
                let mut nd = match self.new(name, chain.clone(), attr) {
                        Ok(inode) => inode,
                        Err(msg) => {
                                self.chain.fs.clear_chain(chain.chain[0]).unwrap();
                                return Err(msg)
                        },
                };
                nd.new(".", chain, DirEntryRaw::ATTR_SUBDIR).unwrap();
                nd.new("..", self.chain.clone(), DirEntryRaw::ATTR_SUBDIR).unwrap();
                return Ok(nd);
        }

        pub fn new_file(&mut self, name: &str, attr:u8) -> Result<Inode, &'static str> {
                let attr = attr | DirEntryRaw::ATTR_FILE;
                let chain = Vec::new();
                let chain = Chain::new(self.chain.fs.clone(), chain);
                match self.new(name, chain, attr){
                        Ok(inode) => return Ok(inode),
                        Err(_) => return Err("new_file: create failed"),
                };
        }

        pub fn delete_inode(&mut self, name: &String) -> Result<(), &'static str> {
                if !self.group.entry.is_dir() {
                        return Err("delete_inodes: not a directory");
                }
                let mut offset = 0;
                loop {
                        match read_dirent_group(&self.chain, offset) {
                                Ok((group, next)) => {
                                        let iname = group.get_name().unwrap();
                                        if name.eq(&iname) {
                                                if group.entry.is_dir() {
                                                        let chain = self.chain.fs.get_chain(group.get_start());
                                                        let chain = Chain::new(self.chain.fs.clone(), chain);
                                                        if !empty_dir(&chain) {
                                                                return Err("delete_inode: dir not empty");
                                                        }
                                                } 
                                                self.chain.fs.clear_chain(group.get_start()).unwrap();
                                                delete_dirent_group(&mut self.chain, offset).unwrap();
                                                return Ok(());
                                        }
                                        offset = next;
                                },
                                Err(_) => return Err("delete_inode: inode not found"),
                        }

                }
        }
}
