//! Some C-like memory operation

/// Copy a C-style string from `src` to `dst`
/// # Description
/// iterate through `src` until `b'\\0'` was encountered, and copy it to `dst`
/// # Example
/// ```
/// let src = b"Hello world\0";
/// let res = [0u8; 100];
/// strcpy(src.as_ptr(), res.as_ptr());
/// ```
pub fn strcpy(src: *const u8, dst: *mut u8) {
    assert_ne!(src as usize, 0, "NULL src in strcpy!");
    assert_ne!(dst as usize, 0, "NULL dst in strcpy!");
    let mut p = src;
    let mut q = dst;
    unsafe {
        while *p != b'\0' {
            *q = *p;
            p = p.add(1);
            q = q.add(1);
        }
        *q = *p;    // that \0
    }
}

/// Count the lenght of a C-style String
/// # Description
/// Iterate through `src` until `b'\\0'` was encountered, then report it's length.
/// # Example
/// ```
/// let src = b"Hello world\0";
/// let len = strlen(src.as_ptr());
/// ```
pub fn strlen(src: *const u8) -> usize {
    assert_ne!(src as usize, 0, "NULL src in strlen!");
    let mut p = src;

    unsafe {
        while *p != b'\0' {
            p = p.add(1);
        }
    }
    return p as usize - src as usize;
}