from pathlib import Path
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
