use std::fs::File;
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let mut file = File::create("settings.ini")?;

    writeln!(file, "[Settings]")?;
    writeln!(file, "ctrl_font_file = Ctrl-Ctrl.ttf")?;
    writeln!(file, "middle_font_size = 512")?;
    writeln!(file, "middle_font_color = 255,255,255,255")?;
    writeln!(file, "text_position_x = 0")?;
    writeln!(file, "text_position_y = 0")?;
    writeln!(file, "bottom_font_size = 19")?;
    writeln!(file, "background_color = 0,0,0,255")?;
    writeln!(file, "")?;
    writeln!(file, "[Paths]")?;
    writeln!(file, "music_file = DUTM.m4a")?;

    Ok(())
}