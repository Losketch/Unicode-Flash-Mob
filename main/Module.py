#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import json
import struct
import logging
from queue import Queue
from pathlib import Path
from threading import Lock
from dataclasses import dataclass
from functools import lru_cache
from fontTools.ttLib import TTFont
from blackrenderer.font import BlackRendererFont
from blackrenderer.backends import getSurfaceClass
from PIL import Image
import uharfbuzz as hb_module
import math
import array

HAS_SKIA = getSurfaceClass("skia") is not None


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

    png_compress_level: int = 2
    png_optimize: bool = False

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
            self.color_cycle = [self._hex_to_rgba_fast(h) for h in hex_cycle]

    @staticmethod
    @lru_cache(maxsize=128)
    def _hex_to_rgba_fast(s: str) -> tuple[int,int,int,int]:
        """优化的hex转RGBA，使用缓存避免重复计算"""
        s = s.lstrip('#')
        if len(s) == 6:
            s = s + 'FF'
        if len(s) != 8:
            raise ValueError(f"Invalid hex color: {s!r}")
        return (
            int(s[0:2], 16),
            int(s[2:4], 16), 
            int(s[4:6], 16),
            int(s[6:8], 16)
        )

    def _hex_to_rgba(self, s: str) -> tuple[int,int,int,int]:
        return self._hex_to_rgba_fast(s)


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

        self._color_cache: dict[str, tuple[int, int, int, int]] = {}
        self._key_cache: dict[str, str] = {}

        self._dirty = False
        self._save_batch_size = 100
        self._changes_count = 0

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

    def save_state(self, force=False):
        if not force and not self._dirty:
            return
            
        try:
            temp_file = self.state_file.with_suffix('.json.tmp')
            with open(temp_file, 'w', encoding='utf-8') as f:
                json.dump({
                    "mapping": self._mapping, 
                    "counter": self._counter
                }, f, indent=2, ensure_ascii=False)

            if os.name == 'nt':  # Windows
                if self.state_file.exists():
                    self.state_file.unlink()
            temp_file.replace(self.state_file)
            self._dirty = False

        except Exception as e:
            logging.error(f"保存颜色状态文件时出错：{e}")
            temp_file = self.state_file.with_suffix('.json.tmp')
            if temp_file.exists():
                temp_file.unlink()

    @lru_cache(maxsize=512)
    def _hex_to_rgba_cached(self, hex_color: str) -> tuple[int, int, int, int]:
        """将十六进制颜色转换为 RGBA 元组"""
        hex_color = hex_color.lstrip('#')
        if len(hex_color) == 3:  # #RGB
            hex_color = ''.join([c*2 for c in hex_color]) + 'FF'
        elif len(hex_color) == 6:  # #RRGGBB
            hex_color = hex_color + 'FF'
        elif len(hex_color) != 8:  # #RRGGBBAA
            raise ValueError(f"Invalid hex color format: {hex_color}")

        return (
            int(hex_color[0:2], 16),
            int(hex_color[2:4], 16),
            int(hex_color[4:6], 16),
            int(hex_color[6:8], 16)
        )

    def get_key_from_description(self, description: str) -> str:
        """从描述中提取关键部分,忽略详细信息"""
        if description in self._key_cache:
            return self._key_cache[description]
            
        if '|' in description:
            key = description.split('|')[0].strip()
        else:
            key = description

        if len(self._key_cache) < 10000:
            self._key_cache[description] = key
        return key

    def get_color(self, description: str) -> tuple[int, int, int, int]:
        key = self.get_key_from_description(description)

        if key in self._mapping:
            value = self._mapping[key]

            cache_key = f"{key}_{value}"
            if cache_key in self._color_cache:
                return self._color_cache[cache_key]

            if isinstance(value, str):
                color = self._hex_to_rgba_cached(value)
            else:
                color = self._cycle[value % len(self._cycle)]

            if len(self._color_cache) < 5000:
                self._color_cache[cache_key] = color
            return color

        with self._lock:
            if key not in self._mapping:
                self._mapping[key] = self._counter
                self._counter = (self._counter + 1) % len(self._cycle)
                self._dirty = True
                self._changes_count += 1

                if self._changes_count >= self._save_batch_size:
                    self.save_state()
                    self._changes_count = 0

            value = self._mapping[key]

            if isinstance(value, str):
                color = self._hex_to_rgba_cached(value)
            else:
                color = self._cycle[value % len(self._cycle)]
            
            cache_key = f"{key}_{value}"
            if len(self._color_cache) < 5000:
                self._color_cache[cache_key] = color
            return color

    def set_custom_color(self, description: str, color: str):
        """为特定描述设置自定义颜色"""
        key = self.get_key_from_description(description)
        with self._lock:
            self._mapping[key] = color
            self._dirty = True
            self._changes_count += 1

            cache_keys_to_remove = [k for k in self._color_cache.keys() if k.startswith(f"{key}_")]
            for k in cache_keys_to_remove:
                del self._color_cache[k]
            
            if self._changes_count >= self._save_batch_size:
                self.save_state()
                self._changes_count = 0

    def build_initial_mapping(self, entries: list):
        """根据 Unicode 条目构建初始映射"""
        seen = set()
        unique_descriptions = []

        sorted_entries = sorted(entries, key=lambda x: int(x.code_str[2:], 16))
        
        for entry in sorted_entries:
            key = self.get_key_from_description(entry.description)
            if key not in seen:
                unique_descriptions.append(key)
                seen.add(key)

        with self._lock:
            for desc in unique_descriptions:
                if desc not in self._mapping:
                    self._mapping[desc] = self._counter
                    self._counter = (self._counter + 1) % len(self._cycle)
            
            self._dirty = True
            self.save_state(force=True)

        logging.info(f"构建了 {len(unique_descriptions)} 个描述的颜色映射")

    def finalize(self):
        """完成处理时调用，确保所有更改都已保存"""
        if self._dirty:
            self.save_state(force=True)


