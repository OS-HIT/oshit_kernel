use std::io::{Result, Write};
use std::fs::{File, OpenOptions, read_dir};
use chrono::{DateTime, Utc};

fn main() {
    println!("cargo:rerun-if-changed=./src/");
    updata_version_number().unwrap();
    insert_app_data().unwrap();
}

fn updata_version_number() -> Result<()> {
    let now: DateTime<Utc> = Utc::now();
    let mut fo = OpenOptions::new()
        .write(true)
        .open("src/version.rs")
        .unwrap();
    
    let ni = format!(r#"//! This is a uname constant, and will be update automatically on building.
/// NOTE: following line will be found and modified by build.rs. ***DONT CHANGE THIS LINE MANUALLY!!!!***
pub const VERSION : &[u8] = b"{}\0";"#, now.to_rfc2822());
    writeln!(fo, "{}", ni)?;
    Ok(())
}

static TARGET_PATH: &str = "built_in_elfs/";

fn insert_app_data() -> Result<()> {
    let mut f = File::create("src/link_app.asm").unwrap();
    let mut apps: Vec<_> = read_dir(TARGET_PATH)
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            dir_entry.unwrap().file_name().into_string().unwrap()
        }).filter(|name|
            name == "proc0"
        )
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
    if apps.len() > 0 {
        writeln!(f, r#"    .quad app_{}_end"#, apps.len() - 1)?;
    }

    writeln!(f, r#"
    .global _app_names
_app_names:"#)?;
    for app in apps.iter() {
        writeln!(f, r#"    .string "{}""#, app)?;
    }

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
    writeln!(f, "# Try to make cargo happy: last compiled @ {}", Utc::now().to_rfc2822()).unwrap();
    Ok(())
}