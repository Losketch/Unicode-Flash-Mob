import requests
import msvcrt

# 下载UnicodeData.txt文件
url_unicode_data = 'https://unicode.org/Public/UNIDATA/UnicodeData.txt'
r_unicode_data = requests.get(url_unicode_data)
with open('UnicodeData.txt', 'wb') as f:
    f.write(r_unicode_data.content)
print('UnicodeData.txt downloaded successfully!')

# 下载Blocks.txt文件
url_blocks_data = 'https://unicode.org/Public/UNIDATA/Blocks.txt'
r_blocks_data = requests.get(url_blocks_data)
with open('UnicodeBlocks.txt', 'wb') as f:
    f.write(r_blocks_data.content)
print('UnicodeBlocks.txt downloaded successfully!')

# 等待用户按下任意键退出
print('Press any key to exit...')
msvcrt.getch()
