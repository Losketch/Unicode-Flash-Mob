#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import sys
import time
import logging
import argparse
import random
import colorsys
import math
import re
from pathlib import Path
from threading import Thread
from queue import Queue
from concurrent.futures import ThreadPoolExecutor
from io import BytesIO
from typing import Sequence, List, Tuple, Optional, Dict, Set

sys.path.insert(0, os.path.dirname(__file__))

from control_map import get_char, CTRLS
from Module import Config, UnicodeEntry, ColorManager, load_unicode_entries, setup_logging

from PIL import Image, ImageDraw, ImageFont
from tqdm import tqdm

Image.MAX_IMAGE_PIXELS = None

def write_batch(batch: list):
    """批量写入文件"""
    for data, path in batch:
        try:
            path.parent.mkdir(parents=True, exist_ok=True)
            with open(path, 'wb') as f:
                f.write(data)
        except Exception as e:
            logging.error(f"写入文件失败 {path}: {e}")

def optimized_writer_thread_fn(queue: Queue):
    """优化的写入线程，支持批量写入"""
    batch = []
    batch_size = 10

    while True:
        item = queue.get()
        if item is None:
            if batch:
                write_batch(batch)
            queue.task_done()
            break

        batch.append(item)
        if len(batch) >= batch_size:
            write_batch(batch)
            batch.clear()

        queue.task_done()

class ColorGradient:
    def __init__(self, key_colors: Optional[List[Tuple[int, int, int, int]]] = None,
                 cycle_length: int = 225):
        if key_colors is None:
            self.key_colors = [
                (255, 0, 0, 255),
                (255, 127, 0, 255),
                (255, 255, 0, 255),
                (0, 255, 0, 255),
                (0, 255, 255, 255),
                (0, 0, 255, 255),
                (127, 0, 255, 255)
            ]
        else:
            self.key_colors = key_colors

        self.cycle_length = cycle_length

        if self.key_colors and len(self.key_colors) > 1:
            self.extended_colors = self.key_colors + [self.key_colors[0]]
        else:
            self.extended_colors = self.key_colors

    def get_gradient_color(self, position: float) -> Tuple[int, int, int, int]:
        if not self.extended_colors or len(self.extended_colors) <= 1:
            return (255, 255, 255, 255) if not self.extended_colors else self.extended_colors[0]

        t = position / self.cycle_length
        t = t % 1.0

        segment = t * (len(self.key_colors) - 1)

        idx1 = int(math.floor(segment))
        idx2 = idx1 + 1

        if idx1 < 0:
            idx1 = 0
        if idx2 >= len(self.extended_colors):
            idx2 = len(self.extended_colors) - 1

        if idx1 == idx2:
            return self.extended_colors[idx1]

        ratio = segment - idx1

        color1 = self.extended_colors[idx1]
        color2 = self.extended_colors[idx2]

        r = int(color1[0] * (1 - ratio) + color2[0] * ratio)
        g = int(color1[1] * (1 - ratio) + color2[1] * ratio)
        b = int(color1[2] * (1 - ratio) + color2[2] * ratio)
        a = int(color1[3] * (1 - ratio) + color2[3] * ratio)

        return (r, g, b, a)

    def get_smooth_gradient_color(self, position: float) -> Tuple[int, int, int, int]:
        if not self.key_colors or len(self.key_colors) <= 1:
            return (255, 255, 255, 255) if not self.key_colors else self.key_colors[0]

        t = position / self.cycle_length
        angle = t * 2 * math.pi
        hue = (angle / (2 * math.pi)) % 1.0

        saturation = 1.0
        value = 1.0

        rgb = colorsys.hsv_to_rgb(hue, saturation, value)

        r = int(rgb[0] * 255)
        g = int(rgb[1] * 255)
        b = int(rgb[2] * 255)
        a = 255

        return (r, g, b, a)

