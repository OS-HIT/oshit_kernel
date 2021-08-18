//! Mount Manager
use core::cmp::Ordering;

use super::super::VirtualFileSystem;
use super::super::parse_path;
use super::super::Path;
use super::super::to_string;
use alloc::borrow::ToOwned;
use alloc::{collections::BTreeMap, string::ToString};
use alloc::string::String;
use spin::{Mutex, MutexGuard};
use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::fs::{File, OpenMode};
use lazy_static::*;

/// Mount Manager (Wrapper)
/// # Description
/// File operations like "open", "create", "delete" needs to go through
/// Mount Manager first to get the curresponding filesystem.
/// 
/// We use a tree-like struct to record the mounted filesystem:
/// 
/// +-------+
/// |   /   |
/// +-------+
///    |
///    v
/// +-------+-------+-------+-------+
/// | dev/  | proc/ | Fat32 | foo/  |
/// +-------+-------+-------+-------+
///    |        \                |
///    v         v               v
/// +-------+  +-------+    +-------+
/// | devfs |  |procfs |    | bar/  |
/// +-------+  +-------+    +-------+
///                              |
///                              v
///                         +-------+
///                         | xxfs  |
///                         +-------+
/// Each cell is a MountNode, representing a directory or a fs.
/// If a node 'A' points to a vector of nodes, means all the nodes in the 
/// vector is the children of 'A'.
/// If a fs is mounted at /foo/, then node fs will be the child of node foo/, 
/// which is a child of node /.
/// In the graph above, Fat32 is mounted at /; devfs is mounted at /dev/;
/// procfs is mounted at /proc/; xxfs is mounted at /foo/bar/.
/// A node can at most have one fs node.
pub struct MountManager {
    inner: Mutex<MountManagerInner>,
}
unsafe impl Sync for MountManager {}

