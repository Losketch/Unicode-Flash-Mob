#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import sys
import time
import logging
import argparse
from pathlib import Path
from threading import Thread
from queue import Queue
from concurrent.futures import ThreadPoolExecutor, as_completed
from io import BytesIO

sys.path.insert(0, os.path.dirname(__file__))

from control_map import get_char, CTRLS
from Module import Config, UnicodeEntry, ColorManager, load_unicode_entries, setup_logging, writer_thread_fn, get_bitmap_font_sizes, has_colr_table, has_svg_table, render_colr_glyph

from PIL import Image, ImageDraw, ImageFont
from tqdm import tqdm

Image.MAX_IMAGE_PIXELS = None

class PrecomputedValues:
    """预计算常用值以避免重复计算"""
    def __init__(self, cfg: Config):
        self.W, self.H = cfg.image_size
        self.center_x = self.W // 2
        self.center_y = self.H // 2
        self.bottom_text_y = cfg.image_size[1] - cfg.bottom_font_size - 125
        self.baseline_offset = cfg.text_position[1]
        self.text_x_offset = cfg.text_position[0]
        self.alpha = cfg.middle_font_color[3] / 255
        self.fg_color = cfg.middle_font_color[:3]
        self.overlay_alpha = self.alpha * 0.5

def fast_blend_colors(fg: tuple[int,int,int], bg: tuple[int,int,int], alpha: float) -> tuple[int,int,int]:
    """计算前景色与背景色的叠加结果"""
    inv_alpha = 1.0 - alpha
    return (
        int(fg[0] * alpha + bg[0] * inv_alpha),
        int(fg[1] * alpha + bg[1] * inv_alpha),
        int(fg[2] * alpha + bg[2] * inv_alpha)
    )

def normalize_color(color) -> tuple[int, int, int, int]:
    """标准化颜色格式，确保返回 RGBA 元组"""
    if isinstance(color, int):
        return (color, color, color, 255)
    if not isinstance(color, (list, tuple)) or len(color) not in (3, 4):
        raise ValueError(f"无效的颜色格式: {color}")
    return tuple(color) + (255,) if len(color) == 3 else tuple(color)

def load_font_with_bitmap_fallback(font_path: Path, size: int, log_prefix: str = "") -> tuple[ImageFont.FreeTypeFont, float]:
    """加载字体"""
    try:
        font = ImageFont.truetype(str(font_path), size)
        return font, 1.0
    except OSError as e:
        if 'invalid pixel size' not in str(e):
            logging.error(f"{log_prefix}加载字体失败 {font_path}: {e}")
            return ImageFont.load_default(), 1.0

        try:
            best_size, actual_size = get_bitmap_font_sizes(font_path, size)
            font = ImageFont.truetype(str(font_path), best_size)
            scale_factor = size / actual_size if actual_size > 0 else 1.0
            logging.info(f"{log_prefix}位图字体 `{font_path}` PPEM={best_size}, 实际={actual_size}, 缩放={scale_factor:.2f}")
            return font, scale_factor
        except Exception as e2:
            logging.error(f"{log_prefix}加载位图字体失败 {font_path}: {e2}")
            return ImageFont.load_default(), 1.0

def render_text_scaled(draw: ImageDraw.Draw, char: str, font: ImageFont.FreeTypeFont, 
                       pos: tuple[int, int], fill: tuple, scale_factor: float, 
                       img: Image.Image, center_x: int, text_x_offset: int) -> None:
    """渲染文本并自动缩放（用于位图字体），返回最终位置"""
    if scale_factor == 1.0:
        draw.text(pos, char, font=font, fill=fill, embedded_color=True)
        return pos

    bbox = draw.textbbox((0, 0), char, font=font)
    w, h = bbox[2] - bbox[0], bbox[3] - bbox[1]
    temp_size = int(max(w, h) * 1.5)
    temp_img = Image.new('RGBA', (temp_size, temp_size), (0, 0, 0, 0))
    temp_draw = ImageDraw.Draw(temp_img)
    temp_x, temp_y = (temp_size - w) // 2, (temp_size - h) // 2
    temp_draw.text((temp_x, temp_y), char, font=font, fill=fill, embedded_color=True)

    new_w, new_h = int(w * scale_factor), int(h * scale_factor)
    scaled_img = temp_img.resize((new_w, new_h), Image.Resampling.LANCZOS)
    paste_x = center_x - new_w // 2 + text_x_offset
    paste_y = pos[1] + (h - new_h) // 2
    img.paste(scaled_img, (paste_x, paste_y), scaled_img)
    
    scaled_img.close()
    temp_img.close()
    return (paste_x, paste_y)