class PositionAnimator:
    def __init__(self, total_images: int, animation_type: str = "smooth",
                 amplitude: int = 50, speed: float = 0.1,
                 movement_speed: float = None):
        self.total_images = total_images
        self.animation_type = animation_type
        self.amplitude = amplitude
        self.speed = (speed if movement_speed is None else movement_speed) * 0.01

        self.phase_offsets = {
            'code': random.random() * 2 * math.pi,
            'name': random.random() * 2 * math.pi,
            'block': random.random() * 2 * math.pi,
            'font': random.random() * 2 * math.pi,
            'content': random.random() * 2 * math.pi
        }

        self.frequency_offsets = {
            'code': random.uniform(0.9, 1.1),
            'name': random.uniform(0.9, 1.1),
            'block': random.uniform(0.9, 1.1),
            'font': random.uniform(0.9, 1.1),
            'content': random.uniform(0.9, 1.1)
        }

    def get_position_offset(self, index: int, base_x: int, base_y: int,
                            element_type: str = "text") -> Tuple[int, int]:
        if self.animation_type == "none":
            return (base_x, base_y)

        phase_offset = self.phase_offsets.get(element_type, random.random() * 2 * math.pi)
        frequency = self.frequency_offsets.get(element_type, 1.0)

        t = index * self.speed * frequency + phase_offset

        if self.animation_type == "smooth":
            dx = int(math.sin(t) * self.amplitude)
            dy = int(math.cos(t * 0.7) * self.amplitude * 0.8)
        elif self.animation_type == "random_smooth":
            dx = int(math.sin(t * 1.3) * self.amplitude * 0.5 +
                     math.cos(t * 0.5) * self.amplitude * 0.3 +
                     math.sin(t * 0.2) * self.amplitude * 0.2)
            dy = int(math.cos(t * 0.9) * self.amplitude * 0.4 +
                     math.sin(t * 0.3) * self.amplitude * 0.4 +
                     math.cos(t * 0.1) * self.amplitude * 0.2)
        else:
            dx = 0
            dy = 0

        max_offset = self.amplitude * 2
        dx = max(-max_offset, min(max_offset, dx))
        dy = max(-max_offset, min(max_offset, dy))

        return (base_x + dx, base_y + dy)

class NamesListParser:
    def __init__(self, path: Path):
        self.path = path
        self.entries: Dict[str, List[str]] = {}
        self._parse_file()

    def _parse_file(self):
        if not self.path.exists():
            logging.warning(f"NamesList.txt 未找到: {self.path}")
            return

        current_code = None
        current_lines = []

        try:
            content = self.path.read_text(encoding='utf-8')
            lines = content.splitlines()

            for line in lines:
                line = line.rstrip('\n')

                if len(line) >= 4 and line[0:4].isalnum():
                    if current_code and current_lines:
                        self.entries[current_code] = current_lines

                    parts = line.split('\t', 1)
                    if len(parts) > 0:
                        current_code = parts[0].strip().upper()
                        current_lines = [line]
                elif current_code is not None:
                    current_lines.append(line)

            if current_code and current_lines:
                self.entries[current_code] = current_lines

            logging.info(f"加载 NamesList.txt: {len(self.entries)} 个条目")

        except Exception as e:
            logging.error(f"解析 NamesList.txt 失败: {e}")

    def get_info_for_code(self, code_str: str) -> List[str]:
        if code_str.startswith("U+"):
            hex_code = code_str[2:].upper().zfill(4)
        else:
            hex_code = code_str.upper().zfill(4)

        return self.entries.get(hex_code, [])

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
        self.fg_color = tuple(int(c) for c in cfg.middle_font_color[:3])
        self.overlay_alpha = self.alpha * 0.5

        self.padding = 20
        self.top_left = (self.padding, self.padding)
        self.top_y = self.padding
        self.block_name_y = self.H - cfg.bottom_font_size * 2 - self.padding
        self.block_name_pos = (self.padding, self.block_name_y)
        self.font_name_y = self.H - cfg.bottom_font_size - self.padding
        self.font_name_pos = (self.padding, self.font_name_y)

        self.info_right_margin = self.padding
        self.info_bottom_margin = self.padding
        self.info_line_height = cfg.bottom_font_size + 2
        self.info_max_width = self.W // 3

        self.info_bottom_y = self.font_name_y + cfg.bottom_font_size

def fast_blend_colors(fg: Sequence[int], bg: Sequence[int], alpha: float) -> tuple[int, int, int]:
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
        c = int(color)
        return (c, c, c, 255)
    elif isinstance(color, (list, tuple)):
        if len(color) == 3:
            return (int(color[0]), int(color[1]), int(color[2]), 255)
        elif len(color) == 4:
            return (int(color[0]), int(color[1]), int(color[2]), int(color[3]))
        else:
            raise ValueError(f"无效的颜色格式: {color}")
    else:
        raise ValueError(f"不支持的颜色类型: {type(color)}")

def get_random_color() -> tuple[int, int, int, int]:
    return (
        random.randint(0, 255),
        random.randint(0, 255),
        random.randint(0, 255),
        255
    )

def precompute_blend_colors(cfg: Config, bg_colors: list) -> tuple[dict, dict]:
    """预计算所有可能的混合颜色"""
    alpha = cfg.middle_font_color[3] / 255
    fg = tuple(int(c) for c in cfg.middle_font_color[:3])
    overlay_alpha = alpha * 0.5

    blend_cache: dict[tuple[int, int, int], tuple[int, int, int]] = {}
    overlay_cache: dict[tuple[int, int, int], tuple[int, int, int]] = {}

    for bg_color in bg_colors:
        normalized_bg = normalize_color(bg_color)
        key = (int(normalized_bg[0]), int(normalized_bg[1]), int(normalized_bg[2]))

        blend_cache[key] = fast_blend_colors(fg, key, alpha)
        overlay_cache[key] = fast_blend_colors(fg, key, overlay_alpha)

    return blend_cache, overlay_cache

