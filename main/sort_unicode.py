import argparse
import re
import sys

def parse_args():
    parser = argparse.ArgumentParser(
        description="按 Unicode 码点从小到大排序带有 U+XXXX 字段的行，并警告重复码点；可删除分号数量不足的行。"
    )
    parser.add_argument(
        "infile",
        help="待排序的输入文件，文本格式，每行包含一个 U+XXXX 字段"
    )
    parser.add_argument(
        "-o", "--outfile",
        help="排序后的输出文件（可选，不指定则打印到标准输出）"
    )
    parser.add_argument(
        "--min-semi",
        type=int,
        default=0,
        help="删除;号数量少于该值的行，默认 0（即不删除）。"
    )
    return parser.parse_args()

RE_CODE = re.compile(r'U\+([0-9A-Fa-f]{1,6})')

def extract_codepoint(line):
    """
    从一行文本中提取 U+XXXX 字段，返回对应的整数码点。
    找不到则返回 -1（会排在末尾）。
    """
    m = RE_CODE.search(line)
    if not m:
        return -1
    try:
        return int(m.group(1), 16)
    except ValueError:
        return -1

def main():
    args = parse_args()

    try:
        with open(args.infile, encoding="utf-8") as f:
            lines = [ln.rstrip("\n") for ln in f]
    except FileNotFoundError:
        sys.exit(f"错误：找不到文件 {args.infile}")
    except Exception as e:
        sys.exit(f"读取文件出错：{e}")

    if args.min_semi > 0:
        filtered = []
        for idx, line in enumerate(lines):
            cnt = line.count(";")
            if cnt < args.min_semi:
                sys.stderr.write(f"警告：第 {idx+1} 行分号数 ({cnt}) 少于 {args.min_semi}，已删除：{line}\n")
            else:
                filtered.append(line)
        lines = filtered

    decorated = []
    for idx, line in enumerate(lines):
        cp = extract_codepoint(line)
        decorated.append((cp, idx, line))

    cp_counts = {}
    for cp, _, _ in decorated:
        cp_counts[cp] = cp_counts.get(cp, 0) + 1
    for cp, cnt in sorted(cp_counts.items()):
        if cp >= 0 and cnt > 1:
            sys.stderr.write(f"警告：代码点 U+{cp:04X} 出现 {cnt} 次\n")

    decorated.sort(key=lambda x: (x[0], x[1]))

    sorted_lines = [item[2] for item in decorated]

    if args.outfile:
        try:
            with open(args.outfile, "w", encoding="utf-8") as f:
                for line in sorted_lines:
                    f.write(line + "\n")
        except Exception as e:
            sys.exit(f"写入文件出错：{e}")
    else:
        for line in sorted_lines:
            print(line)

if __name__ == "__main__":
    main()