def load_unicode_entries(path: Path) -> list[UnicodeEntry]:
    """
    读取新的 Unicode.txt 格式，每行格式：
      "font_path";"U+xxxx";"Description"
    去除空行，拆分三段，去除两端引号后返回 UnicodeEntry 列表。
    """
    entries: list[UnicodeEntry] = []

    content = path.read_text(encoding='utf-8')
    lines = content.splitlines()
    
    for line in lines:
        line = line.strip()
        if not line:
            continue

        parts = line.split(';', 2)
        if len(parts) == 2:
            parts.append('')
        elif len(parts) != 3:
            raise ValueError(f"行格式错误（期望 2 或 3 段，用 ; 分隔）: {line!r}")

        font_path = parts[0].strip().strip('"')
        code_str = parts[1].strip().strip('"')
        desc = parts[2].strip().strip('"')
        
        entries.append(UnicodeEntry(Path(font_path), code_str, desc))
    
    return entries


def setup_logging():
    if not logging.getLogger().handlers:
        logging.basicConfig(
            format='[%(asctime)s] - %(levelname)s - %(message)s',
            datefmt='%H:%M:%S',
            level=logging.INFO
        )


def writer_thread_fn(q: Queue, batch_size: int = 10):
    """单线程顺序写入磁盘，减少 HDD 随机寻道。"""
    batch = []
    
    while True:
        item = q.get()
        if item is None:
            if batch:
                _write_batch(batch)
            q.task_done()
            break
            
        batch.append(item)
        if len(batch) >= batch_size:
            _write_batch(batch)
            batch.clear()
            
        q.task_done()


def _write_batch(batch: list):
    """批量写入文件"""
    for data, out_path in batch:
        try:
            out_path.parent.mkdir(parents=True, exist_ok=True)
            with open(out_path, 'wb') as f:
                f.write(data)
        except Exception as e:
            logging.error(f"写入文件失败 {out_path}: {e}")


@lru_cache(maxsize=1024)
def blend_colors(fg: tuple[int,int,int], bg: tuple[int,int,int], alpha: float) -> tuple[int,int,int]:
    """缓存版颜色混合函数"""
    inv_alpha = 1.0 - alpha
    return (
        int(fg[0] * alpha + bg[0] * inv_alpha),
        int(fg[1] * alpha + bg[1] * inv_alpha),
        int(fg[2] * alpha + bg[2] * inv_alpha)
    )


