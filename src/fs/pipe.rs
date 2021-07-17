use alloc::sync::Arc;

use super::File;

pub struct PipeEnd {
    // todo: finish this. Copy-paste from old
}

impl File for PipeEnd {
    fn seek(&self, offset: u64, op: super::SeekOp) -> Result<(), &'static str> {
        todo!()
    }

    fn read(&self, buffer: &mut [u8]) -> Result<u64, &'static str> {
        todo!()
    }

    fn write(&self, buffer: &[u8]) -> Result<u64, &'static str> {
        todo!()
    }

    fn to_common_file(&self) -> Option<alloc::sync::Arc<dyn super::CommonFile>> {
        todo!()
    }

    fn to_dir_file(&self) -> Option<alloc::sync::Arc<dyn super::DirFile>> {
        todo!()
    }

    fn to_device_file(&self) -> Option<alloc::sync::Arc<dyn super::DeviceFile>> {
        todo!()
    }

    fn poll(&self) -> super::file::FileStatus {
        todo!()
    }

    fn rename(&self, new_name: alloc::string::String) -> Result<(), &'static str> {
        todo!()
    }

    fn get_vfs(&self) -> alloc::sync::Arc<dyn super::VirtualFileSystem> {
        todo!()
    }

    fn get_path(&self) -> alloc::string::String {
        todo!()
    }

    fn read_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<u64, &'static str> {
        todo!()
    }

    fn write_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<u64, &'static str> {
        todo!()
    }

    fn get_cursor(&self) -> Result<u64, &'static str> {
        todo!()
    }
}

impl Drop for PipeEnd {
    fn drop(&mut self) {
        todo!()
    }
}

pub fn make_pipe() -> (Arc<PipeEnd>, Arc<PipeEnd>) {
    todo!()
}