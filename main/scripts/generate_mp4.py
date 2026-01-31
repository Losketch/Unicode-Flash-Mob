#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import os
import platform
import subprocess
import re
import sys
import logging
import tempfile

sys.path.insert(0, os.path.dirname(__file__))

from Module import Config, setup_logging

def find_ffmpeg():
    ffmpeg_exe = "ffmpeg.exe" if platform.system() == "Windows" else "ffmpeg"

    search_paths = [
        (os.path.join(path, ffmpeg_exe) for path in os.environ.get("PATH", "").split(os.pathsep)),
        [os.path.abspath(os.path.join(".", "ffmpeg", "bin", ffmpeg_exe))],
        [os.path.join(root, ffmpeg_exe) for root, _, files in os.walk(".") if ffmpeg_exe in files]
    ]

    for ffmpeg_path in (p for group in search_paths for p in group):
        if os.path.exists(ffmpeg_path):
            logging.info(f"\n找到 {ffmpeg_exe} 文件\n在：{ffmpeg_path}\n")
            return ffmpeg_path

    logging.error(f"\n未找到 {ffmpeg_exe} 文件")
    logging.info("请确保已安装 ffmpeg 并添加到环境变量，或将 ffmpeg 放在当前目录的 ffmpeg/bin/ 文件夹中")
    sys.exit(1)

def convert_images_to_video(image_folder, output_file, frame_rate, file_list):
    ffmpeg_path = find_ffmpeg()
    image_duration = 1.0 / frame_rate

    with tempfile.NamedTemporaryFile(mode='w', prefix='ffmpeg_concat_', suffix='.txt', delete=False) as temp_file:
        temp_file_name = temp_file.name
        for image_file in file_list:
            temp_file.write(f"file '{os.path.join(image_folder, image_file).replace('\\', '/')}'\n")
            temp_file.write(f"duration {image_duration}\n")
        if file_list:
            temp_file.write(f"file '{os.path.join(image_folder, file_list[-1])}'\n")

    try:
        ffmpeg_command = [
            ffmpeg_path, '-y', '-r', str(frame_rate), '-f', 'concat', '-safe', '0',
            '-i', temp_file_name, '-c:v', 'libx264', '-crf', '18', '-preset', 'fast',
            '-pix_fmt', 'yuv420p', '-fflags', '+genpts+discardcorrupt', '-vsync', 'vfr',
            '-avoid_negative_ts', 'make_zero', '-threads', str(os.cpu_count() or 4), output_file
        ]

        logging.info(f"开始转换视频，共 {len(file_list)} 张图片...")
        logging.info(f"使用 {os.cpu_count() or 4} 个线程进行编码")
        process = subprocess.Popen(ffmpeg_command)
        process.wait()

        if process.returncode == 0:
            logging.info("视频转换完成！")
        else:
            logging.error(f"FFmpeg 返回码：{process.returncode}, 命令: {' '.join(ffmpeg_command)}")

    except Exception as e:
        logging.exception(f"转换过程中出现异常：{e}")
    finally:
        if os.path.exists(temp_file_name):
            os.remove(temp_file_name)

    return ffmpeg_path

def add_music_to_video(video_file, music_file, output_file, ffmpeg_path):
    if not os.path.exists(music_file):
        logging.warning(f"音乐文件不存在：{music_file}")
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
        logging.info("正在添加音乐...")
        process = subprocess.run(ffmpeg_command, check=True)
        logging.info("音乐添加完成！")
        return True
    except subprocess.CalledProcessError as e:
        logging.error(f"添加音乐时出现错误：\n{e.stderr}")
        return False

def get_output_video_name():
    while True:
        output_file_name = input("请输入输出视频的名称：").strip().lstrip('\ufeff')
        if not output_file_name:
            logging.warning("文件名不能为空，请重新输入。")
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

def parse_unicode_filename(filename: str) -> tuple[bool, int, str]:
    clean_name = filename.lstrip('\ufeff')
    match = re.search(r'_U\+([0-9A-Fa-f]+)\.png', clean_name)
    if match:
        return True, int(match.group(1), 16), filename
    return False, 0, filename

