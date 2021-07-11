///! Still don't know about rootfs thing. Maybe need a rootfs to chroot.

mod file;
mod pipe;
mod mount_manager;
mod fs_impl;
mod block_cache;

pub use file::{File, SeekOp};
pub use fs_impl::{
	CommonFile, 
    DirFile, 
    DeviceFile,
    VirtualFileSystem,
    FSFlags,
    OpenMode,
    FSStatus
};

pub use mount_manager::{
	mount_fs,
	unmount_fs,
	parse,
	open,
	mkdir,
	mkfile,
	remove,
	link,
	sym_link,
	rename
};

pub use pipe::{
	PipeEnd,
	make_pipe
};