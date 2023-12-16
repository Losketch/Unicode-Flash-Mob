import configparser

# 写入settings.ini文件
config = configparser.ConfigParser()
config['Settings'] = {'middle_font_size': '512', 'text_position_x': '0', 'text_position_y': '0'}

with open('settings.ini', 'w') as configfile:
    config.write(configfile)
