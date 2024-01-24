#include <stdio.h>

int main() {
    FILE *configfile = fopen("settings.ini", "w");
    if (configfile == NULL) {
        printf("Error opening file for writing.");
        return 1;
    }

    fprintf(configfile, "[Settings]\n");
    fprintf(configfile, "middle_font_size = 512\n");
    fprintf(configfile, "middle_font_color = 255,255,255,255\n");
    fprintf(configfile, "text_position_x = 0\n");
    fprintf(configfile, "text_position_y = 0\n");
    fprintf(configfile, "bottom_font_size = 19\n");
    fprintf(configfile, "background_color = 0,0,0,255\n");

    fclose(configfile);

    return 0;
}
