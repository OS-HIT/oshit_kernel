//! Load the proc0 (init process)
use super::ProcessControlBlock;
use super::enqueue;
use lazy_static::*;
use alloc::sync::Arc;
use crate::fs::File;
use crate::fs::{open, OpenMode};
use alloc::vec::Vec;
use alloc::string::ToString;

#[cfg(feature = "built_in_proc0")]
lazy_static! {
    /// Lazy initalized proc0. Read from the file system.  
    /// Panic if proc0 was not found.
    pub static ref PROC0: Arc<ProcessControlBlock> = {
        info!("Loading proc0 form builtin");
        Arc::new(ProcessControlBlock::new(crate::process::kernel_stored_app_loader::get_app("proc0").unwrap(), "/".to_string()))
    };
}

#[cfg(not(feature = "built_in_proc0"))]
lazy_static! {
    /// Lazy initalized proc0. Read from the file system.  
    /// Panic if proc0 was not found.
    pub static ref PROC0: Arc<ProcessControlBlock> = {
        info!("Loading proc0 form fs");
        let app_name = "/proc0";
        verbose!("Exec {}", app_name);
        match open("/proc0".to_string(), OpenMode::SYS) {
            Ok(mut file) => {
                verbose!("File found {}", app_name);
                let mut v: Vec<u8> = Vec::with_capacity(file.poll().size as usize);
                v.resize(file.poll().size as usize, 0);
    
                match file.read(&mut v) {
                    Ok(res) => {
                        verbose!("Loaded App {}, size = {}", app_name, res);
                        return Arc::new(ProcessControlBlock::new(&v, app_name.to_string()));
                    },
                    Err(msg) => {
                        panic!("Failed to read file: {}", msg);
                    }
                }
            } ,
            Err(msg) =>{
                panic!("Failed to open file: {}", msg);
            }
        }
    };
}

/// Add proc0 to the process queue.
pub fn init_proc0() {
    verbose!("init_proc0");
    enqueue(PROC0.clone());
}