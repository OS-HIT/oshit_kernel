use alloc::vec::Vec;
use crate::utils::strlen;
use lazy_static::*;

lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        let count = get_app_count();
        extern "C" { fn _app_names(); }
        let mut start = _app_names as usize as *const u8;
        let mut v = Vec::new();
        let len = strlen(start);
        unsafe {
            for i in 0..count {
                let slice = core::slice::from_raw_parts(start, len);
                if let Ok(name) = core::str::from_utf8(slice) {
                    v.push(name);
                    start = start.add(len + 1);
                } else {
                    panic!("Invalid app name for app {}", i);
                } 
            }
        }
        v
    };
}

pub fn get_app_count() -> usize {
    extern "C" { fn _num_app(); }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

pub fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" { fn _num_app(); }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_app_count();
    let app_start = unsafe {
        core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1)
    };
    assert!(app_id < num_app);
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id]
        )
    }
}

pub fn get_app(name: &str) -> Option<&'static [u8]> {
    for i in 0..get_app_count() {
        if APP_NAMES[i] == name {
            return Some(get_app_data(i));
        }
    }
    error!("Application {} not found!", name);
    return None;
}