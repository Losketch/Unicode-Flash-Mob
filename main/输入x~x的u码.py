import os

current_path = os.getcwd()

file_name = input("请输入文件名（不用加.txt）：")
if not file_name.endswith(".txt"):
    file_name += ".txt"

file_path = os.path.join(current_path, file_name)

max_unicode = 0x10FFFF

while True:
    start = input("请输入 Unicode 起始值[十六进制]（不要什么都没输入就回车）：")
    end = input("请输入 Unicode 结束值[十六进制]（否则程序崩溃）：")

    valid_chars = set("0123456789abcdefABCDEF")  # 合法的十六进制字符
    if not set(start.lower()).issubset(valid_chars) or not set(end.lower()).issubset(valid_chars):
        print("\n输入的十六进制值包含非法字符！")
        input("请按任意键继续...\n")
    else:
        start = int(start, 16)
        end = int(end, 16)

        if start > max_unicode or end > max_unicode:
            print("\n你输入的位置不在当前世界的 Unicode 编码范围内！")
        elif start > end:
            print("\n起始值不能大于结束值！")
        else:
            break

with open(file_path, "w", encoding="utf-8") as f:
    print("\n正在生成文件，请稍候...")
    for i in range(start, end + 1):
        f.write("U+{:04X}\n".format(i))

print("\n文件生成成功！文件名：", file_name)
