# Optional bundled assets

Place optional runtime assets in this directory before packaging:

- `data/UnicodeData.txt`
- `data/UnicodeBlocks.txt` — downloaded from Unicode's upstream `Blocks.txt`
- `fonts/`
- `audio/`
- `ffmpeg/bin/ffmpeg.exe` on Windows
- `ffmpeg/bin/ffmpeg` on macOS and Linux

When an asset is absent, the application uses user-selected fonts, empty Unicode metadata, or FFmpeg from the configured path or process `PATH`.

Store bundled references in configuration files as portable `assets/...` paths, for example `assets/fonts/Example.ttf`. The application resolves them to the development or packaged resource directory at runtime.

GUI applications launched from Finder or a desktop menu on macOS and Linux may not inherit the shell's complete `PATH`. Bundle FFmpeg or configure its absolute path when system discovery is unreliable.

Ensure bundled executables have the required platform permissions. Only distribute fonts, audio, FFmpeg builds and other third-party assets when their licenses permit redistribution.
