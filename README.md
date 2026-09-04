<p align="center">
  <img src="./public/icon.svg" alt="Unicode Flash Mob" width="120" />
</p>

# Unicode Flash Mob

Unicode Flash Mob 是一个以 Rust、Tauri 和 React 构建的 Unicode 字形视频生成器。它可以从一个或多个字体中提取字符，使用 JSON 配置控制排版、颜色、事件和编码参数，并将 RGBA 帧直接传输给 FFmpeg。

## 主要能力

- 从 TTF、OTF 等字体中提取 Unicode 映射并生成配置。
- 使用 `U+XXXX`、`/glyphName` 和 `#glyphIndex` 选择字形。
- Unicode 选择器通过 Parley/HarfRust 应用 OpenType shaping、feature、可变字体轴与 optical sizing；`{fontIndex}` 只固定该 Unicode run 使用的字体，连续且 fontIndex 相同的码点仍一起 shaping。只有显式 `/glyphName` 与 `#glyphIndex` 保持 exact-glyph 语义并绕过 GSUB/GPOS substitution。
- 支持有序场景组件、每组件字体回退、彩色字体、组合字符和事件动画。
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

当前 Windows/Linux 官方构建采用 **unsigned** 发布策略，并使用 `--no-sign`。Windows 从浏览器下载后可能出现 SmartScreen 警告；这是未签名发布的已知限制。

macOS 暂不进入官方 artifact 矩阵；具备合适的 Apple 签名/notarization 条件后再恢复公开 macOS 包。

## CI

GitHub Actions 分为两个阶段：

1. Ubuntu 22.04 上执行前端构建、Rust 格式检查、单一 `--all-features` Cargo gate、全部测试和 Clippy。
2. 质量检查通过后，在 Windows 和 Ubuntu 22.04 上构建 unsigned Tauri bundle，并上传工作流 artifact。
3. 推送 `v*-rc*` tag 时，`.github/workflows/release.yml` 用同样的 Windows/Linux unsigned 策略创建 **draft prerelease** 并附加安装包。

不再为了 CLI/GUI 重复维护 `--no-default-features` 与 `--all-features` 两套质量矩阵；当前发行二进制本身同时提供 GUI 和 CLI。

## CLI

CLI 与 GUI 使用同一个二进制入口。项目不把 headless feature 组合维护为独立支持矩阵；文档、CI 和 release gate 统一使用 `--all-features`。所有命令均从仓库根目录执行，并显式指定 Rust manifest：

```bash
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- --help
```

常用命令：

```bash
# 从字体生成配置
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- \
  extract ./example.ttf --output unicode_config.json

# 根据配置渲染视频
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- \
  render --config unicode_config.json

# 在不渲染的情况下执行完整 render-ready 配置预检
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- \
  validate --config unicode_config.json

# 只验证静态预览所需条件（不要求 frames/output/FFmpeg/music）
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- \
  validate --config unicode_config.json --preview

# 检查运行时资源、配置与 FFmpeg（适合安装后/干净机器验收）
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- \
  doctor --config unicode_config.json

# 只检查内置资源与 preview-ready 配置，不要求 FFmpeg
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- \
  doctor --config unicode_config.json --preview

# 渲染单帧预览
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- \
  frame --config unicode_config.json --entry-index 0 --output frame-preview.png

# 生成默认配置
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- \
  config unicode_config.json

# 下载 UnicodeData.txt 与 Blocks.txt；后者在本项目中保存为 UnicodeBlocks.txt
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- \
  download --output src-tauri/assets/data

# 直接验证字体的 OpenType feature 与可变轴
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- \
  font-preview --font ./example.ttf --text "ffi" --feature liga=1 --variation wght=650
```

## 场景组件与配置版本

配置 schema 当前为 `4`。画面不再固定为一个 `main_text` 和一个 `bottom_text`，而是由有序的 `components` 数组组成；数组顺序就是绘制顺序，后面的组件覆盖前面的组件。

