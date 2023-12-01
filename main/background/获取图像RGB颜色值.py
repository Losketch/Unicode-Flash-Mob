import PIL.Image

image = PIL.Image.open('1.png')

# 获取图像的RGB颜色值
rgb_values = image.getpixel((0, 0))

print(rgb_values)