def parse_color_list(color_str: str) -> List[Tuple[int, int, int, int]]:
    colors = []
    if not color_str:
        return colors

    for color_part in color_str.split(';'):
        parts = color_part.split(',')
        if len(parts) >= 3:
            r = int(parts[0].strip())
            g = int(parts[1].strip())
            b = int(parts[2].strip())
            a = int(parts[3].strip()) if len(parts) >= 4 else 255
            colors.append((r, g, b, a))

    return colors

def load_unicode_blocks(path: Path) -> list[tuple[int, int, str]]:
    blocks: list[tuple[int, int, str]] = []
    try:
        text = path.read_text(encoding='utf-8')
        for line in text.splitlines():
            line = line.strip()
            if not line or line.startswith('#'):
                continue
            if ';' not in line:
                continue
            range_part, name = line.split(';', 1)
            name = name.strip()
            if '..' not in range_part:
                continue
            start_s, end_s = range_part.split('..', 1)
            try:
                start = int(start_s, 16)
                end = int(end_s, 16)
                blocks.append((start, end, name))
            except Exception:
                continue
    except Exception:
        return []
    blocks.sort(key=lambda x: x[0])
    return blocks

def find_block_name(cp: int, blocks: list[tuple[int, int, str]]) -> str:
    for start, end, name in blocks:
        if start <= cp <= end:
            return name
    return 'No_Block'

def preload_middle_fonts(entries: list[UnicodeEntry], cfg: Config) -> tuple[dict, dict]:
    """预加载字体和度量信息"""
    paths = {entry.font_path for entry in entries}
    paths.update(cfg.font_files)

    font_cache: dict[Path, ImageFont.FreeTypeFont | None] = {}
    metrics_cache: dict[Path, tuple[int, int]] = {}

    for p in paths:
        try:
            font = ImageFont.truetype(str(p), cfg.middle_font_size)
            font_cache[p] = font
            metrics_cache[p] = font.getmetrics()
        except Exception as e:
            logging.warning(f"预加载字体失败 `{p}`: {e}")
            font_cache[p] = None

    return font_cache, metrics_cache

def load_unicode_names(path: Path) -> dict[int, str]:
    names: dict[int, str] = {}
    try:
        text = path.read_text(encoding='utf-8')
        for line in text.splitlines():
            line = line.strip()
            if not line or line.startswith('#'):
                continue
            parts = line.split(';')
            if len(parts) < 2:
                continue
            try:
                cp = int(parts[0], 16)
            except Exception:
                continue
            name = parts[1].strip()
            names[cp] = name
    except Exception:
        return {}
    return names

def calculate_lines_needed(text: str, max_width: int, font: ImageFont.FreeTypeFont) -> int:
    temp_img = Image.new('RGBA', (100, 100), (0, 0, 0, 0))
    temp_draw = ImageDraw.Draw(temp_img)

    words = text.split()
    current_line_width = 0
    lines = 1

    for word in words:
        bbox = temp_draw.textbbox((0, 0), word + ' ', font=font)
        word_width = bbox[2] - bbox[0]

        if current_line_width + word_width > max_width:
            lines += 1
            current_line_width = word_width
        else:
            current_line_width += word_width

    return lines

def render_info_text_simple(
        draw: ImageDraw.Draw,
        text: str,
        start_x: int,
        start_y: int,
        max_width: int,
        line_height: int,
        base_font: ImageFont.FreeTypeFont,
        fg_color: Tuple[int, int, int]
) -> int:
    x = start_x
    y = start_y

    words = text.split()
    for word in words:
        word_with_space = word + ' '
        bbox = draw.textbbox((0, 0), word_with_space, font=base_font)
        word_width = bbox[2] - bbox[0]

        if x + word_width > start_x + max_width:
            x = start_x
            y += line_height

        draw.text((x, y), word_with_space, font=base_font, fill=fg_color)
        x += word_width

    return y

def check_bounds_with_padding(x: int, y: int, width: int, height: int,
                              padding: int, canvas_width: int, canvas_height: int) -> Tuple[int, int]:
    if x + width > canvas_width - padding:
        x = canvas_width - padding - width
    if x < padding:
        x = padding
    if y + height > canvas_height - padding:
        y = canvas_height - padding - height
    if y < padding:
        y = padding

    return x, y

