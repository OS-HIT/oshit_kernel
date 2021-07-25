//! File system FILE, but with a mutex to protect it.
use spin::{
    Mutex,
    MutexGuard
};
use core::convert::TryInto;

pub struct FileWithLock {
    pub inner: Mutex<super::FILE>,
}

impl FileWithLock {
    pub fn new(inner: super::FILE) -> Self {
        Self {
            inner: Mutex::new(inner)
        }
    }
    pub fn get_inner_locked(&self) -> MutexGuard<super::FILE> {
        return self.inner.lock();
    }
}

impl super::VirtFile for FileWithLock {
    fn read(&self, buf: crate::memory::UserBuffer) -> isize {
        let mut inner = self.inner.lock();
        let mut total_len = 0;
        for part in buf.parts {
            match inner.read_file(part) {
                Ok(l) => {
                    total_len += l;
                    // premature stop
                    if l < part.len().try_into().unwrap() {
                        break;
                    }
                }
                Err(msg) => {
                    error!("{}", msg);
                    break;
                }
            }
        }
        return total_len.try_into().unwrap();
    }

    fn write(&self, buf: crate::memory::UserBuffer) -> isize {
        let mut inner = self.inner.lock();
        let res = inner.write_file(&buf.clone_bytes());
        match res {
            Ok(n) => n.try_into().unwrap(),
            Err(err) => {
                error!("{}", err);
                -1
            }
        }
    }
    fn to_fs_file_locked(&self) -> Result<MutexGuard<super::FILE>, &str> {
        Ok(self.get_inner_locked())
    }
}