def precompute_blend_colors(cfg: Config, bg_colors: list) -> tuple[dict, dict]:
    """预计算所有可能的混合颜色"""
    alpha = cfg.middle_font_color[3] / 255
    fg = cfg.middle_font_color[:3]
    overlay_alpha = alpha * 0.5

    blend_cache = {}
    overlay_cache = {}
    
    for bg_color in bg_colors:
        normalized_bg = normalize_color(bg_color)
        key = tuple(normalized_bg[:3])
        
        blend_cache[key] = fast_blend_colors(fg, key, alpha)
        overlay_cache[key] = fast_blend_colors(fg, key, overlay_alpha)

    return blend_cache, overlay_cache

def wrap_text(text: str, font: ImageFont.FreeTypeFont, max_width: int) -> str:
    """自动换行函数：将文本按最大宽度分割成多行"""
    lines = []
    original_lines = text.split('\n')
    
    for line in original_lines:
        if not line:
            lines.append('')
            continue
        
        # 尝试按单词分割（如果有空格），否则按字符分割
        if ' ' in line:
            words = line.split(' ')
        else:
            words = list(line)
        
        current_line = ''
        
        for word in words:
            # 如果当前行是空的，直接添加单词（即使单个单词超过宽度）
            if not current_line:
                test_line = word
            else:
                # 如果有空格分隔，需要添加空格
                separator = ' ' if ' ' in line else ''
                test_line = current_line + separator + word
            
            bbox = font.getbbox(test_line)
            if bbox is None:
                width = 0
            else:
                width = bbox[2] - bbox[0]
            
            if width <= max_width or not current_line:
                current_line = test_line
            else:
                lines.append(current_line)
                current_line = word
        
        if current_line:
            lines.append(current_line)
    
    return '\n'.join(lines)


def preprocess_descriptions(entries: list[UnicodeEntry], bottom_font: ImageFont.FreeTypeFont, max_width: int) -> dict[str, str]:
    """预处理所有描述文本"""
    processed = {}
    for entry in entries:
        if entry.description:
            desc = entry.description.replace('|', '\n').replace('";"', '\n')
            text = f"{entry.code_str}\n{desc}"
        else:
            text = entry.code_str
        
        processed[entry.code_str] = wrap_text(text, bottom_font, max_width)
    return processed

def preload_middle_fonts(entries: list[UnicodeEntry], cfg: Config) -> tuple[dict, dict, dict]:
    """预加载字体和度量信息"""
    paths = {entry.font_path for entry in entries}
    paths.update(cfg.font_files)

    font_cache: dict[Path, ImageFont.FreeTypeFont | None] = {}
    metrics_cache: dict[Path, tuple[int, int]] = {}
    font_scale_cache: dict[Path, float] = {}

    for p in paths:
        font, scale_factor = load_font_with_bitmap_fallback(p, cfg.middle_font_size, f"预加载字体 ")
        font_cache[p] = font
        metrics_cache[p] = font.getmetrics()
        font_scale_cache[p] = scale_factor

    return font_cache, metrics_cache, font_scale_cache

def write_batch(batch: list):
    """批量写入文件，优化磁盘I/O"""
    for data, path in batch:
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            with open(path, 'wb') as f:
                f.write(data)
        except Exception as e:
            logging.error(f"写入文件失败 {path}: {e}")

def optimized_writer_thread_fn(queue: Queue, batch_size: int = 50):
    """优化的写入线程，支持批量写入，增大batch_size减少磁盘I/O"""
    batch = []
    total_written = 0

    while True:
        item = queue.get()
        if item is None:
            if batch:
                write_batch(batch)
                total_written += len(batch)
                logging.info(f"写入线程完成，共写入 {total_written} 个文件")
            queue.task_done()
            break

        batch.append(item)
        if len(batch) >= batch_size:
            write_batch(batch)
            total_written += len(batch)
            batch.clear()

        queue.task_done()

