use super::super::VirtualFileSystem;
use alloc::collections::BTreeMap;
use alloc::string::String;
use spin::{Mutex, MutexGuard};

pub struct MountManager {
    inner: Mutex<MountManagerInner>,
}

impl MountManager {
    pub fn get_inner_locked (&self) -> MutexGuard<MountManagerInner> {
        self.inner.lock()
    }

    pub fn mount_fs(&self) -> Result<(), &'static str> {
        todo!();
    }

    pub fn unmount_fs(&self) -> Result<(), &'static str> {
        todo!();
    }

    /// get vfs and string relative to it.
    pub fn parse(&self, total_path: String) -> Result<(dyn VirtualFileSystem, String), &'static str> {
        todo!();
    }
}

pub struct MountManagerInner {
    mounted_fs: BTreeMap<String, dyn VirtualFileSystem>,
}

impl MountManagerInner {
    // todo
}