impl MountManager {
    /// Create a Mount Manager
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MountManagerInner::new())
        }
    }

    pub fn get_inner_locked (&self) -> MutexGuard<MountManagerInner> {
        self.inner.lock()
    }

    /// Mount a filesystem on "path"
    pub fn mount_fs(&self, path: String, vfs: Arc<dyn VirtualFileSystem>) -> Result<(), &'static str> {
        self.get_inner_locked().mount_fs(&path, vfs)
    }

    /// Unmount the filesystem on "path"
    pub fn unmount_fs(&self, path: String) -> Result<(), &'static str> {
        self.get_inner_locked().unmount_fs(&path)
    }

    /// get vfs and string relative to it.
    pub fn parse(&self, total_path: String) -> Result<(Arc<dyn VirtualFileSystem>, Path), &'static str> {
        self.get_inner_locked().parse(&total_path)
    }
    
    /// Open file
    pub fn open(&self, abs_path: String, mode: OpenMode) -> Result<Arc<dyn File>, &'static str> {
        self.get_inner_locked().open(abs_path, mode)
    }

    /// Create diretory
    pub fn mkdir(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str> {
        self.get_inner_locked().mkdir(abs_path)
    }

    /// Create file
    pub fn mkfile(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str> {
        self.get_inner_locked().mkfile(abs_path)
    }

    /// Delete file
    pub fn remove(&self, abs_path: String) -> Result<(), &'static str> {
        self.get_inner_locked().remove(abs_path)
    }
    
    /// Create hard link
    pub fn link(&self, to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str> {
        self.get_inner_locked().link(to_link, dest)
    }

    /// Create symbolic link
    pub fn sym_link(&self, to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str> {
        self.get_inner_locked().sym_link(to_link, dest)
    }

    /// Rename file (dummy function)
    pub fn rename(&self, to_rename: Arc<dyn File>, new_name: String) -> Result<(), &'static str> {
        self.get_inner_locked().rename(to_rename, new_name)
    }
}

enum MountNode {
    SubDir(String, Vec<MountNode>),
    FileSystem(Arc<dyn VirtualFileSystem>),
}

/// Mount Manager Inner
pub struct MountManagerInner {
    root: Vec<MountNode>,
}

impl MountManagerInner {
    pub fn new() -> Self {
        Self {
            root: Vec::new()
        }
    }

    fn mount(queue: &mut Vec<MountNode>, mut path: Vec::<String>, vfs: Arc<dyn VirtualFileSystem>) -> Result<(), &'static str> {
        if path.len() == 0 {
            for i in 0..queue.len() {
                if let MountNode::FileSystem(_) = queue[i] {
                    return Err("current dir already mounted");
                }
            }
            queue.push(MountNode::FileSystem(vfs.clone()));
            return Ok(());
        } else {
            let dname = path.pop().unwrap();
            for i in 0..queue.len() {
                if let MountNode::SubDir(ref name, ref mut queue) = queue[i] {
                    if dname.eq(name) {
                        return MountManagerInner::mount(queue, path, vfs);
                    }
                }
            }
            let mut sq = Vec::new();
            match MountManagerInner::mount(&mut sq, path, vfs) {
                Ok(()) => {
                    queue.push(MountNode::SubDir(dname, sq));
                    return Ok(());
                },
                Err(msg) => return Err(msg),
            }
        }
    }

    pub fn mount_fs(&mut self, path: &str, vfs: Arc<dyn VirtualFileSystem>) -> Result<(), &'static str> {
        let path = match parse_path(&path) {
            Ok(path) => path,
            Err(err) => return Err(to_string(err)),
        };
        if !path.is_abs {
            return Err("mount_fs: absolute path required");
        }
        let Path {path:mut path, ..} = path;
        path.reverse();
        MountManagerInner::mount(&mut self.root, path, vfs);
        return Ok(());
    }

    fn unmount(queue: &mut Vec<MountNode>, mut path: Vec<String>) -> Option<Arc<dyn VirtualFileSystem>> {
        if path.len() == 0 {
            for i in 0..queue.len() {
                if let MountNode::FileSystem(_) = queue[i] {
                    if let MountNode::FileSystem(fs) = queue.remove(i) {
                        return Some(fs);
                    }
                }
            }
            return None;
        } else {
            let dname = path.pop().unwrap();
            for i in 0..queue.len() {
                if let MountNode::SubDir(ref name, ref mut sq) = queue[i] {
                    if dname.eq(name) {
                        let result = MountManagerInner::unmount(sq, path);
                        if sq.len() == 0 {
                            queue.remove(i);
                        }
                        return result;
                    }
                }
            }
            return None;
        }
    }

    pub fn unmount_fs(&mut self, path: &str) -> Result<(), &'static str> {
        let path = match parse_path(&path) {
            Ok(path) => path,
            Err(err) => return Err(to_string(err)),
        };
        if !path.is_abs {
            return Err("unmount_fs: absolute path required");
        }
        let Path {path:mut path, ..} = path;
        path.reverse();
        if let Some(vfs) = MountManagerInner::unmount(&mut self.root, path) {
            if Arc::strong_count(&vfs) > 1 {
                error!("The vfs you are about to remove have {} reference count. Proceed with caution.", Arc::strong_count(&vfs));
            }
            return Ok(());
        }
        return Err("unmount_fs: path not mounted");
    }

    fn find_path(queue: &Vec<MountNode>, path:&mut Vec<String>) -> Option<Arc<dyn VirtualFileSystem>> {
        if path.len() == 0 {
            for i in 0..queue.len() {
                if let MountNode::FileSystem(vfs) = &queue[i] {
                    return Some(vfs.clone());
                }
            }
            return None;
        } else {
            let dname = path.pop().unwrap();
            let mut result:Option<Arc<dyn VirtualFileSystem>> = None;
            for i in 0..queue.len() {
                if let MountNode::SubDir(ref name, ref sq) = queue[i] {
                    if dname.eq(name) {
                        return MountManagerInner::find_path(sq, path);
                    }
                } else if let MountNode::FileSystem(ref fs) = queue[i] {
                    result = Some(fs.clone());
                }
            }
            path.push(dname);
            return result;
        }
    }

    /// get vfs and string relative to it.
    pub fn parse(&self, total_path: &str) -> Result<(Arc<dyn VirtualFileSystem>, Path), &'static str> {
        verbose!("Parsing path: {}", total_path);
        let path = match parse_path(&total_path) {
            Ok(path) => path,
            Err(err) => return Err(to_string(err)),
        };
        if !path.is_abs {
            return Err("parse: absolute path required");
        }
        let Path {mut path, must_dir, ..} = path;
        path.reverse();
        if let Some(vfs) = MountManagerInner::find_path(&self.root, &mut path) {
            path.reverse();
            let path = Path { path, must_dir, is_abs: true};
            return Ok((vfs, path));
        }
        return Err("parse: fs not found");
    }

    fn find_fs(queue: &Vec<MountNode>, vfs: &Arc<dyn VirtualFileSystem>, path:&mut Vec<String>) -> Result<(),()> {
        for i in 0..queue.len() {
            match &queue[i] {
                MountNode::FileSystem(fs) => {
                    if Arc::ptr_eq(fs, vfs) {
                        return Ok(());
                    }
                },
                MountNode::SubDir(name,queue) => {
                    path.push(name.clone());
                    if let Ok(()) = MountManagerInner::find_fs(queue, vfs, path) {
                        return Ok(());
                    }
                    path.pop();
                },
            }
        }
        return Err(());
    }

    pub fn mounted_at(&self, vfs: Arc<dyn VirtualFileSystem>) -> Result<String, &'static str> {
        let mut path = Vec::new();
        if let Ok(()) = MountManagerInner::find_fs(&self.root, &vfs, &mut path) {
            let path = Path {
                path, 
                must_dir: true,
                is_abs: true,
            };
            return Ok(path.to_string());
        };
        return Err("mounted_at: VFS not found");
    }
    
    pub fn open(&self, abs_path: String, mode: OpenMode) -> Result<Arc<dyn File>, &'static str> {
        let (vfs, rel_path) = self.parse(&abs_path)?;
        verbose!("open: parsing res: path {}, relative path {}", abs_path, rel_path.to_string());
        return vfs.open(rel_path, mode);
    }

    pub fn mkdir(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str> {
        let (vfs, rel_path) = self.parse(&abs_path)?;
        return vfs.mkdir(rel_path);
    }

    pub fn mkfile(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str> {
        let (vfs, rel_path) = self.parse(&abs_path)?;
        return vfs.mkfile(rel_path);
    }

    pub fn remove(&self, abs_path: String) -> Result<(), &'static str> {
        let (vfs, rel_path) = self.parse(&abs_path)?;
        return vfs.remove(rel_path);
    }
    
    pub fn link(&self, to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str> {
        let src_vfs = to_link.get_vfs()?;
        let src_path = to_link.get_path();
        let (dst_vfs, dst_path) = self.parse(&dest)?;
        if Arc::ptr_eq(&src_vfs, &dst_vfs) {
            return Err("Cannot create hard link accross file systems!");
        } else {
            return src_vfs.link(to_link, dst_path);
        }
    }

    pub fn sym_link(&self, to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str> {
        let src_vfs = to_link.get_vfs()?;
        let src_rel_path = to_link.get_path();
        let mut path = Vec::new();
        match MountManagerInner::find_fs(&self.root, &src_vfs, &mut path) {
            Ok(()) => {
                let mut src_abs_path = Path {
                    path, 
                    must_dir: true,
                    is_abs: true,
                };
                src_abs_path.merge(src_rel_path);
                let (dst_vfs, dst_path) = self.parse(&dest)?;
                return dst_vfs.sym_link(src_abs_path, dst_path);
            },
            Err(()) => {
                return Err("sym_link: fs not found");
            },
        };
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
    MOUNT_MANAGER.get_inner_locked().unmount_fs(&path)
}

/// get vfs and string relative to it.
pub fn parse(total_path: String) -> Result<(Arc<dyn VirtualFileSystem>, Path), &'static str> {
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