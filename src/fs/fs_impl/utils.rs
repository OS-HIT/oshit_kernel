use crate::fs::fs_impl::vfs::OpenMode;

use super::fat32::file;

pub fn OpenMode2usize(mode: OpenMode) -> usize {
        let mut result:usize = 0;
        if mode.contains(OpenMode::READ) {
            result |= file::READ;
        }
        if mode.contains(OpenMode::WRITE) {
            result |= file::WRITE;
        }
        if mode.contains(OpenMode::CREATE) {
            result |= file::CREATE;
        }
        if mode.contains(OpenMode::DIR) {
            result |= file::DIR;
        }
        if mode.contains(OpenMode::NO_FOLLOW) {
            result |= file::NO_FOLLOW;
        }
        return result;
}