use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

const TARGETS: &[(&str, &str)] = &[
    (
        "https://unicode.org/Public/latest/ucd/UnicodeData.txt",
        "UnicodeData.txt",
    ),
    (
        "https://unicode.org/Public/latest/ucd/Blocks.txt",
        "UnicodeBlocks.txt",
    ),
];

fn http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .user_agent(concat!("unicode-flash-mob/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("Failed to initialize HTTP client")
}

pub fn download_all(output_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create directory: {}", output_dir.display()))?;
    let client = http_client()?;

    for (url, filename) in TARGETS {
        let destination = output_dir.join(filename);
        if destination.exists() {
            println!("  {} already exists, skipping", filename);
            continue;
        }
        download_file_with_client(&client, url, &destination)?;
    }
    Ok(())
}

pub fn download_file(url: &str, destination: &Path) -> Result<()> {
    let client = http_client()?;
    download_file_with_client(&client, url, destination)
}

fn download_file_with_client(client: &Client, url: &str, destination: &Path) -> Result<()> {
    if let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    println!("  Downloading: {}", url);
    let response = client
        .get(url)
        .send()
        .with_context(|| format!("Request failed: {}", url))?
        .error_for_status()
        .with_context(|| format!("Download failed: {}", url))?;
    let bytes = response
        .bytes()
        .with_context(|| format!("Failed to read response: {}", url))?;

    let temporary = temporary_download_path(destination);
    let write_result = (|| -> Result<()> {
        let mut file = std::fs::File::create(&temporary)
            .with_context(|| format!("Failed to create file: {}", temporary.display()))?;
        file.write_all(&bytes)
            .with_context(|| format!("Failed to write file: {}", temporary.display()))?;
        file.sync_all()
            .with_context(|| format!("Failed to flush file: {}", temporary.display()))?;
        std::fs::rename(&temporary, destination).with_context(|| {
            format!(
                "Failed to move downloaded file to {}",
                destination.display()
            )
        })?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    write_result?;

    println!("  Saved: {}", destination.display());
    Ok(())
}

fn temporary_download_path(destination: &Path) -> PathBuf {
    let filename = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    destination.with_file_name(format!(".{filename}.part"))
}
