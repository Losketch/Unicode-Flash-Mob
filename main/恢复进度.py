import os
import re
from pathlib import Path

def get_processed_codepoints(png_dir):
    """
    获取已生成的png文件对应的码点
    忽略小于1000字节的文件
    """
    processed = set()
    png_path = Path(png_dir)

    if not png_path.exists():
        print(f"PNG目录不存在: {png_dir}")
        return processed
    
    pattern = re.compile(r'^image_(U\+[0-9A-Fa-f]{4,6})\.png$')

    for file_path in png_path.glob("image_U+*.png"):
        if file_path.stat().st_size < 1000:
            print(f"跳过小文件: {file_path.name} ({file_path.stat().st_size} bytes)")
            continue

        match = pattern.match(file_path.name)
        if match:
            codepoint = match.group(1)
            processed.add(codepoint)

    print(f"找到 {len(processed)} 个有效的已生成文件")
    return processed

def update_unicode_file(unicode_file, processed_codepoints):
    unicode_path = Path(unicode_file)

    if not unicode_path.exists():
        print(f"Unicode文件不存在: {unicode_file}")
        return

    backup_file = unicode_path.with_suffix('.txt.backup')

    unicode_path.rename(backup_file)
    print(f"已备份原文件为: {backup_file}")

    removed_count = 0
    total_lines = 0

    with open(backup_file, 'r', encoding='utf-8') as infile, \
         open(unicode_path, 'w', encoding='utf-8') as outfile:

        for line in infile:
            total_lines += 1
            line = line.strip()

            if not line:
                outfile.write('\n')
                continue

            parts = line.split(';')
            if len(parts) >= 2:
                # 提取码点部分，去掉引号
                codepoint_part = parts[1].strip('"')

                if codepoint_part in processed_codepoints:
                    removed_count += 1
                    print(f"删除已处理: {codepoint_part}")
                    continue

            outfile.write(line + '\n')

    print(f"\n处理完成:")
    print(f"原文件总行数: {total_lines}")
    print(f"删除已处理行数: {removed_count}")
    print(f"剩余待处理行数: {total_lines - removed_count}")
    print(f"更新后文件: {unicode_path}")

def main():
    # 配置路径
    png_directory = "png"
    unicode_file = "Unicode.txt"

    print("开始恢复进度...")
    print(f"PNG目录: {png_directory}")
    print(f"Unicode文件: {unicode_file}")
    print("-" * 50)

    # 获取已处理的码点
    processed = get_processed_codepoints(png_directory)

    if not processed:
        print("没有找到已处理的文件，无需更新Unicode文件")
        return

    print(f"\n已处理的码点示例: {list(processed)[:10]}...")

    update_unicode_file(unicode_file, processed)

    print("\n进度恢复完成！")

if __name__ == "__main__":
    main()