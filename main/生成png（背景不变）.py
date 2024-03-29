import os
from PIL import Image, ImageDraw, ImageFont
from tqdm import tqdm
from concurrent.futures import ThreadPoolExecutor
import time
import configparser

# 创建文件目录下新建一个png的目录
os.makedirs("png", exist_ok=True)

# 获取Unicode.txt的路径
unicode_txt = os.path.join(os.getcwd(), "Unicode.txt")

# 读取Unicode.txt文件内容
with open(unicode_txt, "r", encoding="utf-8") as f:
    unicode_lines = f.readlines()

# 读取settings.ini文件
config = configparser.ConfigParser()
config.read('settings.ini')

# 设置字体路径和字体大小   bak:(bottom_font_size = 32)
font_files = [
    os.path.join(os.getcwd(), "font.ttf")
]
# 获取middle_font_size的值，默认为512
middle_font_size = int(config.get('Settings', 'middle_font_size', fallback=512))
bottom_font_path = os.path.join(os.getcwd(), "PressStart2P-1.ttf")
bottom_font_size = int(config.get('Settings', 'bottom_font_size', fallback=19))

# 获取text_height的值，默认为1000
text_height = int(config.get('Settings', 'text_height', fallback=1000))

# 获取 text_position_x 和 text_position_y 的值，默认为 0
text_position_x = int(config.get('Settings', 'text_position_x', fallback=0))
text_position_y = int(config.get('Settings', 'text_position_y', fallback=0))

# 获取中央字符颜色，默认为白色
middle_font_color = tuple(map(int, config.get('Settings', 'middle_font_color', fallback='255,255,255,255').split(',')))

# 获取背景颜色，默认为黑色
background_color = tuple(map(int, config.get('Settings', 'background_color', fallback='0,0,0,255').split(',')))

# 设置图片的尺寸
image_size = (1920, 1080)

# 获取Unicode.txt文件的行数
def get_unicode_lines():
    with open("Unicode.txt", "r", encoding="utf-8") as f:
        lines = f.readlines()
    return [line.strip() for line in lines]

# 获取Unicode.txt的行数
unicode_lines = get_unicode_lines()

# 创建进度条
total_lines = len(unicode_lines)
progress_bar = tqdm(total=total_lines, desc="生成图片进度", unit="行", ncols=80)

# 定义生成图片的函数
def generate_image(line_index):
    try:
        # 转换Unicode编码为字符
        line = unicode_lines[line_index]
        unicode_char = chr(int(line.split("-")[0][2:], 16))

        # 创建空白图像，使用用户定义的背景颜色
        image = Image.new("RGBA", image_size, color=background_color)

        # 在图像中央绘制Unicode字符，使用用户定义的颜色
        draw = ImageDraw.Draw(image)
        middle_font = None
        for font_file in font_files:
            try:
                middle_font = ImageFont.truetype(font_file, middle_font_size)
                break
            except OSError:
                continue

        if middle_font is None:
            # 如果所有字体均无法显示字符，则使用默认的替代字符
            unicode_char = "Please check if there is a font file named “font.ttf” in the directory, paying attention to the case sensitivity.\nAlternatively, if you are unable to display a specific Unicode character, it could be due to the font you are using.\nYou may need to use a different font that supports the Unicode character you're trying to display."

        # 获取文本的位置（x, y）
        text_x, text_y, text_width, text_height = draw.textbbox((text_position_x, text_position_y), unicode_char, font=middle_font)
        # 在图像中央绘制 Unicode 字符，使用用户定义的颜色
        text_position = ((image_size[0] - text_width) // 2, (image_size[1] - text_height) // 4)
        draw.text(text_position, unicode_char, fill=middle_font_color, font=middle_font)

        # 在图像左下角绘制当前生成的Unicode编码相同的Unicode.txt的整行内容
        bottom_text = line.strip()
        if "|" in bottom_text:
            index = bottom_text.index("|")
            modified_text = bottom_text[:index].replace("-", "\n") + bottom_text[index:].replace("|", "\n")
            bottom_text = modified_text
        bottom_font = ImageFont.truetype(bottom_font_path, bottom_font_size)
        bottom_text_position = (100, image_size[1] - bottom_font_size - 125)
        draw.text(bottom_text_position, bottom_text, fill="white", font=bottom_font)

        # 保存生成的图片
        image.save(os.path.join("png", f"image_{line.split('-')[0]}.png"))

        # 更新进度条
        progress_bar.update(1)
    except Exception as e:
        print(f"生成图片时出现错误：{e}")

# 使用多线程生成图片
with ThreadPoolExecutor() as executor:
    start_time = time.time()
    executor.map(generate_image, range(total_lines))

# 关闭进度条
progress_bar.close()

# 计算平均帧数
total_time = time.time() - start_time
average_fps = total_lines / total_time

print("图片生成完成！")
print(f"平均每秒生成帧数：{average_fps:.2f}")
