using System;
using System.IO;

class Program
{
    static void Main()
    {
        using (StreamWriter configfile = new StreamWriter("settings.ini"))
        {
            if (configfile == null)
            {
                Console.WriteLine("Error opening file for writing.");
                return;
            }

            configfile.WriteLine("[Settings]");
            configfile.WriteLine("middle_font_size = 512");
            configfile.WriteLine("middle_font_color = 255,255,255,255");
            configfile.WriteLine("text_position_x = 0");
            configfile.WriteLine("text_position_y = 0");
            configfile.WriteLine("bottom_font_size = 28");
            configfile.WriteLine("background_color = 0,0,0,255");
        }

    }
}