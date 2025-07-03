use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::fs::File;
use std::io::{BufWriter, Write};

pub fn write_to_file(filename: &str, start: &str, end: &str, font_path: Option<&str>) -> Result<()> {
    let path = if filename.to_lowercase().ends_with(".txt") {
        PathBuf::from(filename)
    } else {
        PathBuf::from(format!("{}.txt", filename))
    };
    
    let start_val = u32::from_str_radix(start, 16)
        .map_err(|_| anyhow!("无效的起始十六进制值: {}", start))?;
    let end_val = u32::from_str_radix(end, 16)
        .map_err(|_| anyhow!("无效的结束十六进制值: {}", end))?;
    
    const MAX_UNICODE: u32 = 0x10FFFF;
    if start_val > MAX_UNICODE || end_val > MAX_UNICODE {
        return Err(anyhow!("范围超出Unicode最大值 (U+10FFFF)"));
    }
    if start_val > end_val {
        return Err(anyhow!("起始值不能大于结束值"));
    }
    
    let file = File::create(&path)?;
    let mut writer = BufWriter::new(file);
    println!("\n正在生成文件,请稍候...");
    
    let font_name = font_path.unwrap_or("font.ttf");
    for code_point in start_val..=end_val {
        writeln!(writer, "\"{}\";\"U+{:04X}\"", font_name, code_point)?;
    }
    
    println!("文件已生成: {}", path.display());
    Ok(())
}