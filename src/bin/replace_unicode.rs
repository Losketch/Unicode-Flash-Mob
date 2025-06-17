use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};

const BLOCKS_FILE: &str = "DecipherUnicodeBlocks.txt";
const DATA_FILE:   &str = "DecipherUnicodeData.txt";
const TEMP_FILE:   &str = "Unicode.txt";

fn main() -> Result<()> {
    let option = loop {
        println!(
            "\n请选择生成图片左下角说明文件\n\
             [1]. 选择 {} 各个字符区块\n\
             [2]. 选择 {} 每个字符的详细信息\n\
             你选择：",
            BLOCKS_FILE, DATA_FILE
        );
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .context("读取用户输入失败")?;
        match input.trim() {
            "1" => break "1".to_string(),
            "2" => break "2".to_string(),
            _ => {
                println!("输入非法，请输入 1 或 2。");
                continue;
            }
        }
    };

    let unicode_file = if option == "1" {
        BLOCKS_FILE
    } else {
        DATA_FILE
    };

    let unicode_map = read_unicode_file(unicode_file)
        .with_context(|| format!("解析文件 `{}` 失败", unicode_file))?;

    replace_content(TEMP_FILE, &unicode_map)
        .with_context(|| format!("写入文件 `{}` 失败", TEMP_FILE))?;

    println!("替换完成，结果已写入 `{}`", TEMP_FILE);
    Ok(())
}

fn read_unicode_file(path: &str) -> Result<HashMap<String, String>> {
    let f = File::open(path).with_context(|| format!("无法打开文件 `{}`", path))?;
    let reader = BufReader::new(f);
    let mut map = HashMap::new();

    for line in reader.lines() {
        let line = line.context("读取行失败")?;
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find('-') {
            let key = trimmed[..pos].to_string();
            map.insert(key, trimmed.to_string());
        }
    }
    Ok(map)
}

fn replace_content(path: &str, unicode_map: &HashMap<String, String>) -> Result<()> {
    let f = File::open(path).with_context(|| format!("无法打开文件 `{}`", path))?;
    let reader = BufReader::new(f);

    let mut output_lines = Vec::new();
    for line in reader.lines() {
        let line = line.context("读取行失败")?;
        let trimmed = line.trim();
        if let Some(repl) = unicode_map.get(trimmed) {
            output_lines.push(format!("{}\r\n", repl));
        } else {
            output_lines.push(format!("{}\r\n", line));
        }
    }

    let out_f = File::create(path).with_context(|| format!("无法创建文件 `{}`", path))?;
    let mut writer = BufWriter::new(out_f);
    for l in output_lines {
        writer
            .write_all(l.as_bytes())
            .context("写入行失败")?;
    }
    writer.flush().context("刷新缓冲区失败")?;
    Ok(())
}