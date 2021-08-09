///! Still don't know about rootfs thing. Maybe need a rootfs to chroot.

mod file;
mod pipe;
mod path;
mod mount_manager;
pub mod fs_impl;
mod block_cache;

pub use file::{
	File, 
	SeekOp, 
	FileStatus,
};

pub use fs_impl::{
	OpenMode,
	CommonFile, 
    DirFile, 
    DeviceFile,
    VirtualFileSystem,
    FSFlags,
    FSStatus,
	SDA_WRAPPER,
	DEV_FS,
};

pub use path::{
	parse_path,
	Path,
	to_string,
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