def swap_content(input_file, output_file):
    with open(input_file, 'r', encoding='utf-8') as input_file, \
         open(output_file, 'w', encoding='utf-8') as output:

        for line in input_file:
            # 检查 "|" 符号是否存在
            if '|' in line:
                # 分割每行为两部分：";" 之前和之后
                first_part, second_part = line.split(';')
                # 限制 split 分割次数为 1，确保 second_part 只被分割成两部分
                part1, part2 = second_part.split('|', 1)
                # 交换 "|" 之后的两部分内容
                swapped_line = f'{part2.strip()}|{part1.strip()}'
                # 将交换后的内容拼接回原始行
                output.write(f"{first_part}; {swapped_line}\n")
            else:
                # 如果没有 "|"，原样输出该行
                output.write(line)

if __name__ == "__main__":
    swap_content("UnicodeLosketchBlocks.txt", "SwappedUnicodeLosketchBlocks.txt")