目前实现五种 Scene 组件：

- `glyph`：用于 Unicode 码点、glyph name、glyph index 等字形选择器。Unicode 内容会作为完整 text run shaping；Unicode 后的 `{fontIndex}` 用于固定该 run 的字体而不关闭 shaping，`/glyphName` 与 `#glyphIndex` 才保留 exact-glyph 语义。
- `text`：用于说明文字、Unicode 元数据和自定义文本。由 Parley/HarfRust 完成 text-run shaping、Bidi、cluster、GSUB/GPOS、换行和字体 fallback，最终字形交给 Swash / usvg-resvg paint backend。
- `image`：用于 PNG/JPEG/WebP 静态图像。`position` 表示图像盒中心，`size.width/height` 为相对画布的归一化尺寸；支持 `contain`、`cover`、`stretch` 与独立 `opacity`。图片在 prepare/preview 阶段加载并按 component id 缓存，不进入 TypographyRenderer。
- `progress_bar`：用于矩形进度条 primitive。支持归一化 `position`/`size`、`progress`、背景/填充色、四个填充方向，以及可选的内部边框（颜色与正整数像素宽度可配）；不依赖字体或图片资源。启用边框时 `border.width >= 1`，关闭边框使用 `border: null`。
- `group`：用于递归组合子组件。支持 `translation`、`scale`、`rotation`、`anchor` 与继承式 `opacity`；父子变换按 `World = Parent × Local` 在像素空间组合，identity group 不额外重采样。
- Typography paint backend 使用局部 4× supersampling：普通 outline、COLR v0、彩色 bitmap 由 Swash 4× raster，COLR v1 / SVG-in-OpenType 由 resvg 4× raster，再以 alpha-aware box downsample 合成到目标帧。连接脚本的单色 glyph coverage 会先在高分辨率网格做 union，再降采样，避免目标分辨率 AA 边缘重复叠加。
- SVG-in-OpenType 按 OpenType 的 em-square 初始 viewport、y-down/baseline-y=0 语义解析；resvg 负责 selected-node bbox 本地化，caller 只保留目标像素的 subpixel offset，避免对 SVG ink bbox 重复平移和裁切。

每个文字相关组件独立持有 `fonts` 与 `font`，因此可以分别设置字号、fallback、OpenType feature、variation axis 与 optical sizing。例如：

```json
{
  "schema_version": 4,
  "components": [
    {
      "type": "glyph",
      "id": "main",
      "enabled": true,
      "content": "{glyph}",
      "position": { "x": 0.5, "y": 0.5 },
      "color": { "r": 0, "g": 0, "b": 0, "a": 128 },
      "fonts": ["assets/fonts/Example.ttf"],
      "font": {
        "size": 512,
        "font_feature_settings": { "liga": 1 },
        "font_variation_settings": { "wght": 650 },
        "font_optical_sizing": true
      },
      "overlay_combining_mark": true
    },
    {
      "type": "text",
      "id": "caption",
      "enabled": true,
      "content": "{code}\n{description}",
      "position": { "x": 0.05, "y": 0.95 },
      "color": { "r": 0, "g": 0, "b": 0, "a": 192 },
      "fonts": ["assets/fonts/Caption.ttf"],
      "font": {
        "size": 42,
        "font_feature_settings": {},
        "font_variation_settings": {},
        "font_optical_sizing": true
      },
      "align": "left",
      "wrap": true,
      "max_width": 0.9
    }
  ]
}
```

配置 schema v4 现已冻结为稳定公共 JSON 契约；已公开的 v3 fixed-slot 配置（`main_font`、`bottom_font`、`main_text`、`bottom_text`）仍可读取并迁移为 v4 的默认 `glyph`/`text` scene。`image`、`progress_bar`、`group`、Transform 与 component-property animation 都属于同一套稳定 v4 Scene Component System。从 v4 冻结起，任何破坏既有合法 v4 配置的变更都必须进入新的 schema 版本；高于当前版本的 schema 仍会被拒绝加载或保存，避免旧版程序静默降级未来格式。

