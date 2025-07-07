#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import sys
import time
import hashlib
import logging
import configparser
from dataclasses import dataclass
from pathlib import Path
from threading import Lock, Thread
from queue import Queue
from concurrent.futures import ThreadPoolExecutor, as_completed
from io import BytesIO

sys.path.insert(0, os.path.dirname(__file__))

from control_map import get_char, CTRLS
from PIL import Image, ImageDraw, ImageFont
from tqdm import tqdm


@dataclass
class Config:
    unicode_file: Path
    output_dir: Path
    font_files: list[Path]
    ctrl_font_file: Path
    bottom_font_file: Path
    middle_font_size: int
    bottom_font_size: int
    text_position: tuple[int, int]
    middle_font_color: tuple[int, int, int, int]
    image_size: tuple[int, int]
    background_color: tuple[int, int, int, int]

    @classmethod
    def load(cls, ini_path: Path) -> 'Config':
        parser = configparser.ConfigParser()
        parser.read(ini_path, encoding='utf-8')

        s = parser['Settings']
        def get_tuple(key, fallback, typ=int):
            return tuple(typ(x) for x in s.get(key, fallback).split(','))

        return Config(
            unicode_file=Path.cwd() / 'Unicode.txt',
            output_dir=Path.cwd() / 'png',
            font_files=[Path.cwd() / 'font.ttf'],
            ctrl_font_file=Path(s.get('ctrl_font_file', 'Ctrl-Ctrl.ttf')),
            bottom_font_file=Path.cwd() / 'PressStart2P-1.ttf',
            middle_font_size=int(s.get('middle_font_size', '512')),
            bottom_font_size=int(s.get('bottom_font_size', '19')),
            text_position=(
                int(s.get('text_position_x', '0')),
                int(s.get('text_position_y', '0'))
            ),
            middle_font_color=get_tuple('middle_font_color', '255,255,255,255'),
            image_size=(1920, 1080),
            background_color=get_tuple('background_color', '0,0,0,255')
        )


@dataclass
class UnicodeEntry:
    font_path: Path
    code_str: str
    description: str


def load_unicode_entries(path: Path) -> list[UnicodeEntry]:
    """
    读取新的 Unicode.txt 格式，每行格式：
      "font_path";"U+xxxx";"Description"
    去除空行，拆分三段，去除两端引号后返回 UnicodeEntry 列表。
    """
    entries: list[UnicodeEntry] = []
    for raw in path.read_text(encoding='utf-8').splitlines():
        line = raw.strip()
        if not line:
            continue
        parts = [p.strip().strip('"') for p in line.split(';', 2)]
        if len(parts) == 2:
            parts.append('')
        elif len(parts) != 3:
            raise ValueError(f"行格式错误（期望 2 或 3 段，用 ; 分隔）: {line!r}")
        font_path, code_str, desc = parts
        entries.append(UnicodeEntry(Path(font_path), code_str, desc))
    return entries


def setup_logging():
    logging.basicConfig(
        format='[%(asctime)s] - %(levelname)s - %(message)s',
        datefmt='%H:%M:%S',
        level=logging.INFO
    )


def blend_colors(fg_color, bg_color, alpha_ratio):
    """计算前景色与背景色的叠加结果"""
    r = int(fg_color[0] * alpha_ratio + bg_color[0] * (1 - alpha_ratio))
    g = int(fg_color[1] * alpha_ratio + bg_color[1] * (1 - alpha_ratio))
    b = int(fg_color[2] * alpha_ratio + bg_color[2] * (1 - alpha_ratio))
    return (r, g, b, 255)

def generate_image_bytes(
    entry: UnicodeEntry,
    cfg: Config,
    bottom_font: ImageFont.FreeTypeFont,
    ctrl_font: ImageFont.FreeTypeFont,
    middle_font_cache: dict[Path, ImageFont.FreeTypeFont],
    cache_lock: Lock
) -> tuple[bytes, Path]:
    """
    生成图片并返回 PNG bytes 以及目标路径，写入线程负责落盘。
    """
    try:
        cp = int(entry.code_str.strip()[2:], 16)
    except:
        raise ValueError(f"无效的 code_str: {entry.code_str!r}")

    char = get_char(cp)
    is_control = (cp in CTRLS)

    # 新建背景
    img = Image.new('RGBA', cfg.image_size, cfg.background_color)
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
    blended = blend_colors(fg, cfg.background_color[:3], alpha)
    draw.text((x,y), char, font=middle_font, fill=blended)

    # 底部文字
    if entry.description:
        desc = entry.description.replace('|', '\n')
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
    img.save(buf, format='PNG', compress_level=2)  # compress_level=2 较快, optimize=True
    data = buf.getvalue()

    img.close()
    buf.close()
    del img, draw, buf

    out_path = cfg.output_dir / f"image_{entry.code_str}.png"
    return data, out_path

def writer_thread_fn(q: Queue):
    """单线程顺序写入磁盘，减少 HDD 随机寻道。"""
    while True:
        item = q.get()
        if item is None:
            q.task_done()
            break
        data, out_path = item
        out_path.parent.mkdir(parents=True, exist_ok=True)
        with open(out_path, 'wb') as f:
            f.write(data)
        q.task_done()

def main():
    setup_logging()
    cfg = Config.load(Path('settings.ini'))
    cfg.output_dir.mkdir(exist_ok=True)

    # 解析已存在的 PNG 文件，记录已处理的代码点
    existing_entries = {}
    for png_file in cfg.output_dir.glob('*.png'):
        if png_file.stat().st_size >= 1000:
            code_str = png_file.stem.split('_')[-1]
            existing_entries[code_str] = png_file

    entries = load_unicode_entries(cfg.unicode_file)
    total = len(entries)
    logging.info(f"共读取 {total} 行，将开始生成图片。")

    # 过滤已存在的条目
    entries_to_process = [entry for entry in entries if entry.code_str not in existing_entries]

    bottom_font = ImageFont.truetype(str(cfg.bottom_font_file), cfg.bottom_font_size)
    try:
        ctrl_font = ImageFont.truetype(str(cfg.ctrl_font_file), cfg.middle_font_size)
    except OSError:
        logging.error(f"加载 Ctrl 字形专用字体失败：{cfg.ctrl_font_file}，将回退到默认字体")
        ctrl_font = ImageFont.load_default()

    middle_font_cache: dict[Path, ImageFont.FreeTypeFont | None] = {}

    # HDD 磁盘可以适当增加队列大小，减少写入频率
    q: Queue = Queue(maxsize=100)
    writer = Thread(target=writer_thread_fn, args=(q,), daemon=True)
    writer.start()

    cache_lock = Lock()
    start = time.time()
    with ThreadPoolExecutor(max_workers=4) as pool, tqdm(total=len(entries_to_process), desc="生成图片", unit="项") as bar:
        futures = [
            pool.submit(generate_image_bytes, entry, cfg, bottom_font, ctrl_font, middle_font_cache, cache_lock)
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