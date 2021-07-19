use core::cmp::Ordering;

use super::super::VirtualFileSystem;
use alloc::{collections::BTreeMap, string::ToString};
use alloc::string::String;
use spin::{Mutex, MutexGuard};
use alloc::sync::Arc;
use crate::fs::{File, OpenMode};
use lazy_static::*;

pub struct MountManager {
    inner: Mutex<MountManagerInner>,
}
unsafe impl Sync for MountManager {}

impl MountManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MountManagerInner::new())
        }
    }

    pub fn get_inner_locked (&self) -> MutexGuard<MountManagerInner> {
        self.inner.lock()
    }

    pub fn mount_fs(&self, path: String, vfs: Arc<dyn VirtualFileSystem>) -> Result<(), &'static str> {
        self.get_inner_locked().mount_fs(path, vfs)
    }

    pub fn unmount_fs(&self, path: String) -> Result<(), &'static str> {
        self.get_inner_locked().unmount_fs(path)
    }

    /// get vfs and string relative to it.
    pub fn parse(&self, total_path: String) -> Result<(Arc<dyn VirtualFileSystem>, String), &'static str> {
        self.get_inner_locked().parse(total_path)
    }
    
    pub fn open(&self, abs_path: String, mode: OpenMode) -> Result<Arc<dyn File>, &'static str> {
        self.get_inner_locked().open(abs_path, mode)
    }

    pub fn mkdir(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str> {
        self.get_inner_locked().mkdir(abs_path)
    }

    pub fn mkfile(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str> {
        self.get_inner_locked().mkfile(abs_path)
    }

    pub fn remove(&self, abs_path: String) -> Result<(), &'static str> {
        self.get_inner_locked().remove(abs_path)
    }
    
    pub fn link(&self, to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str> {
        self.get_inner_locked().link(to_link, dest)
    }

    pub fn sym_link(&self, to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str> {
        self.get_inner_locked().sym_link(to_link, dest)
    }

    pub fn rename(&self, to_rename: Arc<dyn File>, new_name: String) -> Result<(), &'static str> {
        self.get_inner_locked().rename(to_rename, new_name)
    }
}

pub struct MountManagerInner {
    mounted_fs: BTreeMap<String, Arc<dyn VirtualFileSystem>>,
}

impl MountManagerInner {
    pub fn new() -> Self {
        Self {
            mounted_fs: BTreeMap::new()
        }
    }

    pub fn mount_fs(&mut self, path: String, vfs: Arc<dyn VirtualFileSystem>) -> Result<(), &'static str> {
        match self.mounted_fs.try_insert(path, vfs) {
            Ok(_) => Ok(()),
            Err(_) => Err("Insert failed.")
        }
    }

    pub fn unmount_fs(&mut self, path: String) -> Result<(), &'static str> {
        match self.mounted_fs.remove(&path) {
            Some(vfs) => {
                if Arc::strong_count(&vfs) > 1 {
                    error!("The vfs you are about to remove have {} reference count. Proceed with caution.", Arc::strong_count(&vfs));
                }
                Ok(())
            },
            None => Err("Remove failed: no such VFS")
        }
    }

    /// get vfs and string relative to it.
    pub fn parse(&self, total_path: String) -> Result<(Arc<dyn VirtualFileSystem>, String), &'static str> {
        let longest_match = self.mounted_fs.iter().max_by(|x, y| -> Ordering {
            if total_path.starts_with(x.0) && total_path.starts_with(y.0){
                x.0.len().cmp(&y.0.len())
            } else if total_path.starts_with(x.0) {
                Ordering::Greater
            } else if total_path.starts_with(y.0) {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        });
        let longest_match = longest_match.ok_or("No VFS mounted!")?;
        let sub_path = total_path[longest_match.0.len()..].to_string();
        return Ok((longest_match.1.clone(), sub_path));
    }

    pub fn mounted_at(&self, vfs: Arc<dyn VirtualFileSystem>) -> Result<String, &'static str> {
        for i in &self.mounted_fs {
            if Arc::ptr_eq(&i.1, &vfs) {
                return Ok(i.0.clone());
            }
        }
        Err("VFS not found")
    }
    
    pub fn open(&self, abs_path: String, mode: OpenMode) -> Result<Arc<dyn File>, &'static str> {
        let (vfs, rel_path) = self.parse(abs_path.clone())?;
        verbose!("open: parsing res: path {}, relative path {}", abs_path, rel_path);
        return vfs.open(rel_path, mode);
    }

    pub fn mkdir(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str> {
        let (vfs, rel_path) = self.parse(abs_path)?;
        return vfs.mkdir(rel_path);
    }

    pub fn mkfile(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str> {
        let (vfs, rel_path) = self.parse(abs_path)?;
        return vfs.mkfile(rel_path);
    }

    pub fn remove(&self, abs_path: String) -> Result<(), &'static str> {
        let (vfs, rel_path) = self.parse(abs_path)?;
        return vfs.remove(rel_path);
    }
    
    pub fn link(&self, to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str> {
        let src_vfs = to_link.get_vfs()?;
        let src_path = to_link.get_path();
        let (dst_vfs, dst_path) = self.parse(dest)?;
        if Arc::ptr_eq(&src_vfs, &dst_vfs) {
            return Err("Cannot create hard link accross file systems!");
        } else {
            return src_vfs.link(to_link, dst_path);
        }
    }

    pub fn sym_link(&self, to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str> {
        let src_vfs = to_link.get_vfs()?;
        let src_rel_path = to_link.get_path();
        let src_abs_path = self.mounted_at(src_vfs)? + &src_rel_path;
        let (dst_vfs, dst_path) = self.parse(dest)?;
        return dst_vfs.sym_link(src_abs_path, dst_path);
    }

    pub fn rename(&self, to_rename: Arc<dyn File>, new_name: String) -> Result<(), &'static str> {
        let vfs = to_rename.get_vfs()?;
        return vfs.rename(to_rename, new_name);
    }
}

lazy_static! {
    static ref MOUNT_MANAGER: MountManager = MountManager::new();
}

pub fn mount_fs(path: String, vfs: Arc<dyn VirtualFileSystem>) -> Result<(), &'static str> {
    MOUNT_MANAGER.mount_fs(path, vfs)
}

pub fn unmount_fs(path: String) -> Result<(), &'static str> {
    MOUNT_MANAGER.get_inner_locked().unmount_fs(path)
}

/// get vfs and string relative to it.
pub fn parse(total_path: String) -> Result<(Arc<dyn VirtualFileSystem>, String), &'static str> {
    MOUNT_MANAGER.parse(total_path)
}

pub fn open(abs_path: String, mode: OpenMode) -> Result<Arc<dyn File>, &'static str> {
    MOUNT_MANAGER.open(abs_path, mode)
}

pub fn mkdir(abs_path: String) -> Result<Arc<dyn File>, &'static str> {
    MOUNT_MANAGER.mkdir(abs_path)
}

pub fn mkfile(abs_path: String) -> Result<Arc<dyn File>, &'static str> {
    MOUNT_MANAGER.mkfile(abs_path)
}

pub fn remove(abs_path: String) -> Result<(), &'static str> {
    MOUNT_MANAGER.remove(abs_path)
}

pub fn link(to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str> {
    MOUNT_MANAGER.link(to_link, dest)
}

pub fn sym_link(to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str> {
    MOUNT_MANAGER.sym_link(to_link, dest)
}

pub fn rename(to_rename: Arc<dyn File>, new_name: String) -> Result<(), &'static str> {
    MOUNT_MANAGER.rename(to_rename, new_name)
}