import os
import platform
import subprocess
import re
import sys
import configparser

os.chdir(os.path.dirname(sys.argv[0]))

def find_ffmpeg():
    ffmpeg_exe = "ffmpeg.exe" if platform.system() == "Windows" else "ffmpeg"

    for path in os.environ.get("PATH", "").split(os.pathsep):
        ffmpeg_path = os.path.join(path, ffmpeg_exe)
        if os.path.exists(ffmpeg_path):
            print(f"\n从环境变量中找到 {ffmpeg_exe} 文件\n在：{ffmpeg_path}\n")
            return ffmpeg_path

    local_ffmpeg_path = os.path.abspath(os.path.join(".", "ffmpeg", "bin", ffmpeg_exe))
    if os.path.exists(local_ffmpeg_path):
        print(f"\n在当前目录下找到 {ffmpeg_exe} 文件\n在：{local_ffmpeg_path}\n")
        return local_ffmpeg_path

    for root, dirs, files in os.walk("."):
        if ffmpeg_exe in files:
            ffmpeg_path = os.path.abspath(os.path.join(root, ffmpeg_exe))
            print(f"\n在当前目录搜索中找到 {ffmpeg_exe} 文件\n在：{ffmpeg_path}\n")
            return ffmpeg_path

    print(f"\n未找到 {ffmpeg_exe} 文件")
    print("请确保已安装 ffmpeg 并添加到环境变量，或将 ffmpeg 放在当前目录的 ffmpeg/bin/ 文件夹中")
    input("按任意键退出...")
    sys.exit(1)

def convert_images_to_video(image_folder, output_file, frame_rate, file_list):
    ffmpeg_path = find_ffmpeg()

    temp_file = 'temp.txt'
    try:
        with open(temp_file, 'w', encoding='utf-8') as file:
            image_duration = 1.0 / frame_rate
            for image_file in file_list:
                file.write(f"file '{os.path.join(image_folder, image_file)}'\n")
                file.write(f"duration {image_duration}\n")

            if file_list:
                file.write(f"file '{os.path.join(image_folder, file_list[-1])}'\n")

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
            '-vsync', 'vfr',
            '-avoid_negative_ts', 'make_zero',
            output_file
        ]

        print(f"开始转换视频，共 {len(file_list)} 张图片...")
        process = subprocess.Popen(ffmpeg_command)
        process.wait()
        if process.returncode == 0:
            print("视频转换完成！")
        else:
            print(f"FFmpeg 返回码：{process.returncode}")
            
    except Exception as e:
        print(f"转换过程中出现异常：{e}")
    finally:
        if os.path.exists(temp_file):
            os.remove(temp_file)

    return ffmpeg_path

def add_music_to_video(video_file, music_file, output_file, ffmpeg_path):
    if not os.path.exists(music_file):
        print(f"音乐文件不存在：{music_file}")
        return False
        
    ffmpeg_command = [
        ffmpeg_path,
        '-y',
        '-i', video_file,
        '-stream_loop', '-1',
        '-i', music_file,
        '-shortest',
        '-c:v', 'copy',
        '-c:a', 'aac',
        output_file
    ]

    try:
        print("正在添加音乐...")
        process = subprocess.run(ffmpeg_command, check=True)
        print("音乐添加完成！")
        return True
    except subprocess.CalledProcessError as e:
        print(f"添加音乐时出现错误：\n{e.stderr}")
        return False

def get_output_video_name():
    while True:
        output_file_name = input("请输入输出视频的名称：").strip().lstrip('\ufeff')
        if not output_file_name:
            print("文件名不能为空，请重新输入。")
            continue

        output_file_name = re.sub(r'[<>:"/\\|?*]', '_', output_file_name)
        output_file = os.path.join('output', output_file_name + '.mp4')

        if os.path.exists(output_file):
            replace = input("已存在同名文件，是否替换？(Y/N): ").strip().lower()
            if replace == 'y':
                return output_file_name
            else:
                continue
        else:
            return output_file_name

