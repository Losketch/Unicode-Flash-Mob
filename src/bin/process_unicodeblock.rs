use anyhow::{Context, Result};
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

fn expand_unicode_blocks<P: AsRef<Path>>(input_path: P, output_path: P) -> Result<()> {
    let input_file = File::open(&input_path)
        .with_context(|| format!("无法打开输入文件 {:?}", input_path.as_ref()))?;
    let reader = BufReader::new(input_file);

    let output_file = File::create(&output_path)
        .with_context(|| format!("无法创建输出文件 {:?}", output_path.as_ref()))?;
    let mut writer = BufWriter::new(output_file);

    let re = Regex::new(r"^([A-F0-9]+)\.\.([A-F0-9]+);\s*(.+)$")
        .expect("正则编译失败");

    for (lineno, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("读取第 {} 行失败", lineno + 1))?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(caps) = re.captures(line) {
            let start = u32::from_str_radix(&caps[1], 16)
                .with_context(|| format!("解析起始码位失败: `{}`", &caps[1]))?;
            let end = u32::from_str_radix(&caps[2], 16)
                .with_context(|| format!("解析结束码位失败: `{}`", &caps[2]))?;
            let desc = &caps[3];

            for cp in start..=end {
                write!(writer, "U+{:04X}-{}\r\n", cp, desc)
                    .with_context(|| format!("写入码位 U+{:04X} 失败", cp))?;
            }
        }
    }

    writer.flush().context("刷新输出缓冲区失败")?;
    Ok(())
}

fn main() {
    let input = "UnicodeBlocks.txt";
    let output = "DecipherUnicodeBlocks.txt";

    if let Err(e) = expand_unicode_blocks(input, output) {
        eprintln!("错误: {:?}", e);
        std::process::exit(1);
    }
}