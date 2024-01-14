def find_longest_line(file_path):
    max_length = 0
    max_line_number = 0
    max_line_content = ""

    with open(file_path, 'r', encoding='utf-8') as file:
        lines = file.readlines()
        for i, line in enumerate(lines, start=1):
            line_length = len(line)
            if line_length > max_length:
                max_length = line_length
                max_line_number = i
                max_line_content = line.rstrip()  # Remove trailing newline characters

    return max_line_number, max_line_content

file_path = 'DecipherUnicodeData.txt'  # 替换成你的文件路径
line_number, line_content = find_longest_line(file_path)

print(f"\n最长的一行在第 {line_number} 行，内容是：\n")
print(line_content)
input(f"\n按任意键退出 . . .")
