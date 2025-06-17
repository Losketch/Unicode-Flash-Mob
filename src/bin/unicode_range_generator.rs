use clap::{Arg, Command};
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

const MAX_UNICODE: u32 = 0x10FFFF;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("Unicode Range Generator")
        .version("1.0")
        .author("Losketch")
        .about("生成Unicode编码范围到文件")
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("FILENAME")
                .help("输出文件名（不用加 .txt）")
                .required(true)
                .num_args(1),
        )
        .arg(
            Arg::new("start")
                .short('s')
                .long("start")
                .value_name("HEX")
                .help("起始Unicode值（十六进制）")
                .required(true)
                .num_args(1),
        )
        .arg(
            Arg::new("end")
                .short('e')
                .long("end")
                .value_name("HEX")
                .help("结束Unicode值（十六进制）")
                .required(true)
                .num_args(1),
        )
        .get_matches();

    // 解析文件名
    let filename = matches.get_one::<String>("file").unwrap();
    let filename = if filename.to_lowercase().ends_with(".txt") {
        filename.clone()
    } else {
        format!("{}.txt", filename)
    };
    let file_path = PathBuf::from(&filename);

    // 解析范围
    let start_str = matches.get_one::<String>("start").unwrap();
    let end_str = matches.get_one::<String>("end").unwrap();

    let start = parse_hex(start_str)?;
    let end = parse_hex(end_str)?;

    if start > MAX_UNICODE || end > MAX_UNICODE {
        return Err("范围超出Unicode最大值 (U+10FFFF)".into());
    }
    if start > end {
        return Err("起始值不能大于结束值".into());
    }

    // 生成文件
    write_to_file(&file_path, start, end)?;

    println!("文件已生成: {}", filename);
    Ok(())
}

fn parse_hex(s: &str) -> Result<u32, Box<dyn Error>> {
    if !is_valid_hex(s) {
        return Err("十六进制值包含非法字符".into());
    }
    u32::from_str_radix(s, 16).map_err(|_| "十六进制转换失败".into())
}

fn is_valid_hex(s: &str) -> bool {
    s.chars().all(|c| c.is_digit(16))
}

fn write_to_file(path: &PathBuf, start: u32, end: u32) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    println!("\n正在生成文件,请稍候...");
    for code_point in start..=end {
        writeln!(writer, "U+{:04X}", code_point)?;
    }
    Ok(())
}