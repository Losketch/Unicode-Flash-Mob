import tkinter as tk

# 定义RGB值
rgb_values = (194, 59, 79, 255)

# 创建窗口
window = tk.Tk()

# 创建画布
canvas = tk.Canvas(window, width=200, height=200)
canvas.pack()

# 设置画布的背景颜色
canvas.configure(background='#{:02x}{:02x}{:02x}'.format(rgb_values[0], rgb_values[1], rgb_values[2]))

# 显示窗口
window.mainloop()