def generate_image_bytes(
        entry: UnicodeEntry,
        cfg: Config,
        color_mgr: ColorManager | None,
        bottom_font: ImageFont.FreeTypeFont,
        ctrl_font: ImageFont.FreeTypeFont,
        middle_font_cache: dict[Path, ImageFont.FreeTypeFont | None],
        metrics_cache: dict[Path, tuple[int, int]],
        text_cache: dict[str, tuple[int, int]],
        precomputed: PrecomputedValues,
        blend_cache: dict[tuple, tuple[int, int, int]],
        overlay_cache: dict[tuple, tuple[int, int, int]],
        overlay_enabled: bool,
        combining_cps: set[int],
        overlay_bbox_cache: dict[str, tuple[int, int]],
        blocks: list[tuple[int, int, str]],
        unicode_names: dict[int, str],
        names_list_parser: NamesListParser,
        random_color: bool = False,
        filename_mapping: dict[str, str] = None,
        gradient_manager: Optional[ColorGradient] = None,
        gradient_index: int = 0,
        gradient_total: int = 1,
        flash_color: Optional[Tuple[int, int, int, int]] = None,
        position_animator: Optional[PositionAnimator] = None,
        animated_elements: List[str] = None,
        content_position_random: bool = False,
        content_position_fixed: Optional[Tuple[int, int]] = None,
        show_names_info: bool = False,
        use_smooth_gradient: bool = True,
) -> tuple[bytes, Path, str | None]:
    """图片生成函数"""
    try:
        cp = int(entry.code_str.strip()[2:], 16)
    except:
        raise ValueError(f"无效的 code_str: {entry.code_str!r}")

    char = get_char(cp)
    is_control = (cp in CTRLS)

    if flash_color:
        bg_color = flash_color
    elif gradient_manager:
        if use_smooth_gradient:
            bg_color = gradient_manager.get_smooth_gradient_color(gradient_index)
        else:
            bg_color = gradient_manager.get_gradient_color(gradient_index)
    elif random_color:
        bg_color = get_random_color()
    elif color_mgr is not None:
        bg_color_raw = color_mgr.get_color(entry.description)
        bg_color = normalize_color(bg_color_raw)
    else:
        bg_color = normalize_color(cfg.background_color)

    img = Image.new('RGBA', cfg.image_size, bg_color)
    draw = ImageDraw.Draw(img)

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

    if font_path_key in metrics_cache:
        ascent, descent = metrics_cache[font_path_key]
    else:
        ascent, descent = middle_font.getmetrics()
        if font_path_key != 'default':
            metrics_cache[font_path_key] = (ascent, descent)

    baseline_y = int(precomputed.center_y + (ascent - descent) / 2) + precomputed.baseline_offset

    final_text_x_offset = precomputed.text_x_offset
    final_baseline_offset = precomputed.baseline_offset

    if content_position_random:
        final_text_x_offset = random.randint(-200, 200)
        final_baseline_offset = random.randint(-100, 100)
    elif content_position_fixed:
        final_text_x_offset, final_baseline_offset = content_position_fixed

    anim_x, anim_y = 0, 0
    if position_animator and animated_elements and 'content' in animated_elements:
        anim_x, anim_y = position_animator.get_position_offset(
            gradient_index, 0, 0, 'content'
        )
        final_text_x_offset += anim_x
        final_baseline_offset += anim_y

    baseline_y = int(precomputed.center_y + (ascent - descent) / 2) + final_baseline_offset

    if hasattr(middle_font, 'path'):
        cache_key = f"{char}_{middle_font.path}_{middle_font.size}"
    else:
        cache_key = f"{char}_{font_path_key}_{cfg.middle_font_size}"

    if cache_key in text_cache:
        w, h = text_cache[cache_key]
    else:
        bbox = draw.textbbox((0, 0), char, font=middle_font)
        w = bbox[2] - bbox[0]
        h = bbox[3] - bbox[1]
        text_cache[cache_key] = (int(w), int(h))

    x = (precomputed.W - w) // 2 + final_text_x_offset
    y = baseline_y - ascent

    x, y = check_bounds_with_padding(x, y, w, h, precomputed.padding, precomputed.W, precomputed.H)

    main_char_right_edge = x + w

    bg_key = tuple(bg_color[:3])
    if bg_key in blend_cache:
        blended = blend_cache[bg_key]
    else:
        blended = fast_blend_colors(precomputed.fg_color, bg_key, precomputed.alpha)
        if not random_color and not gradient_manager and not flash_color:
            blend_cache[bg_key] = blended

    if overlay_enabled and cp in combining_cps:
        overlay_char = '\u25CC'
        overlay_cache_key = f"{overlay_char}_{font_path_key}_{cfg.middle_font_size}"

        if overlay_cache_key in overlay_bbox_cache:
            ow, oh = overlay_bbox_cache[overlay_cache_key]
        else:
            obbox = draw.textbbox((0, 0), overlay_char, font=ctrl_font)
            ow = obbox[2] - obbox[0]
            oh = obbox[3] - obbox[1]
            overlay_bbox_cache[overlay_cache_key] = (int(ow), int(oh))

        if bg_key in overlay_cache:
            overlay_color = overlay_cache[bg_key]
        else:
            overlay_color = fast_blend_colors(precomputed.fg_color, bg_color, precomputed.overlay_alpha)
            if not random_color and not gradient_manager and not flash_color:
                overlay_cache[bg_key] = overlay_color

        ox = (precomputed.W - ow) // 2 + final_text_x_offset
        oy = baseline_y - ascent
        ox, oy = check_bounds_with_padding(ox, oy, ow, oh, precomputed.padding, precomputed.W, precomputed.H)
        draw.text((ox, oy), overlay_char, font=ctrl_font, fill=overlay_color)

    draw.text((x, y), char, font=middle_font, fill=blended)

    code_text = entry.code_str
    top_left_x, top_left_y = precomputed.top_left

    code_bbox = draw.textbbox((0, 0), code_text, font=bottom_font)
    code_w = code_bbox[2] - code_bbox[0]
    code_h = code_bbox[3] - code_bbox[1]

    if position_animator and animated_elements and 'code' in animated_elements:
        top_left_x, top_left_y = position_animator.get_position_offset(
            gradient_index, top_left_x, top_left_y, 'code'
        )

    top_left_x, top_left_y = check_bounds_with_padding(
        top_left_x, top_left_y, code_w, code_h,
        precomputed.padding, precomputed.W, precomputed.H
    )

    draw.text((top_left_x, top_left_y), code_text, font=bottom_font, fill=blended)

    name_text = ''
    if unicode_names and cp in unicode_names:
        name_text = unicode_names[cp]
    else:
        name_text = (entry.description.split('|', 1)[0].strip() if entry.description else '')

    if name_text:
        name_bbox = draw.textbbox((0, 0), name_text, font=bottom_font)
        name_w = name_bbox[2] - name_bbox[0]
        name_h = name_bbox[3] - name_bbox[1]
        name_x = precomputed.W - name_w - precomputed.padding
        name_y = precomputed.top_y

        if position_animator and animated_elements and 'name' in animated_elements:
            name_x, name_y = position_animator.get_position_offset(
                gradient_index, name_x, name_y, 'name'
            )

        name_x, name_y = check_bounds_with_padding(
            name_x, name_y, name_w, name_h,
            precomputed.padding, precomputed.W, precomputed.H
        )

        draw.text((name_x, name_y), name_text, font=bottom_font, fill=blended)

    block_name = find_block_name(cp, blocks) if blocks else 'No_Block'
    block_name_x, block_name_y = precomputed.block_name_pos

    block_bbox = draw.textbbox((0, 0), block_name, font=bottom_font)
    block_w = block_bbox[2] - block_bbox[0]
    block_h = block_bbox[3] - block_bbox[1]

    if position_animator and animated_elements and 'block' in animated_elements:
        block_name_x, block_name_y = position_animator.get_position_offset(
            gradient_index, block_name_x, block_name_y, 'block'
        )

    block_name_x, block_name_y = check_bounds_with_padding(
        block_name_x, block_name_y, block_w, block_h,
        precomputed.padding, precomputed.W, precomputed.H
    )

    draw.text((block_name_x, block_name_y), block_name, font=bottom_font, fill=blended)

    try:
        if font_path_key == 'ctrl':
            font_file_name = Path(cfg.ctrl_font_file).name
        elif font_path_key == 'default':
            font_file_name = 'default'
        else:
            font_file_name = Path(str(font_path_key)).name
    except Exception:
        font_file_name = str(font_path_key)

    max_w = precomputed.W // 3
    fname = font_file_name
    bbox = draw.textbbox((0, 0), fname, font=bottom_font)
    fname_w = bbox[2] - bbox[0]
    fname_h = bbox[3] - bbox[1]
    if fname_w > max_w:
        name_body = fname
        while True:
            if len(name_body) <= 4:
                name_body = name_body[:4]
                fname = name_body
                break
            keep = max(1, len(name_body) - 6)
            name_candidate = '...' + name_body[-keep:]
            bbox = draw.textbbox((0, 0), name_candidate, font=bottom_font)
            fname_w = bbox[2] - bbox[0]
            if fname_w <= max_w:
                fname = name_candidate
                break
            name_body = name_body[1:]

    font_name_x, font_name_y = precomputed.font_name_pos
    if position_animator and animated_elements and 'font' in animated_elements:
        font_name_x, font_name_y = position_animator.get_position_offset(
            gradient_index, font_name_x, font_name_y, 'font'
        )

    font_name_x, font_name_y = check_bounds_with_padding(
        font_name_x, font_name_y, fname_w, fname_h,
        precomputed.padding, precomputed.W, precomputed.H
    )

    draw.text((font_name_x, font_name_y), fname, font=bottom_font, fill=blended)

    if show_names_info:
        info_lines = names_list_parser.get_info_for_code(entry.code_str)
        if info_lines and len(info_lines) > 1:
            info_start_x = precomputed.W - precomputed.info_max_width - precomputed.info_right_margin

            if info_start_x < main_char_right_edge + precomputed.padding:
                info_start_x = main_char_right_edge + precomputed.padding

            lines_to_show = []
            total_lines_needed = 0

            for i, line in enumerate(info_lines[1:], 1):
                line = line.strip()
                if not line:
                    continue

                while line.startswith('\t') or line.startswith(' '):
                    line = line[1:]

                if not line:
                    continue

                lines_needed = calculate_lines_needed(line, precomputed.info_max_width, bottom_font)
                total_lines_needed += lines_needed

                lines_to_show.append((line, lines_needed))

            if lines_to_show:
                total_display_height = total_lines_needed * precomputed.info_line_height

                start_y = precomputed.info_bottom_y - total_display_height

                if start_y < precomputed.padding:
                    start_y = precomputed.padding

                info_max_height = precomputed.info_bottom_y - precomputed.padding
                if total_display_height > info_max_height:
                    max_lines = int(info_max_height / precomputed.info_line_height)
                    if max_lines > 0:
                        lines_to_show = lines_to_show[:max_lines]
                        total_display_height = sum(
                            lines_needed for _, lines_needed in lines_to_show) * precomputed.info_line_height
                        start_y = precomputed.info_bottom_y - total_display_height

                if info_start_x < precomputed.padding:
                    info_start_x = precomputed.padding
                elif info_start_x + precomputed.info_max_width > precomputed.W - precomputed.padding:
                    info_start_x = precomputed.W - precomputed.padding - precomputed.info_max_width

                current_y = start_y

                for line_text, lines_needed in lines_to_show:
                    end_y = render_info_text_simple(
                        draw=draw,
                        text=line_text,
                        start_x=info_start_x,
                        start_y=current_y,
                        max_width=precomputed.info_max_width,
                        line_height=precomputed.info_line_height,
                        base_font=bottom_font,
                        fg_color=blended
                    )

                    current_y = end_y + precomputed.info_line_height

    buf = BytesIO()
    buf.truncate(50000)
    buf.seek(0)

    img.save(buf, format='PNG',
             compress_level=cfg.png_compress_level,
             optimize=cfg.png_optimize,
             pnginfo=None)
    data = buf.getvalue()

    img.close()
    buf.close()

    if filename_mapping and entry.code_str in filename_mapping:
        output_filename = filename_mapping[entry.code_str]
    else:
        output_filename = entry.code_str

    out_path = cfg.output_dir / f"image_{output_filename}.png"
    return data, out_path, None

