#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import sys
import time
import logging
import argparse
from pathlib import Path
from threading import Lock, Thread
from queue import Queue
from concurrent.futures import ThreadPoolExecutor, as_completed
from io import BytesIO

sys.path.insert(0, os.path.dirname(__file__))

from control_map import get_char, CTRLS
from Module import Config, UnicodeEntry, ColorManager, load_unicode_entries, setup_logging, blend_colors, writer_thread_fn

from PIL import Image, ImageDraw, ImageFont
from tqdm import tqdm

Image.MAX_IMAGE_PIXELS = None
# Image.MEMORY_LIMIT = 1024 * 1024 * 1024  # 1GB

def generate_image_bytes(
    entry: UnicodeEntry,
    cfg: Config,
    color_mgr: ColorManager | None,
    bottom_font: ImageFont.FreeTypeFont,
    ctrl_font: ImageFont.FreeTypeFont,
    middle_font_cache: dict[Path, ImageFont.FreeTypeFont],
    cache_lock: Lock
) -> tuple[bytes, Path]:
    """
    生成图片并返回 PNG bytes 以及目标路径，写入线程负责落盘。
    减少内存分配，提高性能
    """
    try:
        cp = int(entry.code_str.strip()[2:], 16)
    except:
        raise ValueError(f"无效的 code_str: {entry.code_str!r}")

    char = get_char(cp)
    is_control = (cp in CTRLS)

    # 确定背景颜色
    if color_mgr is not None:
        bg_color = color_mgr.get_color(entry.description)
    else:
        bg_color = cfg.background_color

    # 新建背景 - 使用 RGB 模式可能更快，如果不需要透明度
    if bg_color[3] == 255:
        img = Image.new('RGB', cfg.image_size, bg_color[:3])
    else:
        img = Image.new('RGBA', cfg.image_size, bg_color)
    
    draw = ImageDraw.Draw(img)

    # 中间字体（按顺序尝试加载支持的字体）
    if is_control:
        middle_font = ctrl_font
    else:
        with cache_lock:
            middle_font = None
            p = entry.font_path
            if p not in middle_font_cache:
                if not p.exists():
                    logging.warning(f"字体文件不存在: {p}，将使用备用字体")
                try:
                    middle_font_cache[p] = ImageFont.truetype(str(p), cfg.middle_font_size)
                except OSError as e:
                    logging.warning(f"加载字体失败 `{p}`: {e}，将使用备用字体")
                    middle_font_cache[p] = None
            if middle_font_cache[p]:
                middle_font = middle_font_cache[p]
            else:
                # 退回到全局备用列表
                for q in cfg.font_files:
                    if q not in middle_font_cache:
                        try:
                            middle_font_cache[q] = ImageFont.truetype(str(q), cfg.middle_font_size)
                        except OSError:
                            middle_font_cache[q] = None
                    if middle_font_cache[q]:
                        middle_font = middle_font_cache[q]
                        break

        if not middle_font:
            char = "无法加载字体：" + char
            middle_font = ImageFont.load_default()

    W, H = cfg.image_size
    ascent, descent = middle_font.getmetrics()
    baseline_y = int(H/2 + (ascent - descent)/2) + cfg.text_position[1]

    bbox = draw.textbbox((0,0), char, font=middle_font)
    w = bbox[2] - bbox[0]
    x = (W - w)//2 + cfg.text_position[0]
    y = baseline_y - ascent

    alpha = cfg.middle_font_color[3]/255
    fg = cfg.middle_font_color[:3]
    if img.mode == 'RGB':
        blended = blend_colors(fg, bg_color[:3], alpha)
    else:
        blended = blend_colors(fg, bg_color[:3], alpha)
    
    draw.text((x,y), char, font=middle_font, fill=blended)

    # 底部文字
    if entry.description:
        seps = ['|', '";"']
        desc = entry.description
        for ch in seps:
            desc = desc.replace(ch, '\n')
        bottom_text = f"{entry.code_str}\n{desc}"
    else:
        bottom_text = entry.code_str
    draw.multiline_text(
        (100, cfg.image_size[1] - cfg.bottom_font_size - 125),
        bottom_text,
        font=bottom_font,
        fill=blended
    )

    # 保存
    buf = BytesIO()
    # 使用较低的压缩级别以获得更快的保存速度
    # optimize=False 可以更快，但文件稍大
    img.save(buf, format='PNG', compress_level=1, optimize=False)
    data = buf.getvalue()

    # 清理资源
    img.close()
    buf.close()
    del img, draw, buf

    out_path = cfg.output_dir / f"image_{entry.code_str}.png"
    return data, out_path