def generate_image_bytes(
    entry: UnicodeEntry,
    cfg: Config,
    color_mgr: ColorManager | None,
    bottom_font: ImageFont.FreeTypeFont,
    ctrl_font: ImageFont.FreeTypeFont,
    ctrl_font_scale: float,
    middle_font_cache: dict[Path, ImageFont.FreeTypeFont | None],
    metrics_cache: dict[Path, tuple[int, int]],
    font_scale_cache: dict[Path, float],
    text_cache: dict[str, tuple[int, int]],
    desc_cache: dict[str, str],
    precomputed: PrecomputedValues,
    blend_cache: dict[tuple, tuple[int,int,int]],
    overlay_cache: dict[tuple, tuple[int,int,int]],
    overlay_enabled: bool,
    combining_cps: set[int],
    overlay_bbox_cache: dict[str, tuple[int, int]],
    use_fast_png: bool = False,
    colr_glyph_cache: dict[str, Image.Image] | None = None
) -> tuple[bytes, Path]:
    """图片生成函数，支持快速和标准模式"""
    try:
        cp = int(entry.code_str.strip()[2:], 16)
    except:
        raise ValueError(f"无效的 code_str: {entry.code_str!r}")

    char = get_char(cp)
    is_control = (cp in CTRLS)

    # 背景颜色
    if color_mgr is not None:
        bg_color_raw = color_mgr.get_color(entry.description)
        bg_color = normalize_color(bg_color_raw)
    else:
        bg_color = normalize_color(cfg.background_color)

    # 创建图像 - 快速模式使用RGB，标准模式使用RGBA
    if use_fast_png:
        img = Image.new('RGB', cfg.image_size, bg_color[:3])
    else:
        img = Image.new('RGBA', cfg.image_size, bg_color)
    draw = ImageDraw.Draw(img)

    # 选择字体
    if is_control:
        middle_font = ctrl_font
        font_path_key = 'ctrl'
    else:
        middle_font = middle_font_cache.get(entry.font_path)
        font_path_key = entry.font_path
        if not middle_font:
            for p in cfg.font_files:
                f = middle_font_cache.get(p)
                if f:
                    middle_font = f
                    font_path_key = p
                    break

    if not middle_font:
        char = "无法加载字体：" + char
        middle_font = ImageFont.load_default()
        font_path_key = 'default'

    # 使用缓存的度量信息
    if font_path_key in metrics_cache:
        ascent, descent = metrics_cache[font_path_key]
    else:
        ascent, descent = middle_font.getmetrics()
        if font_path_key != 'default':
            metrics_cache[font_path_key] = (ascent, descent)

    # 计算基线位置
    baseline_y = int(precomputed.center_y + (ascent - descent)/2) + precomputed.baseline_offset

    # 缓存文本尺寸计算
    if middle_font is not None and hasattr(middle_font, 'path'):
        cache_key = f"{char}_{middle_font.path}_{middle_font.size}"
    else:
        cache_key = f"{char}_{font_path_key}_{cfg.middle_font_size}"

    if cache_key in text_cache:
        w, h = text_cache[cache_key]
    else:
        bbox = draw.textbbox((0,0), char, font=middle_font)
        w = bbox[2] - bbox[0]
        h = bbox[3] - bbox[1]
        text_cache[cache_key] = (w, h)

    x = (precomputed.W - w)//2 + precomputed.text_x_offset
    y = baseline_y - ascent

    # 使用预计算的混合颜色
    bg_key = tuple(bg_color[:3])
    if bg_key in blend_cache:
        blended = blend_cache[bg_key]
    else:
        blended = fast_blend_colors(precomputed.fg_color, bg_key, precomputed.alpha)
        blend_cache[bg_key] = blended

    # 组合标记 overlay
    if overlay_enabled and cp in combining_cps:
        overlay_char = '\u25CC'
        overlay_cache_key = f"{overlay_char}_{font_path_key}_{cfg.middle_font_size}"

        if overlay_cache_key in overlay_bbox_cache:
            ow, oh = overlay_bbox_cache[overlay_cache_key]
        else:
            obbox = draw.textbbox((0,0), overlay_char, font=ctrl_font, embedded_color=True)
            ow = obbox[2] - obbox[0]
            oh = obbox[3] - obbox[1]
            overlay_bbox_cache[overlay_cache_key] = (ow, oh)

        if bg_key in overlay_cache:
            overlay_color = overlay_cache[bg_key]
        else:
            overlay_color = fast_blend_colors(precomputed.fg_color, bg_key, precomputed.overlay_alpha)
            overlay_cache[bg_key] = overlay_color

        ox = (precomputed.W - ow)//2 + precomputed.text_x_offset
        oy = baseline_y - ascent
        render_text_scaled(draw, overlay_char, ctrl_font, (ox, oy), overlay_color, ctrl_font_scale, 
                          img, precomputed.center_x, precomputed.text_x_offset)

    # 主字符
    if not is_control and (has_colr_table(entry.font_path) or has_svg_table(entry.font_path)):
        colr_cache_key = f"{entry.font_path}_{char}_{cfg.middle_font_size}"
        colr_result = colr_glyph_cache.get(colr_cache_key)
        if colr_result is None:
            colr_result = render_colr_glyph(entry.font_path, char, cfg.middle_font_size, precomputed.fg_color)
            if colr_result is not None:
                colr_glyph_cache[colr_cache_key] = colr_result
        if colr_result is not None:
            colr_img, baseline_offset = colr_result
            paste_x = (precomputed.W - colr_img.width) // 2 + precomputed.text_x_offset
            paste_y = baseline_y - baseline_offset
            img.paste(colr_img, (paste_x, paste_y), colr_img)
        else:
            render_text_scaled(draw, char, middle_font, (x, y), blended, 
                              font_scale_cache.get(font_path_key, 1.0), img, precomputed.center_x, precomputed.text_x_offset)
    else:
        render_text_scaled(draw, char, middle_font, (x, y), blended, 
                          font_scale_cache.get(font_path_key, 1.0), img, precomputed.center_x, precomputed.text_x_offset)

    # 底部文字 - 使用预处理的文本
    bottom_text = desc_cache[entry.code_str]
    
    # 计算文本行数，调整垂直位置使文字从底部向上显示
    line_count = bottom_text.count('\n') + 1
    adjusted_y = precomputed.bottom_text_y - (line_count - 1) * (cfg.bottom_font_size + 5)
    
    draw.multiline_text(
        (100, adjusted_y),
        bottom_text,
        font=bottom_font,
        fill=blended,
        align='left',
        spacing=5
    )

    # 保存到内存
    buf = BytesIO()
    buf.truncate(50000)
    buf.seek(0)

    if use_fast_png:
        img.save(buf, format='PNG', compress_level=1, optimize=False)
    else:
        img.save(buf, format='PNG', 
                 compress_level=cfg.png_compress_level, 
                 optimize=cfg.png_optimize,
                 pnginfo=None)
    data = buf.getvalue()

    img.close()
    buf.close()

    out_path = cfg.output_dir / f"image_{entry.code_str}.png"
    return data, out_path

