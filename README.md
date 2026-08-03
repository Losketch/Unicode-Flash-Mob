<p align="center">
  <img src="./public/icon.svg" alt="Unicode Flash Mob" width="120" />
</p>

# Unicode Flash Mob

Unicode Flash Mob 是一个以 Rust、Tauri 和 React 构建的 Unicode 字形视频生成器。它可以从一个或多个字体中提取字符，使用 JSON 配置控制排版、颜色、事件和编码参数，并将 RGBA 帧直接传输给 FFmpeg。

## 主要能力

- 从 TTF、OTF 等字体中提取 Unicode 映射并生成配置。
- 使用 `U+XXXX`、`/glyphName` 和 `#glyphIndex` 选择字形。
- 对三种选择器统一应用 OpenType feature、可变字体轴和 optical sizing。
- 支持彩色字体、字体回退、组合字符、底部说明文字和事件动画。
- 支持单进程编码与分段多进程编码，并限制在途 RGBA 帧以控制内存占用。
- 提供桌面 GUI 与独立 CLI。

## 环境要求

- Node.js 22 或 24。CI 使用 Node.js 22；pnpm 11 不支持 Node.js 20 及更早版本。
- pnpm 11.18.0。版本已由根目录 `package.json` 的 `packageManager` 字段固定。
- Rust stable，以及 `rustfmt` 和 `clippy` 组件。
- Tauri CLI 2.x。本文与 CI 使用 `^2.9.0`，以确保支持 `tauri build --no-sign`。
- FFmpeg，可位于配置指定路径、应用资源目录或进程 `PATH` 中。
- [Tauri 对应平台的系统依赖](https://v2.tauri.app/start/prerequisites/)。

Linux 发布包建议在 Ubuntu 22.04 或同等基线上构建，以避免生成依赖过新 glibc 的二进制文件。

## 开发

以下命令均从仓库根目录执行。根目录没有 `Cargo.toml`；Rust manifest 位于 `src-tauri/Cargo.toml`。

首次配置 pnpm 时，建议先更新并启用 Corepack：

```bash
npm install --global corepack@latest
corepack enable pnpm
```

项目的 `packageManager` 字段会让 Corepack 使用固定的 pnpm 版本。随后安装依赖与 Tauri CLI：

```bash
git clone https://github.com/Losketch/Unicode-Flash-Mob.git
cd Unicode-Flash-Mob
pnpm install --frozen-lockfile
rustup component add rustfmt clippy
cargo install tauri-cli --version "^2.9.0" --locked
cargo tauri dev
```

仅启动前端浏览器预览：

```bash
pnpm dev
```

浏览器预览不能调用 Tauri 文件对话框、任务事件或 Rust 渲染命令。

## 构建桌面应用

从仓库根目录执行：

```bash
pnpm install --frozen-lockfile
cargo tauri build --ci --no-sign -- --locked
```

`cargo tauri build` 会根据 `src-tauri/tauri.conf.json` 自动执行 `beforeBuildCommand`，因此会再次运行 `pnpm build`，然后构建 release 二进制和当前平台的 bundle／安装包。产物位于：

```text
src-tauri/target/release/bundle/
```

`--no-sign` 只适合本地验证和普通 CI。正式签名发布时应配置平台签名凭据，并移除该参数：

```bash
cargo tauri build --ci -- --locked
```

## CI

GitHub Actions 分为两个阶段：

1. Ubuntu 22.04 上执行前端构建、Rust 格式检查、CLI-only 检查、全部测试和 Clippy。
2. 质量检查通过后，在 Windows、macOS 和 Ubuntu 22.04 上分别执行真正的 `cargo tauri build`，并上传 `src-tauri/target/release/bundle/**`。

## CLI

CLI 可以不编译 Tauri GUI 依赖。所有命令均从仓库根目录执行，并显式指定 Rust manifest：

```bash
cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --features cli --locked -- --help
```

常用命令：

```bash
# 从字体生成配置
cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --features cli --locked -- \
  extract ./example.ttf --output unicode_config.json

# 根据配置渲染视频
cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --features cli --locked -- \
  render --config unicode_config.json

# 渲染单帧预览
cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --features cli --locked -- \
  frame --config unicode_config.json --entry-index 0 --output frame-preview.png

# 生成默认配置
cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --features cli --locked -- \
  config unicode_config.json

# 下载 UnicodeData.txt 与 Blocks.txt；后者在本项目中保存为 UnicodeBlocks.txt
cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --features cli --locked -- \
  download --output src-tauri/assets/data

# 直接验证字体的 OpenType feature 与可变轴
cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --features cli --locked -- \
  font-preview --font ./example.ttf --text "ffi" --feature liga=1 --variation wght=650
```

## 字形选择与 OpenType 设置

`characters[].code_point` 支持串联多个选择器：

```text
U+0041
/glyphName
#123
U+0033U+0034
U+0041{1}
```

`{1}` 表示使用 `main_text.fonts` 中索引为 1 的字体。`font_feature_settings` 与 `font_variation_settings` 使用四字符 OpenType tag：

```json
{
  "main_font": {
    "size": 512,
    "font_feature_settings": {
      "liga": 1,
      "kern": 1
    },
    "font_variation_settings": {
      "wght": 650,
      "wdth": 90
    },
    "font_optical_sizing": true
  }
}
```

对于 `/glyphName` 和 `#glyphIndex`，渲染器会保持显式 glyph ID 语义，同时通过与 Unicode 选择器相同的 OpenType-aware 渲染路径应用 feature 和 variation axis。

## 底部文字模板

`bottom_text.content` 支持以下占位符：

- `{char}`：当前 Unicode 字符；显式 glyph name/index 无对应字符时显示替代字符。
- `{code}`：当前 `characters[].code_point` 选择器表达式。
- `{description}`：Unicode 数据中的字符名称或配置中的描述。

示例：

```text
{char}  {code}  {description}
```

## 可选资源

打包前可将资源放入 `src-tauri/assets`：

```text
assets/
├── data/
│   ├── UnicodeData.txt
│   └── UnicodeBlocks.txt
├── fonts/
├── audio/
└── ffmpeg/
    └── bin/
        ├── ffmpeg.exe   # Windows
        └── ffmpeg       # macOS/Linux
```

这些资源均为可选项。缺少时，应用会使用用户选择的字体、空 Unicode 元数据降级，并尝试从进程 `PATH` 查找 FFmpeg。仓库不附带第三方字体、音乐或 FFmpeg 二进制文件。

配置中引用内置资源时，应保存为稳定的相对路径，例如：

```text
assets/fonts/Example.ttf
assets/audio/example.m4a
```

应用运行时会把这些路径解析到开发目录或安装包的 resource directory，不应将 `src-tauri/target/debug/...`、`src-tauri/target/release/...` 等构建目录写入配置。

注意：从 Finder、桌面菜单或图形化启动器打开的 macOS／Linux GUI 通常不会继承 shell 配置文件中的完整 `PATH`。正式分发时，建议打包经过授权的 FFmpeg，或让用户在配置中选择 FFmpeg 的绝对路径。

仅在许可证允许重新分发时，才能把字体、音乐、Unicode 数据或 FFmpeg 放入公开安装包。

## 发布前检查

从仓库根目录执行：

```bash
pnpm install --frozen-lockfile
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features cli --locked
cargo test --manifest-path src-tauri/Cargo.toml --all-features --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings -A dead-code
cargo tauri build --ci --no-sign -- --locked
```

其中最后一条命令验证实际安装包，但跳过签名。正式分发还需要：

- 移除 `--no-sign` 并配置目标平台的签名或 notarization；
- 在干净机器或虚拟机中安装并测试生成的 bundle；
- 确认打包资源的许可证与来源；
- 测试无音乐、带音乐、软件编码和可用的硬件编码路径。

## License

Apache License 2.0。详见 [LICENSE](./LICENSE)。
