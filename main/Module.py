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
            try:
                if self.state_file.stat().st_size == 0:
                    logging.warning(f"颜色状态文件 {self.state_file} 为空，使用默认设置")
                    return

                with open(self.state_file, 'r', encoding='utf-8') as f:
                    state = json.load(f)
                    self._mapping = state.get("mapping", {})
                    self._counter = state.get("counter", 0)
                    logging.info(f"成功加载颜色状态，包含 {len(self._mapping)} 个映射")
            except json.JSONDecodeError as e:
                logging.error(f"颜色状态文件格式错误：{e}，使用默认设置")
                backup_file = self.state_file.with_suffix('.json.backup')
                self.state_file.rename(backup_file)
                logging.info(f"已将损坏的文件备份为 {backup_file}")
            except Exception as e:
                logging.error(f"加载颜色状态文件时出错：{e}，使用默认设置")

    def save_state(self):
        try:
            temp_file = self.state_file.with_suffix('.json.tmp')
            with open(temp_file, 'w', encoding='utf-8') as f:
                json.dump({"mapping": self._mapping, "counter": self._counter}, f, indent=2, ensure_ascii=False)

            temp_file.replace(self.state_file)

        except Exception as e:
            logging.error(f"保存颜色状态文件时出错：{e}")
            temp_file = self.state_file.with_suffix('.json.tmp')
            if temp_file.exists():
                temp_file.unlink()

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

    def get_key_from_description(self, description: str) -> str:
        """从描述中提取关键部分,忽略详细信息"""
        if '|' in description:
            return description.split('|')[0].strip()
        else:
            return description

    def get_color(self, description: str) -> tuple[int, int, int, int]:
        key = self.get_key_from_description(description)
        with self._lock:
            if key not in self._mapping:
                self._mapping[key] = self._counter
                self._counter = (self._counter + 1) % len(self._cycle)
                self.save_state()

            value = self._mapping[key]

            # 如果是字符串（十六进制颜色），直接转换
            if isinstance(value, str):
                return self._hex_to_rgba(value)
            # 如果是数字，使用颜色循环
            else:
                return self._cycle[value % len(self._cycle)]

    def set_custom_color(self, description: str, color: str):
        """为特定描述设置自定义颜色"""
        key = self.get_key_from_description(description)
        with self._lock:
            self._mapping[key] = color
            self.save_state()

    def build_initial_mapping(self, entries: list):
        """根据 Unicode 条目构建初始映射"""
        sorted_entries = sorted(entries, key=lambda x: int(x.code_str[2:], 16))

        seen = set()
        descriptions = []
        for entry in sorted_entries:
            key = self.get_key_from_description(entry.description)
            if key not in seen:
                descriptions.append(key)
                seen.add(key)

            with self._lock:
                for desc in descriptions:
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