def parse_args():
    parser = argparse.ArgumentParser(description='Unicode 字符图片生成器')
    parser.add_argument(
        '--dynamic-bg', 
        action='store_true',
        help='启用动态背景颜色，默认使用固定背景色'
    )
    parser.add_argument(
        '--workers',
        type=int,
        default=4,
        help='并发线程数，默认为 4'
    )
    parser.add_argument(
        '--write-batch-size',
        type=int,
        default=50,
        help='批量写入大小，默认为 50（增大可减少磁盘I/O）'
    )
    parser.add_argument(
        '--png-quality',
        choices=['fast', 'balanced', 'best'],
        default='balanced',
        help='PNG 压缩质量：fast(速度优先), balanced(平衡), best(质量优先)'
    )
    parser.add_argument(
        '--force',
        '-f',
        action='store_true',
        help='忽略已存在的 PNG，强制重新生成并覆盖'
    )
    parser.add_argument(
        '--disable-comb-overlay',
        action='store_true',
        help='禁用组合类标记(Mn/Mc/Me)的◌覆盖提示'
    )
    parser.add_argument(
        '--fast-png',
        action='store_true',
        help='使用快速PNG编码（压缩级别1，无优化），速度提升约2-3倍，文件稍大'
    )
    return parser.parse_args()