def get_frame_rate():
    while True:
        frame_rate_input = input("请输入视频的帧率（帧/秒，默认30）：").strip()
        if not frame_rate_input:
            return 30.0
            
        try:
            frame_rate = float(frame_rate_input)
            if frame_rate <= 0:
                print("帧率必须大于0，请重新输入。")
            else:
                return frame_rate
        except ValueError:
            print("无效的输入，请输入一个数字。")

def load_settings(config_file='settings.ini'):
    config = configparser.ConfigParser()
    script_dir = os.path.dirname(sys.argv[0])
    config_path = os.path.join(script_dir, config_file)

    if not os.path.exists(config_path):
        print(f"警告：配置文件 '{config_path}' 不存在。将使用默认音乐文件名。")
        return None

    try:
        config.read(config_path, encoding='utf-8')
        return config
    except configparser.Error as e:
        print(f"读取配置文件 '{config_path}' 时出错：{e}")
        return None

def main():
    os_type = platform.system()
    print(f"操作系统: {os_type}")
    
    script_path = os.path.realpath(__file__)
    print(f"脚本路径: {script_path}")
    print(f"当前工作目录: {os.getcwd()}")

    runtime_dir = getattr(sys, '_MEIPASS', os.path.abspath("."))
    input_folder = os.path.abspath(os.path.join(runtime_dir, 'png'))
    print(f"图片文件夹路径: {input_folder}")
    
    if not os.path.exists(input_folder):
        print(f"图片文件夹不存在：{input_folder}")
        input("按任意键退出...")
        sys.exit(1)

    output_dir = 'output'
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)
        print(f"创建输出文件夹：{output_dir}")

    output_file_name = get_output_video_name()
    frame_rate = get_frame_rate()
    print(f"\n输出文件名: {output_file_name}")
    print(f"帧率: {frame_rate}")

    image_files = [f for f in os.listdir(input_folder) if f.lower().endswith('.png')]
    if not image_files:
        print("在图片文件夹中未找到PNG文件")
        input("按任意键退出...")
        sys.exit(1)
        
    print(f"找到 {len(image_files)} 个PNG文件")

    valid_files = []
    for file_name in image_files:
        try:
            clean_name = file_name.lstrip('\ufeff')
            match = re.search(r'_U\+([0-9A-Fa-f]+)\.png', clean_name)
            if match:
                unicode_value = int(match.group(1), 16)
                valid_files.append((unicode_value, file_name))
            else:
                print(f"警告：文件名格式不符合要求，跳过：{file_name}")
        except ValueError as e:
            print(f"处理文件 {file_name} 时出错：{e}")

    if not valid_files:
        print("没有找到符合命名规则的PNG文件")
        input("按任意键退出...")
        sys.exit(1)

    valid_files.sort(key=lambda x: x[0])
    sorted_image_files = [f[1] for f in valid_files]
    print(f"有效文件：{len(sorted_image_files)} 个")

    output_file = os.path.join(output_dir, output_file_name + '.mp4')
    ffmpeg_path = convert_images_to_video(input_folder, output_file, frame_rate, sorted_image_files)

    add_music_choice = input("\n是否要为视频添加音乐? (y/n): ").strip().lower()
    if add_music_choice == 'y':
        config = load_settings()
        music_file_name_from_config = None
        if config and 'Paths' in config and 'music_file' in config['Paths']:
            music_file_name_from_config = config['Paths']['music_file']
            print(f"从配置文件中读取到音乐文件名为: {music_file_name_from_config}")
        else:
            print("未能在配置文件中找到 'music_file' 设置，将使用默认值 'DUTM.mp3'。")
            music_file_name_from_config = 'DUTM.mp3'

        music_file = os.path.join(os.path.dirname(sys.argv[0]), music_file_name_from_config)

        if os.path.exists(music_file):
            output_with_music = os.path.join(output_dir, output_file_name + '_music.mp4')
            if add_music_to_video(output_file, music_file, output_with_music, ffmpeg_path):
                print(f"带音乐的视频已保存为: {output_with_music}")
            else:
                print("添加音乐失败，但无音乐版本已保存")
        else:
            print(f"音乐文件不存在：{music_file}")
            print("仅保存无音乐版本")
    else:
        print("程序完成，未添加音乐")

    print(f"\n视频已保存为: {output_file}")
    input("按任意键退出...")

if __name__ == "__main__":
    main()