`ffmpeg.parallel_workers`、`ffmpeg.max_inflight_frames` 与 `ffmpeg.encoding_processes` 是高级并行参数。默认值分别为自动、`2` 与 `1`。提高这些值会增加渲染并发、RGBA 帧队列或 FFmpeg 进程数量，并可能显著增加 CPU/GPU、内存占用以及硬件编码会话压力；GUI 因此将它们标记为危险区设置。

## 内容模板项

所有 `glyph` / `text` 组件的 `content` 都先经过统一的模板解析器。内建占位符包括：

- `{char}`：当前选择器中可还原出的 Unicode 文本。
- `{glyph}`：当前 `characters[].code_point` 字形选择器表达式。
- `{code}`：当前 `characters[].code_point` 字形选择器表达式。
- `{description}`：Unicode 数据中的字符名称或配置中的描述。

`content_templates` 还可以定义命名模板：

```json
{
  "content_templates": {
    "label": {
      "type": "text",
      "value": "{char} — {description}"
    },
    "lookup": {
      "type": "external",
      "executable": "./tools/lookup.exe",
      "args": ["{code}", "{label}"]
    }
  }
}
```

组件中可写 `"content": "{label}"` 或 `"content": "{lookup}"`。命名模板可以引用其他模板；循环引用会报错。外部模板直接启动指定可执行文件而**不经过 shell**，每个 `args` 项都是独立参数，使用 UTF-8 stdout 作为模板结果。相同的可执行文件与已解析参数组合在一次渲染任务中会复用缓存结果，避免逐帧重复启动进程。

## 字形选择与 OpenType 设置

`characters[].code_point` 与 `glyph` 组件解析后的内容支持串联多个选择器：

```text
U+0041
/glyphName
#123
U+0033U+0034
U+0041{1}
```

`{1}` 表示强制使用**当前 glyph 组件** `fonts` 中索引为 1 的字体；不写索引时按该组件自己的字体列表 fallback。`font_feature_settings` 与 `font_variation_settings` 使用四字符 OpenType tag：

```json
{
  "font": {
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

Unicode 选择器（包括单一码点和多码点序列）会作为 Unicode text run 交给 Parley/HarfRust，因此 `liga`、`kern`、`mark`、`mkmk`、`locl` 等适用的 GSUB/GPOS feature 可以参与 shaping；variation selector、emoji modifier 与 ZWJ sequence 也不会再被逐 scalar 拆开。
当同一 glyph 表达式混合 Unicode selector 与 `/glyphName`、`#glyphIndex` 时，Unicode 部分仍以连续 run 独立 shaping；exact selector 只在边界处打断 shaping。Unicode `{fontIndex}` 是 font pin：相邻且 index 相同的码点会保持为同一个 shaping run，index 改变才形成新的 shaping 边界。混合表达式中的 Unicode run 会保留准备阶段生成的同一个 Parley layout 到最终 paint，不再为“测宽”和“绘制”分别 shape 两次。

对于 `/glyphName` 与 `#glyphIndex`（可选再带 `{fontIndex}` 指定从哪一个字体取该显式 glyph），渲染器保持 exact glyph ID 语义：**不会执行 GSUB/GPOS substitution，也不会因为 `font_feature_settings` 把它替换成另一个 glyph**。但 `font_variation_settings` 与 `font_optical_sizing` 仍会应用到该字形实例的 paint、advance、bbox 与垂直 metrics。

同一个表达式可以混合 exact selector 与普通 Unicode selector。例如 `#83/zeroU+0030` 会切成两个 exact glyph 与一个 Unicode shaping run；最后的 `U+0030` 仍可响应 `zero`、`salt`、`locl` 等适用 feature。`U+0041{2}U+030A{2}` 则会作为一个固定到字体 2 的 Unicode run 共同 shaping。不会跨 `/glyphName`、`#glyphIndex` 或不同的 `{fontIndex}` 边界执行 ligature、kerning 或 mark attachment。

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

