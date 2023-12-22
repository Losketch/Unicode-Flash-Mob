import os
import subprocess
import re
import sys

def convert_images_to_video(image_folder, output_file, frame_rate, file_list):
    # 获取环境变量中的 ffmpeg 路径
    ffmpeg_path = os.environ.get("PATH")

    # 从环境变量中查找 ffmpeg.exe
    for path in ffmpeg_path.split(os.pathsep):
        if os.path.exists(os.path.join(path, "ffmpeg.exe")):
            ffmpeg_path = os.path.join(path, "ffmpeg.exe")
            print(f"\n从环境变量中找到 ffmpeg.exe 文件\n在：{ffmpeg_path}\n")
            break

    # 如果环境变量中没有找到 ffmpeg.exe，则在当前目录中查找
    if not os.path.exists(ffmpeg_path):
        for root, dirs, files in os.walk("."):
            for file in files:
                if file.endswith("ffmpeg.exe"):
                    ffmpeg_path = os.path.join(root, file)
                    print(f"\n未发现环境变量有，但找到 ffmpeg.exe 文件\n在：{ffmpeg_path}\n")
                    break

    # 如果仍然没有找到 ffmpeg.exe，则打印提示信息
    if not os.path.exists(ffmpeg_path):
        input("\n从环境变量中未找到 ffmpeg.exe 文件\n在当前目录及所有文件夹中未找到 ffmpeg.exe 文件\n你似乎没有 ffmpeg")
        sys.exit(1)

    # 生成包含所有图像文件的临时文件
    temp_file = 'temp.txt'
    with open(temp_file, 'w', encoding='utf-8') as file:  # 指定编码为UTF-8
        for image_file in file_list:
            file.write(f"file '{os.path.join(image_folder, image_file)}'\n")

    # 使用FFmpeg concat协议将图片合成视频
    ffmpeg_command = [
        ffmpeg_path,
        '-y',
        '-r', str(frame_rate),
        '-f', 'concat',
        '-safe', '0',
        '-i', temp_file,
        '-c:v', 'libx264',
        '-crf', '18',
        '-pix_fmt', 'yuv420p',
        '-fflags', '+genpts',
        '-loglevel', '50',  # 设置记录级别为info
        output_file
    ]

    # 使用subprocess模块执行ffmpeg命令并实时输出日志
    process = subprocess.Popen(ffmpeg_command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, universal_newlines=True)
    for line in process.stdout:
        # 使用正则表达式匹配[libx264 @ xxxxxxxxxxxxxxxx] frame=并刷新显示
        match = re.search(r'\[libx264 @ [^\]]+\] frame=', line)
        if match:
            print(line.strip())  # 输出匹配的内容
            sys.stdout.flush()  # 刷新显示

    process.wait()  # 等待ffmpeg进程完成

    # 删除临时文件
    os.remove(temp_file)

# 在获取输入文件夹的时候使用os.fsdecode
input_folder = os.fsdecode('png')  # 输入文件夹路径

# 获取用户输入的输出视频名称
def get_output_video_name():
    while True:
        output_file_name = input("请输入输出视频的名称：")  # 自定义输出视频名称
        output_file_name = output_file_name.lstrip('\ufeff')  # 去掉文件名中的BOM
        output_file = os.path.join('output', output_file_name + '.mp4')  # 输出视频文件路径（自动添加路径和扩展名）

        if os.path.exists(output_file):
            replace = input("已存在同名文件，是否替换？(Y/N): ")
            if replace.lower() == 'y':
                return output_file_name
        else:
            return output_file_name

# 获取用户输入的视频帧率
def get_frame_rate():
    while True:
        frame_rate_input = input("请输入视频的帧率（帧/秒）：")
        try:
            frame_rate = float(frame_rate_input)
            if frame_rate <= 0:
                print("帧率必须大于0，请重新输入.")
            else:
                return frame_rate
        except ValueError:
            print("无效的输入，请输入一个数字.")

output_file_name = get_output_video_name()
frame_rate = get_frame_rate()

# 获取图像文件列表并按照用户要求的顺序排序
image_files = [f for f in os.listdir(input_folder) if f.endswith('.png')]

for file_name in image_files:
    try:
        # 使用正则表达式提取 Unicode 编码
        match = re.search(r'_U\+([0-9A-Fa-f]+)\.png', file_name)
        if match:
            unicode_value = int(match.group(1), 16)
            print(f"File: {file_name}, Unicode Value: {unicode_value}")
        else:
            raise ValueError("Invalid file name format")
    except ValueError as e:
        print(f"Error processing file {file_name}: {e}")
        print(f"File Content: {file_name.encode('unicode_escape').decode()}")  # 打印文件内容的 Unicode 转义表示

# 使用os.fsdecode规范化文件名
sorted_image_files = sorted(
    image_files,
    key=lambda x: int(re.search(r'_U\+([0-9A-Fa-f]+)\.png', re.sub(r'^[\ufeff]+', '', os.fsdecode(x))).group(1), 16)
    if re.search(r'_U\+([0-9A-Fa-f]+)\.png', re.sub(r'^[\ufeff]+', '', os.fsdecode(x))) else 0
)

# 生成视频
output_file = os.path.join('output', output_file_name + '.mp4')  # 输出视频文件路径
convert_images_to_video(input_folder, output_file, frame_rate, sorted_image_files)
