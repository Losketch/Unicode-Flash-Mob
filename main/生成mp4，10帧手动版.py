import os
import subprocess

def convert_images_to_video(image_folder, output_file):
    # 使用FFmpeg命令将图片合成视频
    ffmpeg_command = f'ffmpeg -y -framerate 8 -i {image_folder}/image_%d.png -c:v libx264 -crf 18 -pix_fmt yuv420p {output_file}'
    subprocess.run(ffmpeg_command, shell=True)

# 设置输入文件夹和输出文件路径
input_folder = 'png'  # 输入文件夹路径
output_file = 'output/video.mp4'  # 输出视频文件路径

# 获取文件夹中的图片文件
image_files = [f for f in os.listdir(input_folder) if f.endswith('.png')]

# 根据图片文件的U码进行排序
sorted_image_files = sorted(image_files, key=lambda x: int(x.split('_')[1].split('.')[0][2:], 16))

# 重命名图片文件，使文件名有序
for i, image_file in enumerate(sorted_image_files):
    os.rename(os.path.join(input_folder, image_file), os.path.join(input_folder, f'image_{i+1}.png'))

# 将图片转换为视频
convert_images_to_video(input_folder, output_file)

# 恢复原始文件名
for i, image_file in enumerate(sorted_image_files):
    os.rename(os.path.join(input_folder, f'image_{i+1}.png'), os.path.join(input_folder, image_file))