除 `assets/...` 外，JSON 中的普通相对字体、图片、音乐、输出路径以及 path-like external template/FFmpeg 路径，均以**配置文件所在目录**为基准解析；保存配置时，位于配置目录内部的绝对路径会尽量重新写为相对路径。`ffmpeg`、`python` 这类只有一个文件名的 bare command 仍按进程 `PATH` 查找。

GUI 创建默认配置时优先把视频输出放在系统 Videos 目录下的 `Unicode Flash Mob` 子目录；如果系统无法提供 Videos 目录，则回退到当前工作目录。应用不会把 Documents 作为隐式回退位置。

注意：从 Finder、桌面菜单或图形化启动器打开的 macOS／Linux GUI 通常不会继承 shell 配置文件中的完整 `PATH`。正式分发时，建议打包经过授权的 FFmpeg，或让用户在配置中选择 FFmpeg 的绝对路径。

仅在许可证允许重新分发时，才能把字体、音乐、Unicode 数据或 FFmpeg 放入公开安装包。

## 发布前检查

从仓库根目录执行：

```bash
pnpm install --frozen-lockfile
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml --all-features --locked
cargo test --manifest-path src-tauri/Cargo.toml --all-features --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features --locked -- -D warnings
cargo run --manifest-path src-tauri/Cargo.toml --all-features --locked -- validate --config src-tauri/tests/fixtures/configs/component_animation.json --preview
cargo tauri build --ci --no-sign -- --locked
```

Rust tests and runnable fixtures are organized under `src-tauri/tests/`; see `src-tauri/tests/README.md` for the functional fixture map.

其中最后一条命令验证实际 unsigned 安装包。发布 Windows/Linux 安装包前仍需要：

- 在干净机器或虚拟机中安装并测试生成的 bundle；
- 明确记录 Windows SmartScreen 等未签名提示属于已知发布限制；
- 确认打包资源的许可证与来源；
- 测试无音乐、带音乐、软件编码和可用的硬件编码路径。

macOS 公开包暂不属于当前 release gate。

## License

Apache License 2.0。详见 [LICENSE](./LICENSE)。


## Scene groups

The stable v4 scene can nest components with `GroupComponent`. Group transforms use normalized configuration coordinates and pixel-space affine composition:

```text
World = Parent × T(translation) × T(anchor) × R(rotation) × S(scale) × T(-anchor)
```

Group opacity is inherited multiplicatively. Identity groups bypass affine resampling; transformed descendants are rendered by their existing component painter and affine-composited once at the leaf boundary.


## Component properties and animation

The stable v4 scene uses a finite component-property model instead of arbitrary string reflection. New configs can use:

```json
{
  "frame": 0,
  "event_type": {
    "animate_component_property": {
      "element_id": "card_group",
      "property": "rotation",
      "start_value": 0.0,
      "end_value": 20.0,
      "duration": 1.0,
      "curve": "ease_in_out"
    }
  }
}
```

Supported properties are deliberately limited to `position_x`, `position_y`, `scale_x`, `scale_y`, `rotation`, `opacity`, `color`, and `progress`. Capabilities follow the component model rather than pretending every component owns every property:

- all components: `position_x`, `position_y`;
- `glyph`, `text`: `color`;
- `image`: `opacity`;
- `progress_bar`: `progress`;
- `group`: `scale_x`, `scale_y`, `rotation`, `opacity`.

For leaf rotation/scale, wrap the leaf in a `group`; the property system does not add a second transform model to every component. Unsupported property/component pairs fail validation instead of becoming silent no-ops.

Legacy `set_component_position`, `move_component_position`, and `set_component_color` events remain readable and are executed through the same internal property state. They exist for legacy/migration continuity rather than as a second animation engine.
