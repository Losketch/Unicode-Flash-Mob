import configparser

# 写入settings.ini文件
config = configparser.ConfigParser()
config['Settings'] = {'middle_font_size': '512', 'middle_font_color': '255,255,255,255', 'text_position_x': '0', 'text_position_y': '0', 'background_color': '0,0,0,255'}

with open('settings.ini', 'w') as configfile:
    config.write(configfile)
