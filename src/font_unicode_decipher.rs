use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fs,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{PathBuf, Path},
};
use ttf_parser::Face;

const BLOCKS_FILE: &str = "DecipherUnicodeBlocks.txt";
const DATA_FILE: &str = "DecipherUnicodeData.txt";
const TEMP_FILE: &str = "Unicode.txt";

#[derive(Parser)]
#[command(author, version, about = "集成字体 Unicode 提取与说明文件替换工具")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Extract {
        #[arg(value_name = "FONT_FILES", required = true)]
        font_files: Vec<PathBuf>,

        #[arg(short, long, value_name = "OUT_FILE")]
        out: Option<PathBuf>,
    },
    Replace {
        #[arg(value_name = "MODE")]
        mode: Option<u8>,
    },
}

pub fn extract_unicode_from_fonts(font_paths: &[PathBuf], out_file: Option<&Path>) -> Result<()> {
    let out_path = out_file
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| PathBuf::from("combined_unicode_list.txt"));

    let file = File::create(&out_path)
        .with_context(|| format!("无法创建输出文件：{:?}", out_path))?;
    let mut writer = BufWriter::new(file);

    let mut seen = HashSet::new();
    let mut total = 0;

    for font_path in font_paths {
        let data = fs::read(font_path)
            .with_context(|| format!("无法读取字体文件：{:?}", font_path))?;
        let face = Face::parse(&data, 0)
            .with_context(|| format!("解析字体失败（{:?} 不是有效的 TTF/OTF）", font_path))?;

        let label = font_path.to_string_lossy();
        for cp in 0..=0x10FFFF {
            if let Some(ch) = std::char::from_u32(cp) {
                if face.glyph_index(ch).is_some() && seen.insert(cp) {
                    writeln!(writer, "\"{}\";\"U+{:04X}\"", label, cp)?;
                    total += 1;
                }
            }
        }
    }
    writer.flush().context("写入缓冲区失败")?;

    println!(
        "已提取 {} 个映射的 Unicode 码位，输出到：{:?}",
        total, out_path
    );
    Ok(())
}

pub fn replace_unicode(mode: Option<u8>) -> Result<()> {
    let choice = if let Some(m) = mode {
        match m {
            1 => "1".to_string(),
            2 => "2".to_string(),
            _ => bail!("无效模式：{}，仅支持 1 或 2", m),
        }
    } else {
        loop {
            println!(
                "\n请选择生成图片左下角说明文件：\n\
                 [1]  使用 `{}` 各个字符区块\n\
                 [2]  使用 `{}` 每个字符的详细信息\n\
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
        }
    };

    let unicode_file = if choice == "1" {
        BLOCKS_FILE
    } else {
        DATA_FILE
    };

    let unicode_map =
        read_unicode_file(unicode_file).with_context(|| format!("解析文件 `{}` 失败", unicode_file))?;

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
            let code = trimmed[..pos].to_string();
            let desc = trimmed[pos + 1..].to_string();
            map.insert(code, desc);
        }
    }
    Ok(map)
}

fn replace_content(path: &str, unicode_map: &HashMap<String, String>) -> Result<()> {
    let f = fs::File::open(path)?;
    let reader = BufReader::new(f);

    let re = Regex::new(r#""(U\+[0-9A-Fa-f]{4,6})""#).unwrap();

    let mut output = String::new();
    for line in reader.lines() {
        let line = line?;
        let replaced = re.replace_all(&line, |caps: &regex::Captures| {
            let code = &caps[1];
            if let Some(desc) = unicode_map.get(code) {
                format!("\"{}\";\"{}\"", code, desc)
            } else {
                caps[0].to_string()
            }
        });
        output.push_str(&replaced);
        output.push('\n');
    }

    let out_f = fs::File::create(path)?;
    let mut writer = BufWriter::new(out_f);
    writer.write_all(output.as_bytes())?;
    writer.flush()?;
    Ok(())
}