import os
import subprocess
import re
import sys

def convert_images_to_video(image_folder, output_file, frame_rate, file_list):
    ffmpeg_path = os.path.join(os.path.dirname(__file__), 'ffmpeg', 'bin', 'ffmpeg.exe')

    # 生成包含所有图像文件的临时文件
    temp_file = 'temp.txt'
    with open(temp_file, 'w') as file:
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

# 获取输入文件夹和输出文件路径
input_folder = 'png'  # 输入文件夹路径

# 获取用户输入的输出视频名称
def get_output_video_name():
    while True:
        output_file_name = input("请输入输出视频的名称：")  # 自定义输出视频名称
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
sorted_image_files = sorted(image_files, key=lambda x: int(x.split('_U+')[1].split('.')[0], 16))

# 生成视频
output_file = os.path.join('output', output_file_name + '.mp4')  # 输出视频文件路径
convert_images_to_video(input_folder, output_file, frame_rate, sorted_image_files)
