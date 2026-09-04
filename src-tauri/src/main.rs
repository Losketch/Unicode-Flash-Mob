mod config_validation;
mod content_template;
mod downloader;
mod extractor;
mod ffmpeg;
mod font_loader;
#[cfg(feature = "gui")]
mod gui;
mod image_renderer;
mod json_config;
mod primitive_renderer;
mod renderer;
mod scene;
mod scene_transform;
mod typography_renderer;
mod unicode_data;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(
    name = "unicode-flash-mob",
    version = "3.0.0",
    about = "Unicode flash mob video generator",
    long_about = "Extract Unicode characters from fonts and render them directly to a video file. \
                  Supports JSON-based configuration, event system (position animation, color transitions, curves), \
                  and a Tauri GUI (enabled by default).",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render video from a JSON config file
    Render {
        /// JSON config file path
        #[arg(short, long, required = true)]
        config: PathBuf,
    },

    /// Validate a JSON config without rendering it
    Validate {
        /// JSON config file path
        #[arg(short, long, required = true)]
        config: PathBuf,
        /// Validate only preview-safe requirements; skip frames/output/FFmpeg/music checks.
        #[arg(long)]
        preview: bool,
    },

    /// Check runtime assets, configuration readiness, and FFmpeg availability
    Doctor {
        /// Optional JSON config to validate as part of the runtime check
        #[arg(short, long)]
        config: Option<PathBuf>,
        /// Skip render-only checks such as FFmpeg availability
        #[arg(long)]
        preview: bool,
    },

    /// Render one character entry from a config as a PNG image
    Frame {
        /// JSON config file path
        #[arg(short, long, required = true)]
        config: PathBuf,
        /// Zero-based entry index in `characters`
        #[arg(short = 'i', long, default_value_t = 0)]
        entry_index: usize,
        /// Override the selected entry's code_point expression for this image
        #[arg(short = 'p', long)]
        code_point: Option<String>,
        /// PNG output path
        #[arg(short, long, default_value = "frame-preview.png")]
        output: PathBuf,
        /// Maximum preview width or height; 0 keeps the configured resolution
        #[arg(long, default_value_t = 0)]
        max_dimension: u32,
    },

    /// Extract characters from fonts and generate a JSON config
    Extract {
        /// Font file paths (one or more)
        #[arg(required = true)]
        font_files: Vec<PathBuf>,

        /// Output JSON config path
        #[arg(short, long, default_value = "unicode_config.json")]
        output: PathBuf,

        /// Output video path
        #[arg(long)]
        video_output: Option<PathBuf>,
    },

    /// Generate a default JSON configuration
    Config {
        /// Output path
        #[arg(default_value = "unicode_config.json")]
        output: PathBuf,
    },

    /// Download Unicode data files (UnicodeData.txt, Blocks.txt)
    Download {
        /// Output directory (defaults to bundled data directory)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Render a PNG preview with color-font and advanced OpenType settings
    FontPreview {
        #[arg(short, long)]
        font: PathBuf,
        #[arg(short, long, default_value = "🛑 🩰")]
        text: String,
        #[arg(short, long, default_value = "font-preview.png")]
        output: PathBuf,
        #[arg(long, default_value_t = 256.0)]
        size: f32,
        /// OpenType feature as TAG=VALUE; repeat this option as needed.
        #[arg(long = "feature")]
        features: Vec<String>,
        /// Variable-font axis as TAG=VALUE; repeat this option as needed.
        #[arg(long = "variation")]
        variations: Vec<String>,
        #[arg(long)]
        no_optical_sizing: bool,
    },

    #[cfg(feature = "gui")]
    /// Launch the Tauri GUI
    Gui,
}

fn main() -> Result<()> {
    #[cfg(feature = "gui")]
    if std::env::args_os().nth(1).is_none() {
        gui::run_gui().map_err(|e| anyhow::anyhow!("Failed to start GUI: {e}"))?;
        return Ok(());
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Render { config } => cmd_render(config)?,
        Commands::Validate { config, preview } => cmd_validate(config, preview)?,
        Commands::Doctor { config, preview } => cmd_doctor(config, preview)?,
        Commands::Frame {
            config,
            entry_index,
            code_point,
            output,
            max_dimension,
        } => cmd_frame(config, entry_index, code_point, output, max_dimension)?,
        Commands::Extract {
            font_files,
            output,
            video_output,
        } => cmd_extract(font_files, output, video_output)?,
        Commands::Config { output } => cmd_config(output)?,
        Commands::Download { output } => cmd_download(output)?,
        Commands::FontPreview {
            font,
            text,
            output,
            size,
            features,
            variations,
            no_optical_sizing,
        } => cmd_font_preview(
            font,
            text,
            output,
            size,
            features,
            variations,
            !no_optical_sizing,
        )?,
        #[cfg(feature = "gui")]
        Commands::Gui => {
            gui::run_gui().map_err(|e| anyhow::anyhow!("Failed to start GUI: {e}"))?;
        }
    }

    Ok(())
}

fn cmd_validate(config_path: PathBuf, preview: bool) -> Result<()> {
    let config = json_config::RenderConfig::from_file(&config_path)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;
    let mode = if preview { "preview" } else { "render" };
    config_validation::validate_render_config(&config, !preview)
        .with_context(|| format!("Configuration is not {mode}-ready"))?;
    if !preview {
        ffmpeg::check_ffmpeg_runtime(Some(&config.ffmpeg.path))
            .context("FFmpeg runtime is not render-ready")?;
    }
    println!("Configuration is {mode}-ready: {}", config_path.display());
    Ok(())
}

fn check_runtime_asset(label: &str, relative: &str) -> Result<()> {
    let path = json_config::asset_path(relative);
    if !path.is_file() {
        anyhow::bail!("Missing required runtime asset {label}: {}", path.display());
    }
    println!("[ok] {label}: {}", path.display());
    Ok(())
}

fn cmd_doctor(config_path: Option<PathBuf>, preview: bool) -> Result<()> {
    check_runtime_asset("UnicodeData.txt", "data/UnicodeData.txt")?;
    check_runtime_asset("UnicodeBlocks.txt", "data/UnicodeBlocks.txt")?;
    check_runtime_asset("NotoSansTest-Regular.ttf", "fonts/NotoSansTest-Regular.ttf")?;
    check_runtime_asset("IBMPlexSans-Bold.ttf", "fonts/IBMPlexSans-Bold.ttf")?;

    let config = config_path
        .as_ref()
        .map(|path| {
            json_config::RenderConfig::from_file(path)
                .with_context(|| format!("Failed to parse config file: {}", path.display()))
        })
        .transpose()?;

    if let (Some(config), Some(path)) = (config.as_ref(), config_path.as_ref()) {
        config_validation::validate_render_config(config, !preview).with_context(|| {
            format!(
                "Configuration is not {}-ready",
                if preview { "preview" } else { "render" }
            )
        })?;
        println!(
            "[ok] Configuration is {}-ready: {}",
            if preview { "preview" } else { "render" },
            path.display()
        );
    }

    if !preview {
        let (ffmpeg_path, version) = ffmpeg::check_ffmpeg_runtime(
            config.as_ref().map(|config| config.ffmpeg.path.as_path()),
        )?;
        println!("[ok] FFmpeg: {} ({version})", ffmpeg_path.display());
    }

    println!(
        "Runtime diagnostics passed ({})",
        if preview { "preview" } else { "render" }
    );
    Ok(())
}

fn cmd_render(config_path: PathBuf) -> Result<()> {
    let config = json_config::RenderConfig::from_file(&config_path)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;
    renderer::render_video(&config)?;
    Ok(())
}

fn cmd_frame(
    config_path: PathBuf,
    entry_index: usize,
    code_point: Option<String>,
    output: PathBuf,
    max_dimension: u32,
) -> Result<()> {
    let mut config = json_config::RenderConfig::from_file(&config_path)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;
    if let Some(code_point) = code_point {
        let entry = config.characters.get_mut(entry_index).ok_or_else(|| {
            anyhow::anyhow!("Character index {entry_index} is out of range for --code-point")
        })?;
        entry.code_point = code_point;
    }
    renderer::render_frame_to_file(&config, entry_index, &output, max_dimension)?;
    println!("Frame preview saved: {}", output.display());
    Ok(())
}

fn cmd_extract(
    font_files: Vec<PathBuf>,
    output: PathBuf,
    video_output: Option<PathBuf>,
) -> Result<()> {
    let config = extractor::extract_to_config(&font_files, &output, video_output.as_deref())?;

    println!(
        "Extracted {} characters from {} font file(s)",
        config.characters.len(),
        font_files.len()
    );
    println!("Config saved: {}", output.display());
    println!("Video output path: {}", config.output_path.display());
    Ok(())
}

fn cmd_config(output: PathBuf) -> Result<()> {
    let mut config = json_config::RenderConfig::default();

    let cwd = std::env::current_dir()?;
    config.output_path = cwd
        .join(json_config::DEFAULT_OUTPUT_DIR)
        .join(json_config::default_output_filename());
    config
        .to_file(&output)
        .with_context(|| format!("Failed to save config file: {}", output.display()))?;

    println!("Default config saved: {}", output.display());
    println!("\nEdit the config and run:");
    println!("  unicode-flash-mob render --config {}", output.display());
    Ok(())
}

fn cmd_download(output: Option<PathBuf>) -> Result<()> {
    let dir = output.unwrap_or_else(|| json_config::asset_path("data"));
    downloader::download_all(&dir)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_font_preview(
    font: PathBuf,
    text: String,
    output: PathBuf,
    size: f32,
    features: Vec<String>,
    variations: Vec<String>,
    optical_sizing: bool,
) -> Result<()> {
    let config = scene::FontConfig {
        size,
        font_feature_settings: parse_settings(&features, "feature")?,
        font_variation_settings: parse_settings(&variations, "variation")?,
        font_optical_sizing: optical_sizing,
    };
    let loader = Arc::new(font_loader::FontLoader::from_path_with_config(
        &font, &config,
    )?);
    let mut image = image::RgbaImage::from_pixel(1024, 512, image::Rgba([255, 255, 255, 255]));
    let component_id = "__font_preview".to_string();
    let mut component_fonts = HashMap::new();
    component_fonts.insert(component_id.clone(), vec![loader]);
    let mut typography = typography_renderer::TypographyRenderer::new(&component_fonts)?;
    let component = scene::TextComponent {
        id: component_id,
        enabled: true,
        content: text.clone(),
        position: scene::Position { x: 0.0, y: 0.0 },
        color: scene::Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        fonts: vec![font],
        font: config,
        align: scene::TextAlign::Left,
        wrap: false,
        max_width: 0.0,
    };
    typography.render_text(
        &component,
        &text,
        component.color.clone(),
        typography_renderer::TextPlacement {
            origin_x: 48.0,
            last_baseline_y: 256.0,
            canvas_width: image.width(),
        },
        &mut image,
    )?;
    image
        .save(&output)
        .with_context(|| format!("Failed to save font preview: {}", output.display()))?;
    println!("Font preview saved: {}", output.display());
    Ok(())
}

fn parse_settings<T>(items: &[String], kind: &str) -> Result<BTreeMap<String, T>>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    items
        .iter()
        .map(|item| {
            let (tag, value) = item
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("{kind} must use TAG=VALUE format: {item}"))?;
            if tag.len() != 4 || !tag.is_ascii() {
                anyhow::bail!("OpenType tag must be 4 ASCII characters: {tag:?}");
            }
            let value = value
                .parse::<T>()
                .map_err(|error| anyhow::anyhow!("Invalid value for {kind} {tag}: {error}"))?;
            Ok((tag.to_string(), value))
        })
        .collect()
}