def get_bitmap_font_sizes(font_path: Path, target_size: int) -> tuple[int, int]:
    """读取位图字体(CBDT/sbix)的可用尺寸，返回 (PPEM, 实际位图像素尺寸)"""
    tt = TTFont(str(font_path))

    if 'CBDT' in tt:
        ppem = None
        actual_size = None

        if 'CBLC' in tt:
            cblc = tt['CBLC']
            if hasattr(cblc, 'strikes') and cblc.strikes:
                strikes = cblc.strikes
                for strike in strikes:
                    if hasattr(strike, 'bitmapSizeTable'):
                        bt = strike.bitmapSizeTable
                        if hasattr(bt, 'ppemX') and bt.ppemX:
                            ppem = int(bt.ppemX)
                        break

        cbdt = tt['CBDT']
        if hasattr(cbdt, 'strikeData') and cbdt.strikeData:
            strike_data = cbdt.strikeData[0]
            for glyph_name, glyph_data in strike_data.items():
                if hasattr(glyph_data, 'metrics'):
                    metrics = glyph_data.metrics
                    if hasattr(metrics, 'height'):
                        actual_size = int(metrics.height)
                    break

        if ppem and actual_size:
            tt.close()
            return (ppem, actual_size)
        if ppem:
            tt.close()
            return (ppem, ppem)
        if actual_size:
            tt.close()
            return (actual_size, actual_size)

    if 'sbix' in tt:
        sbix = tt['sbix']
        if hasattr(sbix, 'strikes') and sbix.strikes:
            ppem = min(sbix.strikes.keys())
            actual_size = None

            for strike in sbix.strikes.values():
                if hasattr(strike, 'glyphs'):
                    for glyph_name, glyph_data in strike.glyphs.items():
                        if hasattr(glyph_data, 'imageData') and glyph_data.imageData:
                            try:
                                data = glyph_data.imageData
                                if len(data) >= 24:
                                    actual_size = struct.unpack('>I', data[16:20])[0]
                            except Exception:
                                pass
                            break
                    break

            if actual_size:
                tt.close()
                return (ppem, actual_size)
            tt.close()
            return (ppem, ppem)

    if 'head' in tt:
        upem = tt['head'].unitsPerEm
        tt.close()
        return (upem, upem)
        
    tt.close()

    return (target_size, target_size)


def has_colr_table(font_path: Path) -> bool:
    """检查字体是否有 COLR 表（支持 COLRv0 和 COLRv1）"""
    tt = TTFont(str(font_path))
    has_colr = 'COLR' in tt
    tt.close()
    return has_colr


def has_svg_table(font_path: Path) -> bool:
    """检查字体是否有 SVG 表"""
    tt = TTFont(str(font_path))
    has_svg = 'SVG ' in tt
    tt.close()
    return has_svg


