def replace_content(temp_file, unicode_file):
    # 读取UnicodeD.txt的内容
    with open(unicode_file, 'r', encoding='utf-8') as f:
        unicode_lines = f.readlines()

    # 读取temp.txt的内容
    with open(temp_file, 'r', encoding='utf-8') as f:
        temp_lines = f.readlines()

    # 创建一个字典，用于存储UnicodeD.txt中每行内容前面U+xxxx的部分和整行内容
    unicode_dict = {}
    for line in unicode_lines:
        key, value = line.strip().split('-', 1)  # 获取每行内容前面的U+xxxx和对应的描述
        unicode_dict[key] = key + '-' + value

    # 替换temp.txt中相同内容
    replaced_lines = []
    for line in temp_lines:
        key = line.strip()  # 获取每行内容前面的U+xxxx
        if key in unicode_dict:
            replaced_lines.append(unicode_dict[key] + '\n')
        else:
            replaced_lines.append(line)

    # 将替换后的内容写入temp.txt
    with open(temp_file, 'w', encoding='utf-8') as f:
        f.writelines(replaced_lines)

# 测试
replace_content('Unicode.txt', 'DecipherUnicodeBlocks.txt')
