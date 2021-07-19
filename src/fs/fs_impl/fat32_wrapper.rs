use alloc::sync::Arc;
use alloc::string::String;
use spin::Mutex;

use super::BlockDeviceFile;
use super::cache_mgr::BLOCK_SZ;
use super::devfs::CommonFileAsBlockDevice;
use super::fat32;
use super::fat32::Fat32FS;
use super::fat32::wrapper::FAT32File;

use super::vfs::*;
use super::utils::*;

use crate::fs::File;

pub struct Fat32W {
        pub inner: Arc<Fat32FS>,
}

impl Fat32W {
        pub fn new(blk: Arc<dyn File>) -> Option<Self>{
                if let Some(dev) = blk.clone().to_device_file() {
                        if let Some(blk_dev) = dev.to_blk_dev() {
                                Some( Self {
                                        inner: Arc::new(Fat32FS::openFat32(blk_dev)),
                                })
                        } else {
                                None
                        }
                } else {
                        Some( Self{
                                inner: Arc::new(Fat32FS::openFat32(Arc::new(CommonFileAsBlockDevice::new(blk.clone(), BLOCK_SZ))))
                        })
                }
        }
}

impl VirtualFileSystem for Fat32W {
        /// force write back all dirty
        fn sync(&self, wait: bool) {
                self.inner.sync();
        }

        /// get status
        fn get_status(&self) -> FSStatus {
                return FSStatus {
                        name: Fat32FS::name,
                        flags: FSFlags::empty(),
                }
        }

        // ==================== file level ops ====================
        /// create inode (read from disc etc), used for open files.  
        /// we first create it's inode, then opens it.
        /// todo: maybe a specific Path struct?
        fn open(&self, abs_path: String, mode: OpenMode) -> Result<Arc<dyn File>, &'static str> {
                let mode = OpenMode2usize(mode);
                match fat32::open(self.inner.clone(), &abs_path, mode){
                        Ok(file) => return Ok(Arc::new(
                                FAT32File {
                                        inner: Mutex::new(file)
                                }
                        )),
                        Err(msg) => return Err(msg),
                };
        }

        fn mkdir(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str> {
                match fat32::mkdir(self.inner.clone(), &abs_path) {
                        Ok(file) => return Ok(Arc::new(
                                FAT32File {
                                        inner: Mutex::new(file)
                                }
                        )),
                        Err(msg) => return Err(msg),
                }
        }

        fn mkfile(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str> {
                match fat32::mkfile(self.inner.clone(), &abs_path) {
                        Ok(file) => return Ok(Arc::new(
                                FAT32File {
                                        inner: Mutex::new(file)
                                }
                        )),
                        Err(msg) => return Err(msg),
                }
        }

        fn remove(&self, abs_path: String) -> Result<(), &'static str> {
                return fat32::remove(self.inner.clone(), &abs_path);
        }
        
        fn link(&self, to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str> {
                return Err("Not supported by fat32");
        }

        fn sym_link(&self, abs_src: String, rel_dst: String) -> Result<(), &'static str> {
                return fat32::sym_link(self.inner.clone(), &rel_dst, &abs_src);
        }

        fn rename(&self, to_rename: Arc<dyn File>, new_name: String) -> Result<(), &'static str> {
                return Err("???????");
        }
}