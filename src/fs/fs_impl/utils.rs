use crate::fs::fs_impl::vfs::OpenMode;

use super::fat32::file;

pub fn OpenMode2usize(mode: OpenMode) -> usize {
        let mut result:usize = 0;
        if mode.contains(OpenMode::READ) || mode.contains(OpenMode::SYS){
            result |= file::READ;
        }
        if mode.contains(OpenMode::WRITE) || mode.contains(OpenMode::SYS) {
            result |= file::WRITE;
        }
        if mode.contains(OpenMode::CREATE) {
            result |= file::CREATE;
        }
        if mode.contains(OpenMode::DIR) {
            result |= file::DIR;
        }
        if mode.contains(OpenMode::NO_FOLLOW) || mode.contains(OpenMode::SYS) {
            result |= file::NO_FOLLOW;
        }
        if mode.contains(OpenMode::TRUNCATE) {
            result |= file::TRUNCATE;
        }
        return result;
}