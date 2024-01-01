import os
from PIL import Image, ImageDraw, ImageFont
from tqdm import tqdm
from concurrent.futures import ThreadPoolExecutor
import time
import hashlib
import configparser

# 添加颜色索引字典
color_index_dict = {}

# 添加哈希函数
def calculate_color_index(description, color_cycle_length):
    hash_value = int(hashlib.sha256(description.encode()).hexdigest(), 16)
    return hash_value % color_cycle_length

# 添加全局变量
global_color_index = 0

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

# 设置字体路径和字体大小
font_files = [
    os.path.join(os.getcwd(), "font.ttf")
]
# 获取middle_font_size的值，默认为512
middle_font_size = int(config.get('Settings', 'middle_font_size', fallback=512))
bottom_font_path = os.path.join(os.getcwd(), "PressStart2P-1.ttf")
bottom_font_size = 38

# 获取text_height的值，默认为1000
text_height = int(config.get('Settings', 'text_height', fallback=1000))

# 获取 text_position_x 和 text_position_y 的值，默认为 0
text_position_x = int(config.get('Settings', 'text_position_x', fallback=0))
text_position_y = int(config.get('Settings', 'text_position_y', fallback=0))

# 获取中央字符颜色，默认为白色
middle_font_color = tuple(map(int, config.get('Settings', 'middle_font_color', fallback='255,255,255,255').split(',')))

# 设置图片的尺寸
image_size = (1920, 1080)

# 定义颜色循环顺序
color_cycle = [
    (171, 223, 86, 255),
    (109, 231, 78, 255),
    (0, 203, 129, 255),
    (0, 190, 157, 255),
    (0, 149, 224, 255),
    (52, 181, 223, 255),
    (51, 226, 253, 255),
    (104, 245, 159, 255)
]

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
def generate_image(line):
    try:
        # 转换Unicode编码为字符
        parts = line.split("-")
        if len(parts) >= 2 and parts[0]:
            unicode_char, description = chr(int(parts[0][2:], 16)), parts[1]
        else:
            raise ValueError("Invalid line format")

        # 如果所有字体均无法显示字符，则使用默认的替代字符
        default_text = "请检查目录中是否有名为“font.ttf”的字体文件(区分大小写)\n如果无法显示特定的字符, 可能是使用的字体不正常\n也有可能你使用的不是常规字体, 列如emoji字体"

        # 创建空白图像
        image = Image.new("RGBA", image_size, color=(0, 0, 0, 0))

        # 设置背景颜色
        # 修改颜色计算部分
        global global_color_index, color_index_dict

        # 计算哈希值
        hash_value = calculate_color_index(description, len(color_cycle))
        #print(f"Description: {description}, Hash Value: {hash_value}")
    
        # 使用字典获取颜色索引，如果不存在则添加
        if hash_value not in color_index_dict:
            color_index_dict[hash_value] = global_color_index % len(color_cycle)
            global_color_index += 1

        # 获取颜色索引
        color_index = color_index_dict[hash_value]

        # 设置背景颜色
        background_color = color_cycle[color_index]
        image = image.convert("RGB")
        image.putalpha(255)
        image.paste(Image.new("RGBA", image_size, background_color), (0, 0), image)

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
            middle_font = ImageFont.truetype(bottom_font_path, 40)  # 指定字体为 "PressStart2P-1.ttf"，大小为 40

        # 获取文本的位置（x, y）
        text_x, text_y, text_width, text_height = draw.textbbox((text_position_x, text_position_y), default_text, font=middle_font)
        # 在图像中央绘制 Unicode 字符，使用用户定义的颜色
        text_position = ((image_size[0] - text_width) // 2, (image_size[1] - text_height) // 4)
        draw.text(text_position, default_text, fill=middle_font_color, font=middle_font)

        # 在图像左下角绘制当前生成的Unicode编码相同的Unicode.txt的整行内容
        bottom_text = line.strip()
        if "|" in bottom_text:
            index = bottom_text.index("|")
            modified_text = bottom_text[:index].replace("-", "\n") + bottom_text[index:].replace("|", "\n")
            bottom_text = modified_text
        bottom_font = ImageFont.truetype(bottom_font_path, bottom_font_size)
        bottom_text_position = (50, image_size[1] - bottom_font_size - 150)
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
    executor.map(generate_image, unicode_lines)

# 关闭进度条
progress_bar.close()

# 计算平均帧数
total_time = time.time() - start_time
average_fps = total_lines / total_time

print("图片生成完成！")
print(f"平均每秒生成帧数：{average_fps:.2f}")
