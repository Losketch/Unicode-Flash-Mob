def replace_content(temp_file, option):
    file_map = {
        '1': 'DecipherUnicodeBlocks.txt',
        '2': 'DecipherUnicodeData.txt',
        '3': 'DecipherUnicodeLosketchBlocks.txt',
        '4': 'DecipherUnicodeDataBlocks.txt'
    }

    while option not in ['1', '2', '3', '4']:
        option = input("\n输入非1~4，请重新选择：")

    unicode_file = file_map[option]

    # 读取Unicode文件的内容
    with open(unicode_file, 'r', encoding='utf-8') as f:
        unicode_lines = f.readlines()

    # 读取temp.txt的内容
    with open(temp_file, 'r', encoding='utf-8') as f:
        temp_lines = f.readlines()

    # 创建一个字典，用于存储Unicode文件中每行内容前面U+xxxx的部分和整行内容
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
option = input("请选择生成图片左下角说明文件\n\n1.选择DecipherUnicodeBlocks.txt 无翻译各个字符区块\n\n2.选择DecipherUnicodeData.txt  每个字符的详细信息\n\n3.选择DecipherUnicodeLosketchBlocks.txt 有翻译各个字符区块\n\n4.选择DecipherUnicodeDataBlocks.txt 每个字符的详细信息+有翻译字符区块信息\n\n你选择：")
replace_content('Unicode.txt', option)
