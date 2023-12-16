import os
from fontTools.ttLib import TTFont

def extract_unicode_from_font(font_file):
    font_path = font_file
    font = TTFont(font_path)
    cmap = font.getBestCmap()

    unicode_list = []
    for char_code, glyph_name in cmap.items():
        unicode_list.append(f"U+{char_code:04X}")

    file_name = os.path.basename(font_file)
    out_file = file_name.split('.')[0] + '_字体提取的显示列表Unicode.txt'
    with open(out_file, 'w', encoding='utf-8') as file:
        file.write('\n'.join(unicode_list))

if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        font_file = sys.argv[1]
        extract_unicode_from_font(font_file)
    else:
        print("\n请将字体文件拖放到本文件上来执行字体提取功能，你可能没有指定字体文件才显示此通知")
        input("\n请按任意键退出...")
