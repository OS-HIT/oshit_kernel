use spin::Mutex;
use core::convert::TryInto;

pub struct VirtFile {
    pub inner: Mutex<super::FILE>,
}

impl VirtFile {
    pub fn new(inner: super::FILE) -> Self {
        Self {
            inner: Mutex::new(inner)
        }
    }
}

impl super::File for VirtFile {
    fn read(&self, mut buf: crate::memory::UserBuffer) -> isize {
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
}