def render_svg_glyph(
    font_path: Path,
    char: str,
    font_size: int,
    fg_color: tuple[int, int, int]
) -> tuple[Image.Image, int] | None:
    """使用 skia-python 渲染 SVG 彩色字形，返回 (PIL Image, baseline_offset)
    baseline_offset: 从图像底部到字形基线的像素数
    """
    try:
        import skia
    except ImportError:
        return None
    
    ttFont = TTFont(str(font_path))
    
    if 'SVG ' not in ttFont:
        ttFont.close()
        return None
    
    svg_table = ttFont['SVG ']
    if not svg_table.docList:
        ttFont.close()
        return None
    
    char_code = ord(char)
    
    cmap = ttFont.getBestCmap()
    if cmap is None:
        ttFont.close()
        return None
    
    glyph_name = cmap.get(char_code)
    if glyph_name is None:
        ttFont.close()
        return None
    
    glyph_order = ttFont.getGlyphOrder()
    try:
        glyph_id = glyph_order.index(glyph_name)
    except ValueError:
        ttFont.close()
        return None
    
    svg_doc = None
    for doc in svg_table.docList:
        if doc.startGlyphID <= glyph_id <= doc.endGlyphID:
            svg_doc = doc
            break
    
    if svg_doc is None:
        ttFont.close()
        return None
    
    svg_doc_str = svg_doc.data
    
    unitsPerEm = ttFont['head'].unitsPerEm
    scale = font_size / unitsPerEm
    
    canvas_width = font_size + 4
    canvas_height = font_size + 4
    
    ascent = ttFont['hhea'].ascent
    baseline_offset = int(ascent * scale) + 2
    
    svg_with_xml = '<?xml version="1.0" encoding="UTF-8"?>' + svg_doc_str
    svg_bytes = svg_with_xml.encode('utf-8')
    
    stream = skia.MemoryStream.Make(svg_bytes)
    dom = skia.SVGDOM.MakeFromStream(stream)
    if dom is None:
        ttFont.close()
        return None
    
    surface = skia.Surface.MakeRaster(skia.ImageInfo.MakeN32Premul(canvas_width, canvas_height))
    canvas = surface.getCanvas()
    
    canvas.clear(0)
    
    transform = skia.Matrix()
    transform.setScale(scale, scale)
    transform.postTranslate(0, canvas_height)
    canvas.setMatrix(transform)
    
    dom.setContainerSize(skia.Size.Make(canvas_width / scale, canvas_height / scale))
    dom.render(canvas)
    
    image = surface.makeImageSnapshot()
    if image is None:
        ttFont.close()
        return None
    
    png_data = image.encodeToData(skia.EncodedImageFormat.kPNG, 100)
    
    from io import BytesIO
    img = Image.open(BytesIO(png_data))
    img = img.convert("RGBA")
    
    ttFont.close()
    
    return (img, baseline_offset)


def render_colr_glyph(
    font_path: Path,
    char: str,
    font_size: int,
    fg_color: tuple[int, int, int]
) -> tuple[Image.Image, int] | None:
    """使用 BlackRenderer 渲染 COLR 彩色字形，返回 (PIL Image, baseline_offset)
    baseline_offset: 从图像底部到字形基线的像素数
    """
    if HAS_SKIA and has_colr_table(font_path):
        surfaceClass = getSurfaceClass("skia")
        if surfaceClass is None:
            return None
        
        brFont = BlackRendererFont(str(font_path))
        
        if not brFont.colrV0Glyphs and not brFont.colrV1Glyphs:
            return None
        
        glyph_names = brFont.glyphNames
        
        import uharfbuzz as hb_module
        
        buf = hb_module.Buffer()
        buf.add_str(char)
        buf.guess_segment_properties()
        hb_module.shape(brFont.hbFont, buf)
        
        if not buf.glyph_infos:
            return None
        
        glyph_info = buf.glyph_infos[0]
        glyph_name = glyph_names[glyph_info.codepoint]
        
        bounds = brFont.getGlyphBounds(glyph_name)
        if bounds is None:
            return None
        
        x_min, y_min, x_max, y_max = bounds
        glyph_width = x_max - x_min
        glyph_height = y_max - y_min
        
        if glyph_width == 0 or glyph_height == 0:
            return None
        
        scale = font_size / brFont.unitsPerEm
        
        canvas_width = int(glyph_width * scale) + 4
        canvas_height = int(glyph_height * scale) + 4
        baseline_offset = int(y_max * scale) + 2
        
        bounds_for_canvas = (0, -canvas_height, canvas_width, 0)
        
        surface = surfaceClass()
        
        try:
            with surface.canvas(bounds_for_canvas) as canvas:
                canvas.scale(scale)
                canvas.translate(-x_min, -y_max)
                palette = brFont.getPalette(0)
                brFont.drawGlyph(glyph_name, canvas, palette=palette)
        except Exception as e:
            return None
        
        import tempfile
        import os
        with tempfile.NamedTemporaryFile(suffix='.png', delete=False) as f:
            temp_path = f.name
        
        surface.saveImage(temp_path)
        
        from io import BytesIO
        with open(temp_path, 'rb') as f:
            png_data = f.read()
        os.unlink(temp_path)
        
        img = Image.open(BytesIO(png_data))
        img = img.convert("RGBA")
        
        return (img, baseline_offset)
    
    if has_svg_table(font_path):
        return render_svg_glyph(font_path, char, font_size, fg_color)
    
    return None