def parse_args():
    """解析命令行参数"""
    parser = argparse.ArgumentParser(description='Unicode 字符图片生成器')
    parser.add_argument(
        '--dynamic-bg', 
        action='store_true',
        help='启用动态背景颜色（根据描述生成不同颜色背景），默认使用固定背景色'
    )
    parser.add_argument(
        '--workers',
        type=int,
        default=4,
        help='并发线程数，默认为 4'
    )
    parser.add_argument(
        '--png-quality',
        choices=['fast', 'balanced', 'best'],
        default='balanced',
        help='PNG 压缩质量：fast(速度优先), balanced(平衡), best(质量优先)'
    )
    return parser.parse_args()

def main():
    args = parse_args()
    
    setup_logging()
    cfg = Config()
    cfg.output_dir.mkdir(exist_ok=True)

    # 根据质量参数调整 PNG 设置
    if args.png_quality == 'fast':
        cfg.png_compress_level = 1
        cfg.png_optimize = False
    elif args.png_quality == 'balanced':
        cfg.png_compress_level = 2
        cfg.png_optimize = False
    else:  # best
        cfg.png_compress_level = 6
        cfg.png_optimize = True

    # 解析已存在的 PNG 文件，记录已处理的代码点
    existing_entries = {}
    for png_file in cfg.output_dir.glob('*.png'):
        if png_file.stat().st_size >= 1000:
            code_str = png_file.stem.split('_')[-1]
            existing_entries[code_str] = png_file

    entries = load_unicode_entries(cfg.unicode_file)
    total = len(entries)
    logging.info(f"共读取 {total} 行，将开始生成图片。")

    # 根据参数决定是否使用动态颜色管理器
    color_mgr = None
    if args.dynamic_bg:
        logging.info("启用动态背景颜色模式")
        color_mgr = ColorManager(cfg.color_cycle, Path('color_state.json'))
        
        # 检查是否需要构建初始映射
        if not color_mgr.state_file.exists() or len(color_mgr._mapping) == 0:
            logging.info("构建初始颜色映射...")
            color_mgr.build_initial_mapping(entries)
    else:
        logging.info(f"使用固定背景颜色模式: {cfg.background_color}")

    # 过滤已存在的条目
    entries_to_process = [entry for entry in entries if entry.code_str not in existing_entries]
    
    if not entries_to_process:
        logging.info("所有图片已存在，无需重新生成")
        return

    logging.info(f"需要生成 {len(entries_to_process)} 张图片（PNG 质量: {args.png_quality}）")

    bottom_font = ImageFont.truetype(str(cfg.bottom_font_file), cfg.bottom_font_size)
    try:
        ctrl_font = ImageFont.truetype(str(cfg.ctrl_font_file), cfg.middle_font_size)
    except OSError:
        logging.error(f"加载 Ctrl 字形专用字体失败：{cfg.ctrl_font_file}，将回退到默认字体")
        ctrl_font = ImageFont.load_default()

    middle_font_cache: dict[Path, ImageFont.FreeTypeFont | None] = {}

    # HDD 磁盘可以适当增加队列大小，减少写入频率
    q: Queue = Queue(maxsize=200)
    writer = Thread(target=writer_thread_fn, args=(q,), daemon=True)
    writer.start()

    cache_lock = Lock()
    start = time.time()
    
    with ThreadPoolExecutor(max_workers=args.workers) as pool, tqdm(total=len(entries_to_process), desc="生成图片", unit="项") as bar:
        futures = [
            pool.submit(generate_image_bytes, entry, cfg, color_mgr, bottom_font, ctrl_font, middle_font_cache, cache_lock)
            for entry in entries_to_process
        ]
        for fut in as_completed(futures):
            try:
                data, path = fut.result()
                q.put((data, path))
            except Exception as e:
                logging.error(f"生成行失败: {e}")
            finally:
                bar.update(1)

    q.join()
    q.put(None)
    writer.join()

    elapsed = time.time() - start
    fps = len(entries_to_process) / elapsed if elapsed > 0 else float('inf')
    logging.info(f"图片生成完成，用时 {elapsed:.2f}s，平均 {fps:.2f} 张/秒。")

if __name__ == '__main__':
    main()
