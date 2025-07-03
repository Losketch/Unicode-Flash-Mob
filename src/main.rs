mod downloader;
mod font_unicode_decipher;
mod process_unicodeblock;
mod process_unicodedata;
mod settings_writer;
mod unicode_range_generator;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about = "Unicode 工具集")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Download,
    Extract {
        #[arg(value_name = "FONT_FILES", required = true)]
        font_files: Vec<PathBuf>,
        #[arg(short, long, value_name = "OUT_FILE")]
        out: Option<PathBuf>,
    },
    ReplaceUnicodeData {
        #[arg(value_name = "MODE")]
        mode: Option<u8>,
    },
    ProcessUnicodeBlock,
    ProcessUnicodeData,
    GenerateUnicodeRange {
        #[arg(short, long, value_name = "FILENAME")]
        file: String,
        #[arg(short, long, value_name = "START_HEX")]
        start: String,
        #[arg(short, long, value_name = "END_HEX")]
        end: String,
        #[arg(short, long, value_name = "FONT_PATH")]
        font: Option<String>,
    },
    WriteSettings,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Download => downloader::main()?,
        Commands::Extract { font_files, out } => {
            font_unicode_decipher::extract_unicode_from_fonts(&font_files, out.as_deref())?
        }
        Commands::ReplaceUnicodeData { mode } => {
            font_unicode_decipher::replace_unicode(mode)?
        }
        Commands::ProcessUnicodeBlock => process_unicodeblock::main()?,
        Commands::ProcessUnicodeData => process_unicodedata::main()?,
        Commands::GenerateUnicodeRange {
            file,
            start,
            end,
            font,
        } => {
            unicode_range_generator::write_to_file(&file, &start, &end, font.as_deref())?
        }
        Commands::WriteSettings => settings_writer::main()?,
    }
    Ok(())
}