use anyhow::{Context, Result};
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

fn parse_unicode_data<P: AsRef<Path>>(input: P, output: P) -> Result<()> {
    let reader = BufReader::new(
        File::open(&input)
            .with_context(|| format!("无法打开输入文件 {:?}", input.as_ref()))?,
    );
    let mut writer = BufWriter::new(
        File::create(&output)
            .with_context(|| format!("无法创建输出文件 {:?}", output.as_ref()))?,
    );

    let clean_re = Regex::new(r"[<>,]|\b(?:First|Last)\b")
        .expect("正则表达式编译失败");

    let mut range_start: Option<u32> = None;
    let mut range_desc = String::new();

    for (lineno, line) in reader.lines().enumerate() {
        let line = line.context(format!("读取第 {} 行失败", lineno + 1))?;
        let fields: Vec<&str> = line.split(';').collect();
        if fields.is_empty() {
            continue;
        }

        let code_point = u32::from_str_radix(fields[0], 16)
            .context(format!("解析十六进制码位失败: `{}`", fields[0]))?;

        let raw_desc = fields.get(1).map(|s| *s).unwrap_or("").trim();

        if raw_desc.contains("First") {
            range_start = Some(code_point);
            range_desc = clean_re
                .replace_all(raw_desc, "")
                .trim()
                .to_string();
        } else if raw_desc.contains("Last") {
            if let Some(start) = range_start.take() {
                for cp in start..=code_point {
                    write!(writer, "U+{:04X}-{}\r\n", cp, range_desc)
                        .context("写入输出失败")?;
                }
            }
        } else {
            let desc = raw_desc.replace('-', " ").trim().to_string();
            if desc.is_empty() {
                write!(writer, "U+{:04X}\r\n", code_point)
                    .context("写入输出失败")?;
            } else {
                write!(writer, "U+{:04X}-{}\r\n", code_point, desc)
                    .context("写入输出失败")?;
            }
        }
    }

    writer.flush().context("刷新输出缓冲区失败")?;
    Ok(())
}

pub fn main() -> Result<()> {
    let input_path = "UnicodeData.txt";
    let output_path = "DecipherUnicodeData.txt";

    if let Err(err) = parse_unicode_data(input_path, output_path) {
        eprintln!("错误: {:#}", err);
        std::process::exit(1);
    }

    Ok(())
}