def main():
    args = parse_args()

    # 加载组合类标记
    def load_combining_marks(path: Path) -> set[int]:
        cps = set()
        try:
            for line in path.read_text(encoding='utf-8').splitlines():
                if not line or line.startswith('#'):
                    continue
                f = line.split(';')
                if len(f) < 3: 
                    continue
                cp = int(f[0], 16)
                if f[2] in ('Mn','Mc','Me'): 
                    cps.add(cp)
        except Exception as e:
            logging.error(f"读取 UnicodeData.txt 失败: {e}")
        return cps

    unicode_data_path = Path.cwd() / 'UnicodeData.txt'
    if unicode_data_path.exists():
        combining_cps = load_combining_marks(unicode_data_path)
        logging.info(f"加载 {len(combining_cps)} 个组合标记")
    else:
        combining_cps = set()
        logging.warning("UnicodeData.txt 未找到，组合 overlay 无效")

    overlay_enabled = not args.disable_comb_overlay
    setup_logging()
    cfg = Config()
    cfg.output_dir.mkdir(exist_ok=True)

    # PNG 质量设置
    if args.png_quality == 'fast':
        cfg.png_compress_level, cfg.png_optimize = 1, False
    elif args.png_quality == 'balanced':
        cfg.png_compress_level, cfg.png_optimize = 2, False
    else:
        cfg.png_compress_level, cfg.png_optimize = 6, True

    # 已存在 PNG
    existing = {p.stem.split('_')[-1] for p in cfg.output_dir.glob('*.png') if p.stat().st_size >= 1000}

    entries = load_unicode_entries(cfg.unicode_file)
    if args.force:
        to_process = entries
    else:
        to_process = [e for e in entries if e.code_str not in existing]

    if not to_process:
        logging.info("所有图片已存在，无需生成")
        return

    # 预计算和预处理
    precomputed = PrecomputedValues(cfg)

    # 动态背景和颜色混合缓存
    color_mgr = None
    if args.dynamic_bg:
        logging.info("启用动态背景模式")
        color_mgr = ColorManager(cfg.color_cycle, Path('color_state.json'))
        if not color_mgr.state_file.exists() or not color_mgr._mapping:
            color_mgr.build_initial_mapping(entries)

        # 预计算所有可能的背景色
        unique_colors = []
        if hasattr(color_mgr, '_mapping') and color_mgr._mapping:
            unique_colors.extend(color_mgr._mapping.values())
        if hasattr(color_mgr, 'color_cycle') and color_mgr.color_cycle:
            unique_colors.extend(color_mgr.color_cycle)
        unique_colors.append(cfg.background_color)

        # 去重
        seen = set()
        normalized_colors = []
        for color in unique_colors:
            normalized = normalize_color(color)
            color_key = tuple(normalized)
            if color_key not in seen:
                seen.add(color_key)
                normalized_colors.append(normalized)

        blend_cache, overlay_cache = precompute_blend_colors(cfg, normalized_colors)
    else:
        logging.info(f"固定背景: {cfg.background_color}")
        blend_cache, overlay_cache = precompute_blend_colors(cfg, [cfg.background_color])

    logging.info(f"需要生成 {len(to_process)} 张图片 (workers={args.workers}, png-quality={args.png_quality})")

    # 加载字体
    bottom_font = ImageFont.truetype(str(cfg.bottom_font_file), cfg.bottom_font_size)
    ctrl_font, ctrl_font_scale = load_font_with_bitmap_fallback(cfg.ctrl_font_file, cfg.middle_font_size, "Ctrl ")

    # 计算最大宽度（图像宽度减去左右边距）
    max_text_width = cfg.image_size[0] - 200
    desc_cache = preprocess_descriptions(to_process, bottom_font, max_text_width)

    middle_font_cache, metrics_cache, font_scale_cache = preload_middle_fonts(to_process, cfg)

    # 创建各种缓存
    text_cache: dict[str, tuple[int, int]] = {}
    overlay_bbox_cache: dict[str, tuple[int, int]] = {}
    colr_glyph_cache: dict[str, Image.Image] = {}

    # 优化的写入队列
    q: Queue = Queue(maxsize=200)
    writer = Thread(target=optimized_writer_thread_fn, args=(q, args.write_batch_size), daemon=True)
    writer.start()

    # 快速模式提示
    if args.fast_png:
        logging.info("使用快速PNG编码模式")

    start = time.time()
    with ThreadPoolExecutor(max_workers=args.workers) as pool, \
         tqdm(total=len(to_process), desc="生成图片", unit="项") as bar:
        futures = [
            pool.submit(
                generate_image_bytes,
                entry, cfg, color_mgr,
                bottom_font, ctrl_font, ctrl_font_scale,
                middle_font_cache, metrics_cache, font_scale_cache,
                text_cache, desc_cache, precomputed,
                blend_cache, overlay_cache,
                overlay_enabled, combining_cps,
                overlay_bbox_cache,
                args.fast_png,
                colr_glyph_cache
            )
            for entry in to_process
        ]

        for fut in as_completed(futures):
            try:
                data, path = fut.result()
                q.put((data, path))
            except Exception as e:
                logging.error(f"生成失败: {e}")
            finally:
                bar.update(1)

    q.join()
    q.put(None)
    writer.join()

    elapsed = time.time() - start
    fps = len(to_process)/elapsed if elapsed > 0 else float('inf')
    
    logging.info(f"完成，用时 {elapsed:.2f}s，{fps:.2f} 张/秒")
    logging.info(f"平均每张图片耗时 {elapsed/len(to_process)*1000:.2f}ms")
    
    if args.fast_png:
        logging.info("提示：使用快速PNG模式，文件大小会比标准模式大20-30%，但速度提升2-3倍")

if __name__ == '__main__':
    main()