def parse_args():
    parser = argparse.ArgumentParser(description='Unicode 字符图片生成器')
    parser.add_argument(
        '--dynamic-bg',
        action='store_true',
        help='启用动态背景颜色，默认使用固定背景色'
    )
    parser.add_argument(
        '--random-color',
        action='store_true',
        help='使生成的图片每一张颜色完全随机'
    )
    parser.add_argument(
        '--rainbow-gradient',
        action='store_true',
        help='启用彩虹渐变背景（按文件名顺序平滑渐变）'
    )
    parser.add_argument(
        '--gradient-colors',
        type=str,
        default='',
        help='自定义渐变关键颜色，格式：R,G,B,A;R,G,B,A;...'
    )
    parser.add_argument(
        '--gradient-cycle',
        type=int,
        default=100,
        help='彩虹渐变一轮回需要的图片张数，默认100张'
    )
    parser.add_argument(
        '--smooth-gradient',
        action='store_true',
        default=True,
        help='使用平滑渐变（默认启用），禁用则使用关键颜色插值'
    )
    parser.add_argument(
        '--flash-color',
        type=str,
        default='',
        help='爆闪自定义闪出颜色，格式：R,G,B,A 例如：255,255,255,255（不传则随机）'
    )
    parser.add_argument(
        '--animate-elements',
        type=str,
        default='',
        help='需要动画飘动的信息元素，逗号分隔，可选：code,name,block,font,content'
    )
    parser.add_argument(
        '--animation-type',
        choices=['smooth', 'random_smooth'],
        default='smooth',
        help='动画类型：smooth(平滑正弦), random_smooth(随机平滑)'
    )
    parser.add_argument(
        '--animation-amplitude',
        type=int,
        default=50,
        help='动画飘动幅度（像素）'
    )
    parser.add_argument(
        '--animation-speed',
        type=float,
        default=1.0,
        help='动画颜色变化速度系数（值越大颜色变化越快）'
    )
    parser.add_argument(
        '--movement-speed',
        type=float,
        default=0.1,
        help='飘动动画速度（值越大飘动越快），默认0.1'
    )
    parser.add_argument(
        '--content-position',
        type=str,
        default='',
        help='内容位置：fixed,x,y 或 random 例如：fixed,100,-50 或 random'
    )
    parser.add_argument(
        '--shuffle-content',
        action='store_true',
        help='内容按原顺序生成，但文件名在列表内随机分配'
    )
    parser.add_argument(
        '--show-names-info',
        action='store_true',
        help='在右下角显示NamesList.txt中的信息'
    )
    parser.add_argument(
        '--workers',
        type=int,
        default=8,
        help='并发线程数，默认为 8'
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
    return parser.parse_args()

def create_filename_mapping(entries: list[UnicodeEntry]) -> dict[str, str]:
    """创建文件名映射：原文件名 -> 随机文件名"""
    original_filenames = [entry.code_str for entry in entries]
    shuffled_filenames = original_filenames.copy()
    random.shuffle(shuffled_filenames)

    filename_mapping = {}
    for i, original in enumerate(original_filenames):
        filename_mapping[original] = shuffled_filenames[i]

    logging.info(f"创建文件名映射: {len(filename_mapping)} 个文件")
    if len(filename_mapping) > 0:
        sample_keys = list(filename_mapping.keys())[:3]
        for key in sample_keys:
            logging.info(f"  示例: {key} -> {filename_mapping[key]}")

    return filename_mapping

def parse_content_position(position_str: str):
    if not position_str:
        return None, None

    if position_str.lower() == 'random':
        return 'random', None

    parts = position_str.split(',')
    if len(parts) >= 3 and parts[0].lower() == 'fixed':
        try:
            x = int(parts[1])
            y = int(parts[2])
            return 'fixed', (x, y)
        except ValueError:
            logging.warning(f"无法解析固定位置参数: {position_str}")

    return None, None

def main():
    args = parse_args()

    random.seed()

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
                if f[2] in ('Mn', 'Mc', 'Me'):
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

    if args.png_quality == 'fast':
        cfg.png_compress_level, cfg.png_optimize = 1, False
    elif args.png_quality == 'balanced':
        cfg.png_compress_level, cfg.png_optimize = 2, False
    else:
        cfg.png_compress_level, cfg.png_optimize = 6, True

    existing_files = {p.stem.split('_')[-1] for p in cfg.output_dir.glob('*.png') if p.stat().st_size >= 1000}

    entries = load_unicode_entries(cfg.unicode_file)

    filename_mapping = None
    if args.shuffle_content:
        logging.info("应用文件名随机化：内容按原顺序生成，但文件名随机分配")
        filename_mapping = create_filename_mapping(entries)

    if args.force:
        to_process = entries
    else:
        to_process = []
        for e in entries:
            if filename_mapping and e.code_str in filename_mapping:
                mapped_name = filename_mapping[e.code_str]
                if mapped_name not in existing_files:
                    to_process.append(e)
            else:
                if e.code_str not in existing_files:
                    to_process.append(e)

    if not to_process:
        logging.info("所有图片已存在，无需生成")
        return

    gradient_manager = None
    if args.rainbow_gradient:
        if args.gradient_colors:
            key_colors = parse_color_list(args.gradient_colors)
            if key_colors:
                gradient_manager = ColorGradient(key_colors, args.gradient_cycle)
                logging.info(f"使用自定义渐变颜色: {len(key_colors)} 个关键色，{args.gradient_cycle}张一轮回")
            else:
                gradient_manager = ColorGradient(cycle_length=args.gradient_cycle)
                logging.info(f"使用默认彩虹渐变，{args.gradient_cycle}张一轮回")
        else:
            gradient_manager = ColorGradient(cycle_length=args.gradient_cycle)
            logging.info(f"使用默认彩虹渐变，{args.gradient_cycle}张一轮回")

        if args.smooth_gradient:
            logging.info("使用平滑渐变（HSV色彩空间）")
        else:
            logging.info("使用关键颜色插值渐变")

    flash_color = None
    if args.flash_color:
        colors = parse_color_list(args.flash_color)
        if colors:
            flash_color = colors[0]
            logging.info(
                f"使用闪出颜色: R={flash_color[0]}, G={flash_color[1]}, B={flash_color[2]}, A={flash_color[3]}")
        else:
            flash_color = get_random_color()
            logging.info("使用随机闪出颜色")
    elif args.flash_color == '' and (args.random_color or args.dynamic_bg or args.rainbow_gradient):
        pass
    else:
        flash_color = get_random_color()
        logging.info("使用随机闪出颜色")

    animated_elements = []
    if args.animate_elements:
        animated_elements = [elem.strip() for elem in args.animate_elements.split(',')]
        valid_elements = ['code', 'name', 'block', 'font', 'content']
        animated_elements = [elem for elem in animated_elements if elem in valid_elements]
        if animated_elements:
            logging.info(f"启用元素动画: {', '.join(animated_elements)}")

    position_animator = None
    if animated_elements:
        position_animator = PositionAnimator(
            total_images=len(to_process),
            animation_type=args.animation_type,
            amplitude=args.animation_amplitude,
            speed=args.animation_speed,
            movement_speed=args.movement_speed
        )
        logging.info(
            f"动画设置: 类型={args.animation_type}, 幅度={args.animation_amplitude}, 飘动速度={args.movement_speed}")
        logging.info(f"颜色变化速度系数: {args.animation_speed}")

    content_position_type, content_position_value = parse_content_position(args.content_position)
    content_position_random = (content_position_type == 'random')
    content_position_fixed = content_position_value if content_position_type == 'fixed' else None

    if content_position_random:
        logging.info("内容位置: 完全随机")
    elif content_position_fixed:
        logging.info(f"内容位置: 固定位置 ({content_position_fixed[0]}, {content_position_fixed[1]})")

    names_list_path = Path.cwd() / 'NamesList.txt'
    names_list_parser = NamesListParser(names_list_path)

    precomputed = PrecomputedValues(cfg)

    blocks_path = Path.cwd() / 'UnicodeBlocks.txt'
    if blocks_path.exists():
        blocks = load_unicode_blocks(blocks_path)
        logging.info(f"加载 {len(blocks)} 个 Unicode blocks")
    else:
        blocks = []
        logging.warning("UnicodeBlocks.txt 未找到，区块名称显示为 No_Block")

    color_mgr = None
    if args.random_color and not gradient_manager and not flash_color:
        logging.info("启用完全随机颜色模式")
        blend_cache = {}
        overlay_cache = {}
    elif args.dynamic_bg and not gradient_manager and not flash_color:
        logging.info("启用动态背景模式")
        color_mgr = ColorManager(cfg.color_cycle, Path('color_state.json'))
        if not color_mgr.state_file.exists() or not color_mgr._mapping:
            color_mgr.build_initial_mapping(entries)

        unique_colors = []
        if hasattr(color_mgr, '_mapping') and color_mgr._mapping:
            unique_colors.extend(color_mgr._mapping.values())
        if hasattr(color_mgr, 'color_cycle') and color_mgr.color_cycle:
            unique_colors.extend(color_mgr.color_cycle)
        unique_colors.append(cfg.background_color)

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
        if not gradient_manager and not flash_color:
            logging.info(f"固定背景: {cfg.background_color}")
        blend_cache, overlay_cache = precompute_blend_colors(cfg, [cfg.background_color])

    logging.info(
        f"需要生成 {len(to_process)} 张图片 (workers={args.workers}, png-quality={args.png_quality}, show-names-info={args.show_names_info})")

    bottom_font = ImageFont.truetype(str(cfg.bottom_font_file), cfg.bottom_font_size)
    try:
        ctrl_font = ImageFont.truetype(str(cfg.ctrl_font_file), cfg.middle_font_size)
    except OSError:
        logging.error(f"加载 Ctrl 字体失败: {cfg.ctrl_font_file}")
        ctrl_font = ImageFont.load_default()

    middle_font_cache, metrics_cache = preload_middle_fonts(to_process, cfg)

    text_cache: dict[str, tuple[int, int]] = {}
    overlay_bbox_cache: dict[str, tuple[int, int]] = {}

    unicode_names = load_unicode_names(unicode_data_path) if unicode_data_path.exists() else {}

    q: Queue = Queue(maxsize=200)
    writer = Thread(target=optimized_writer_thread_fn, args=(q,), daemon=True)
    writer.start()

    start = time.time()
    match_results = []

    sorted_entries = sorted(to_process, key=lambda x: int(x.code_str[2:], 16))

    with ThreadPoolExecutor(max_workers=args.workers) as pool, \
            tqdm(total=len(sorted_entries), desc="生成图片", unit="项") as bar:
        for i, entry in enumerate(sorted_entries):
            bar.set_description(f"生成图片: {entry.code_str}")

            if filename_mapping:
                original_order = [e.code_str for e in entries].index(entry.code_str)
                gradient_index = original_order
            else:
                gradient_index = i

            color_index = gradient_index * args.animation_speed

            future = pool.submit(
                generate_image_bytes,
                entry, cfg, color_mgr,
                bottom_font, ctrl_font,
                middle_font_cache, metrics_cache,
                text_cache,
                precomputed,
                blend_cache, overlay_cache,
                overlay_enabled, combining_cps,
                overlay_bbox_cache, blocks,
                unicode_names,
                names_list_parser,
                args.random_color and not gradient_manager and not flash_color,
                filename_mapping,
                gradient_manager,
                color_index,
                len(sorted_entries),
                flash_color,
                position_animator,
                animated_elements,
                content_position_random,
                content_position_fixed,
                args.show_names_info,
                args.smooth_gradient,
            )
            try:
                data, path, matched_key = future.result()
                q.put((data, path))
                match_results.append((entry.code_str, matched_key))
            except Exception as e:
                logging.error(f"生成失败: {e}")
            finally:
                bar.update(1)

    q.join()
    q.put(None)
    writer.join()

    elapsed = time.time() - start
    fps = len(to_process) / elapsed if elapsed > 0 else float('inf')
    logging.info(f"完成，用时 {elapsed:.2f}s，{fps:.2f} 张/秒。")

    if args.shuffle_content and filename_mapping:
        logging.info(f"文件名已随机化")

if __name__ == '__main__':
    main()