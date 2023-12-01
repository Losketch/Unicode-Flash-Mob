import os

def merge_files(input_file1, input_file2, output_file):
    # Create a dictionary to store content from DecipherUnicodeLosketchBlocks.txt
    unicode_blocks_dict = {}

    with open(input_file2, 'r', encoding='utf-8') as file2:
        for line2 in file2:
            unicode_value2, rest2 = line2.strip().split('-', 1)
            unicode_blocks_dict[unicode_value2] = line2.strip()

    with open(input_file1, 'r', encoding='utf-8') as file1, \
         open(output_file, 'w', encoding='utf-8') as output:

        for line1 in file1:
            # Extract Unicode value from DecipherUnicodeData.txt
            unicode_value1, rest1 = line1.strip().split('-', 1)

            # Find corresponding line in the dictionary
            if unicode_value1 in unicode_blocks_dict:
                # Write merged line to output file with a newline character
                output.write(f"{line1.strip()}|{unicode_blocks_dict[unicode_value1]}\n")

if __name__ == "__main__":
    merge_files("DecipherUnicodeData.txt", "DecipherUnicodeLosketchBlocks.txt", "Temp.txt")



def modify_output_file(input_file, output_file):
    with open(input_file, 'r', encoding='utf-8') as input_file, \
         open(output_file, 'w', encoding='utf-8') as output:

        for line in input_file:
            # Find the first "|" character
            first_pipe_index = line.find('|')
            
            if first_pipe_index != -1:
                # Find the next "-" character after the first "|"
                next_dash_index = line.find('-', first_pipe_index)

                # If there is a "-" character after the first "|", remove the substring between them
                if next_dash_index != -1:
                    output.write(f"{line[:first_pipe_index+1]}{line[next_dash_index+1:]}")
                else:
                    # If there is no "-" character after the first "|", write the entire line
                    output.write(line)
            else:
                # If "|" is not found, write the entire line to output
                output.write(line)

if __name__ == "__main__":
    modify_output_file("Temp.txt", "DecipherUnicodeDataBlocks.txt")



# 删除文件部分
# 要删除的文件列表
files_to_delete = ["temp.txt"]

for file_to_delete in files_to_delete:
    try:
        os.remove(file_to_delete)
        print(f"{file_to_delete} 文件已成功删除。")
    except FileNotFoundError:
        print(f"{file_to_delete} 文件未找到。")
    except Exception as e:
        print(f"删除文件时发生错误: {e}")
