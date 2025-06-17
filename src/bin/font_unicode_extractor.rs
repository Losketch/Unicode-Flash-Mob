use fontdb::Font;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

fn extract_unicode_from_font(font_path: &str) -> Result<(), Box<dyn Error>> {
    let font = Font::from_file(font_path)?;
    let cmap = font.char_map()?;

    let mut unicode_list = Vec::new();
    for char_code in cmap.keys() {
        unicode_list.push(format!("U+{:04X}", char_code));
    }

    let file_name = PathBuf::from(font_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string()
        + "_font_unicode_list.txt";

    fs::write(&file_name, unicode_list.join("\n"))?;
    println!("Unicode list saved to: {}", file_name);

    let mut input = String::new();
    print!("\nDo you want to rename the font file to 'font.ttf'? (y/n): ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() == "y" {
        rename_font_file(font_path)?;
    }

    Ok(())
}

fn rename_font_file(font_path: &str) -> Result<(), Box<dyn Error>> {
    let new_font_path = PathBuf::from(font_path)
        .with_file_name("font.ttf")
        .to_str()
        .unwrap()
        .to_string();

    fs::rename(font_path, &new_font_path)?;
    println!("Font file renamed to: {}", new_font_path);

    Ok(())
}

fn main() {
    let font_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| {
            println!("\nPlease drag and drop the font file onto this program.");
            std::process::exit(1);
        });

    if let Err(e) = extract_unicode_from_font(&font_path) {
        eprintln!("Error: {}", e);
    }
}