def get_frame_rate():
    while True:
        frame_rate_input = input("请输入视频的帧率（帧/秒，默认30）：").strip()
        if not frame_rate_input:
            return 30.0
            
        try:
            frame_rate = float(frame_rate_input)
            if frame_rate <= 0:
                logging.warning("帧率必须大于0，请重新输入。")
            else:
                return frame_rate
        except ValueError:
            logging.warning("无效的输入，请输入一个数字。")

def cleanup_png_files(input_folder: str, confirm: bool = True) -> int:
    if not os.path.exists(input_folder):
        return 0

    png_files = [f for f in os.listdir(input_folder) if f.lower().endswith('.png')]
    if not png_files:
        logging.info("没有找到PNG文件需要清理")
        return 0

    total_size = sum(os.path.getsize(os.path.join(input_folder, f)) for f in png_files)
    size_mb = total_size / (1024 * 1024)
    logging.info(f"发现 {len(png_files)} 个PNG文件，占用 {size_mb:.2f} MB")

    if confirm:
        choice = input("是否清理这些中间PNG文件？(y/n): ").strip().lower()
        if choice != 'y':
            logging.info("跳过清理")
            return 0

    deleted_count = 0
    for f in png_files:
        try:
            os.remove(os.path.join(input_folder, f))
            deleted_count += 1
        except OSError:
            pass

    logging.info(f"已清理 {deleted_count} 个PNG文件，释放 {size_mb:.2f} MB 磁盘空间")
    return deleted_count

def main():
    setup_logging()
    cfg = Config()
    script_path = os.path.realpath(__file__)
    logging.info(f"脚本路径: {script_path}")
    logging.info(f"当前工作目录: {os.getcwd()}")

    runtime_dir = getattr(sys, '_MEIPASS', os.path.abspath("."))
    input_folder = os.path.abspath(os.path.join(runtime_dir, 'png'))
    logging.info(f"图片文件夹路径: {input_folder}")

    if not os.path.exists(input_folder):
        logging.error(f"图片文件夹不存在：{input_folder}")
        sys.exit(1)

    output_dir = 'output'
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)
        logging.info(f"创建输出文件夹：{output_dir}")

    output_file_name = get_output_video_name()
    frame_rate = get_frame_rate()
    logging.info(f"\n输出文件名: {output_file_name}")
    logging.info(f"帧率: {frame_rate}")

    image_files = sorted(
        f for f in os.listdir(input_folder)
        if f.lower().endswith(('.png'))
    )
    if not image_files:
        logging.error("在图片文件夹中未找到PNG文件")
        sys.exit(1)

    logging.info(f"找到 {len(image_files)} 个PNG文件")

    valid_files = []
    for file_name in image_files:
        success, unicode_value, _ = parse_unicode_filename(file_name)
        if success:
            valid_files.append((unicode_value, file_name))
        else:
            logging.warning(f"警告：文件名格式不符合要求，跳过：{file_name}")

    if not valid_files:
        logging.warning("没有找到符合命名规则的PNG文件")
        sys.exit(1)

    valid_files.sort(key=lambda x: x[0])
    sorted_image_files = [f[1] for f in valid_files]
    logging.info(f"有效文件：{len(sorted_image_files)} 个")

    output_file = os.path.join(output_dir, output_file_name + '.mp4')
    ffmpeg_path = convert_images_to_video(input_folder, output_file, frame_rate, sorted_image_files)

    add_music_choice = input("\n是否要为视频添加音乐? (y/n): ").strip().lower()
    if add_music_choice == 'y':
        music_file = cfg.music_file

        if os.path.exists(music_file):
            output_with_music = os.path.join(output_dir, output_file_name + '_music.mp4')
            if add_music_to_video(output_file, music_file, output_with_music, ffmpeg_path):
                logging.info(f"带音乐的视频已保存为: {output_with_music}")
            else:
                logging.error("添加音乐失败，但无音乐版本已保存")
        else:
            logging.error(f"音乐文件不存在：{music_file}")
            logging.warning("仅保存无音乐版本")
    else:
        logging.info("程序完成，未添加音乐")

    logging.info(f"\n视频已保存为: {output_file}")

    cleanup_choice = input("\n是否清理中间PNG文件以释放磁盘空间？(y/n): ").strip().lower()
    if cleanup_choice == 'y':
        cleanup_png_files(input_folder, confirm=False)
    else:
        logging.info("保留中间PNG文件")

if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        logging.exception("程序崩溃：%s", e)
        sys.exit(1)