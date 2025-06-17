use anyhow::{Context, Result};
use clap::Parser;
use std::{
    fs,
    io::{self, BufWriter, Write},
    path::PathBuf,
};
use ttf_parser::Face;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(value_name = "FONT_FILE")]
    font_file: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    extract_unicode_from_font(&args.font_file)?;
    Ok(())
}

fn extract_unicode_from_font(font_path: &PathBuf) -> Result<()> {
    let data = fs::read(font_path)
        .with_context(|| format!("无法读取字体文件：{:?}", font_path))?;

    let face = Face::parse(&data, 0)
        .context("解析字体失败（确保这是有效的 TTF/OTF 文件）")?;

    let unicode_list: Vec<String> = (0..=0x10FFFF)
        .filter_map(|cp| {
            std::char::from_u32(cp)
                .and_then(|ch| {
                    face.glyph_index(ch).map(|_| format!("U+{:04X}", cp))
                })
        })
        .collect();

    let stem = font_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let out_path = font_path
        .with_file_name(format!("{}_字体提取的显示列表Unicode.txt", stem));

    let file = fs::File::create(&out_path)
        .with_context(|| format!("无法创建输出文件：{:?}", out_path))?;
    let mut writer = BufWriter::new(file);
    for line in &unicode_list {
        write!(writer, "{}\r\n", line)?;
    }
    println!(
        "已提取 {} 个映射的 Unicode 码位，输出到：{:?}",
        unicode_list.len(),
        out_path
    );

    prompt_rename(font_path)?;
    Ok(())
}

fn prompt_rename(font_path: &PathBuf) -> Result<()> {
    println!("是否要将字体文件重命名为 `font.ttf`？(y/n):");

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let ans = input.trim().to_lowercase();

    if ans == "y" || ans == "yes" {
        let new_path = font_path.with_file_name("font.ttf");
        fs::rename(font_path, &new_path).with_context(|| {
            format!(
                "重命名失败：{:?} -> {:?}",
                font_path, new_path
            )
        })?;
        println!("字体文件已重命名为：{:?}", new_path);
    }
    Ok(())
}