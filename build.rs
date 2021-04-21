use std::io::{Result, Write, prelude::*, SeekFrom};
use std::fs::{File, read_dir, OpenOptions};
use chrono::{DateTime, Utc};
use std::io::{Error, ErrorKind};
// use std::time::SystemTime;

fn main() {
    println!("cargo:rerun-if-changed=../oshit_usrlib/src/");
    println!("cargo:rerun-if-changed={}", TARGET_PATH);
    println!("cargo:rerun-if-changed=./src/");
    insert_app_data().unwrap();
    updata_version_number().unwrap();
}

static TARGET_PATH: &str = "../user_bins/";
static VERSION: &str = "VERSION";

fn updata_version_number() -> Result<()> {
    let now: DateTime<Utc> = Utc::now();
    let mut fo = OpenOptions::new()
        .write(true)
        .read(true)
        // .truncate(true)
        .open("src/config.rs")
        .unwrap();
    let mut data = String::new();
    fo.read_to_string(&mut data)?;
    let mut lines: Vec<&str> = data.split("\n").collect();
    for i in &mut lines {
        if let Some(_pos) = i.find(VERSION) {
            let ni = format!("pub const VERSION : &[u8] = b\"{}\\0\";", now.to_rfc2822());
            *i = ni.as_str();
            fo.seek(SeekFrom::Start(0)).unwrap();
            for j in lines {
                writeln!(fo, "{}", j)?;
            }
            return Ok(());
        }
    }
    Err(Error::new(ErrorKind::Other, "oh no!"))
}

fn insert_app_data() -> Result<()> {
    let mut f = File::create("src/link_app.asm").unwrap();
    let mut apps: Vec<_> = read_dir(TARGET_PATH)
        .unwrap()
        .into_iter()
        // .map(|dir_entry| {
        //     let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
        //     name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
        //     name_with_ext
        // })
        .map(|dir_entry| {
            dir_entry.unwrap().file_name().into_string().unwrap()
        })
        .collect();
    apps.sort();

    writeln!(f, r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}"#, apps.len())?;

    for i in 0..apps.len() {
        writeln!(f, r#"    .quad app_{}_start"#, i)?;
    }
    writeln!(f, r#"    .quad app_{}_end"#, apps.len() - 1)?;

    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        writeln!(f, r#"
    .section .data
    .global app_{0}_start
    .global app_{0}_end
    .align 3
app_{0}_start:
    .incbin "{2}{1}"
app_{0}_end:"#, idx, app, TARGET_PATH)?;
    }
    writeln!(f, "# Try to make cargo happy: last compiled @ {}", Utc::now().to_rfc2822());
    Ok(())
}