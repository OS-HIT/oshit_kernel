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

pub fn strlen(src: *const u8) -> usize {
    assert_ne!(src as usize, 0, "NULL src in strlen!");
    let mut p = src;

    unsafe {
        while *p != b'\0' {
            p.add(1);
        }
    }

    return p as usize - src as usize;
}