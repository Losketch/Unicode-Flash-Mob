import os
import re

# 定义颜色顺序
color_order = {"绿": 0, "青": 1, "蓝": 2, "紫": 3, "红": 4, "橙": 5, "黄": 6}

# 获取目录下所有文件名
file_list = os.listdir('background')

# 定义提取RGB值的正则表达式
rgb_pattern = re.compile(r'RGB\((\d+),\s*(\d+),\s*(\d+),\s*255\).png')

# 创建存储结果的字典
rgb_values = {}

# 遍历文件，提取以RGB开头的文件名的RGB值
for file_name in file_list:
    match = rgb_pattern.search(file_name)
    if match:
        r, g, b = map(int, match.groups())
        rgb_sum = r + g + b
        rgb_values[file_name] = (r, g, b, rgb_sum)

# 按照颜色顺序进行排序
sorted_files = sorted(rgb_values.items(), key=lambda x: color_order.get(x[1][3], x[1][3]))

# 打印排序结果
for file, rgb in sorted_files:
    print(file)
