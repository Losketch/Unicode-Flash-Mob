use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

static DEFAULT_MODULE_PY: &str = include_str!("../main/Module.py");

fn restore_default_module() -> io::Result<()> {
    let out_path = Path::new("Module.py");
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = File::create(out_path)?;
    file.write_all(DEFAULT_MODULE_PY.as_bytes())?;
    Ok(())
}

pub fn main() -> io::Result<()> {

    restore_default_module()?;
    println!("已将 Module.py 恢复为默认内容。");

    Ok(())
}