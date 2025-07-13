#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import json
import logging
from queue import Queue
from pathlib import Path
from threading import Lock
from dataclasses import dataclass

@dataclass
class Config:
    unicode_file: Path = Path.cwd() / 'Unicode.txt'
    output_dir: Path = Path.cwd() / 'png'
    font_files: list[Path] = None
    ctrl_font_file: Path = Path('Ctrl-Ctrl.ttf')
    bottom_font_file: Path = Path.cwd() / 'PressStart2P-1.ttf'
    music_file: Path = Path.cwd() / 'DUTM.m4a'
    middle_font_size: int = 512
    bottom_font_size: int = 19
    text_position: tuple[int, int] = (0, 0) #(text_position_x, text_position_y)
    middle_font_color: tuple[int, int, int, int] = (255, 255, 255, 255)
    image_size: tuple[int, int] = (1920, 1080)
    background_color: tuple[int, int, int, int] = (0,0,0,255)
    color_cycle: list[tuple[int, int, int, int]] = None

    def __post_init__(self):
        if self.font_files is None:
            self.font_files = [Path.cwd() / 'font.ttf']

        if self.color_cycle is None:
            hex_cycle = [
                "#ABDF56FF", "#6DE74EFF", "#68F59FFF",
                "#00BE9DFF", "#00CB81FF", "#A8FD9AFF",
                "#99FEA9FF", "#98FCCAFF", "#98FEEBFF",
                "#97ECFDFF", "#33E2FDFF", "#34B5DFFF",
                "#0095E0FF", "#CD9BFFFF", "#AB9BFFFF",
                "#EE9AFFEF", "#FF9AF0FF", "#FE9ACCFF",
                "#FF9AAAFF", "#FCAB9AFF", "#FBC99AFF",
                "#FDEE99FF", "#EEFE99FF", "#CFFF9BFF",
            ]
            self.color_cycle = [self._hex_to_rgba(h) for h in hex_cycle]

    def _hex_to_rgba(self, s: str) -> tuple[int,int,int,int]:
        s = s.lstrip('#')
        if len(s) == 6:
            s = s + 'FF'
        if len(s) != 8:
            raise ValueError(f"Invalid hex color: {s!r}")
        r = int(s[0:2], 16)
        g = int(s[2:4], 16)
        b = int(s[4:6], 16)
        a = int(s[6:8], 16)
        return (r, g, b, a)


@dataclass
class UnicodeEntry:
    font_path: Path
    code_str: str
    description: str


class ColorManager:
    """
    根据 description 的哈希值循环分配背景色，保持相同 description 使用相同颜色。
    支持预定义映射和直接颜色值。
    """
    def __init__(self, color_cycle: list[tuple[int, int, int, int]], state_file: Path):
        self._cycle = color_cycle
        self._mapping: dict[str, int | str] = {}
        self._counter = 0
        self._lock = Lock()
        self.state_file = state_file

        self.load_state()

    def load_state(self):
        if self.state_file.exists():
            with open(self.state_file, 'r') as f:
                state = json.load(f)
                self._mapping = state.get("mapping", {})
                self._counter = state.get("counter", 0)

    def save_state(self):
        with open(self.state_file, 'w') as f:
            json.dump({"mapping": self._mapping, "counter": self._counter}, f, indent=2)

    def _hex_to_rgba(self, hex_color: str) -> tuple[int, int, int, int]:
        """将十六进制颜色转换为 RGBA 元组"""
        hex_color = hex_color.lstrip('#')
        if len(hex_color) == 3:  # #RGB
            hex_color = ''.join([c*2 for c in hex_color]) + 'FF'
        elif len(hex_color) == 6:  # #RRGGBB
            hex_color = hex_color + 'FF'
        elif len(hex_color) != 8:  # #RRGGBBAA
            raise ValueError(f"Invalid hex color format: {hex_color}")
        
        r = int(hex_color[0:2], 16)
        g = int(hex_color[2:4], 16)
        b = int(hex_color[4:6], 16)
        a = int(hex_color[6:8], 16)
        return (r, g, b, a)

    def get_color(self, description: str) -> tuple[int, int, int, int]:
        with self._lock:
            if description not in self._mapping:
                self._mapping[description] = self._counter
                self._counter = (self._counter + 1) % len(self._cycle)
                self.save_state()
            
            value = self._mapping[description]
            
            # 如果是字符串（十六进制颜色），直接转换
            if isinstance(value, str):
                return self._hex_to_rgba(value)
            # 如果是数字，使用颜色循环
            else:
                return self._cycle[value % len(self._cycle)]

    def set_custom_color(self, description: str, color: str):
        """为特定描述设置自定义颜色"""
        with self._lock:
            self._mapping[description] = color
            self.save_state()

    def build_initial_mapping(self, entries: list):
        """根据 Unicode 条目构建初始映射"""
        descriptions = set()
        for entry in entries:
            if entry.description:
                descriptions.add(entry.description)
        
        with self._lock:
            for desc in sorted(descriptions):
                if desc not in self._mapping:
                    self._mapping[desc] = self._counter
                    self._counter = (self._counter + 1) % len(self._cycle)
            self.save_state()
        
        logging.info(f"构建了 {len(descriptions)} 个描述的颜色映射")


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
