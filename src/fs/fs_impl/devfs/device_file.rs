use crate::fs::CommonFile;

use super::super::super::File;

pub trait DeviceFile : File {
    /// Good old IOCTL, device spcific commands.
    fn ioctl(&self, op: u64) -> Result<u64, &'static str>;
}

pub trait CharDevice : DeviceFile {
    fn flush(&self);
}

pub trait BlockDevice : DeviceFile {
    // todo
}

pub trait NetworkDevice : DeviceFile {
    // todo
}