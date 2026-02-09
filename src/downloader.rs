use reqwest::blocking::get;
use reqwest::StatusCode;
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::Path,
};

fn download_file(url: &str, dest_path: &Path) -> io::Result<()> {
    println!("开始下载：{}", url);

    let mut response = get(url).map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("请求失败 {}: {}", url, e))
    })?;

    if response.status() != StatusCode::OK {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("下载失败，服务器返回 {}", response.status()),
        ));
    }

    let file = File::create(dest_path)?;
    let mut writer = BufWriter::new(file);

    response
        .copy_to(&mut writer)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("写入文件失败: {}", e)))?;
    writer.flush()?;

    println!("下载成功：{}", dest_path.display());
    Ok(())
}

pub fn main() -> io::Result<()> {
    let targets = [
        (
            "https://unicode.org/Public/latest/ucd/UnicodeData.txt",
            "UnicodeData.txt",
        ),
        (
            "https://unicode.org/Public/latest/ucd/Blocks.txt",
            "UnicodeBlocks.txt",
        ),
        (
            "https://www.unicode.org/Public/17.0.0/ucd/NamesList.txt",
            "NamesList.txt",
        ),
    ];

    for (url, filename) in &targets {
        let path = Path::new(filename);
        if let Err(e) = download_file(url, path) {
            eprintln!("Error downloading {}: {}", url, e);
        }
    }

    Ok(())
}
