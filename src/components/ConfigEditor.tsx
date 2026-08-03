import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useSnackbar } from "notistack";
import {
  Button,
  Divider,
  FormControl,
  FormControlLabel,
  Grid,
  IconButton,
  InputLabel,
  InputAdornment,
  MenuItem,
  Paper,
  Select,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@mui/material";
import FolderOpenIcon from "@mui/icons-material/FolderOpen";
import Tooltip from "@mui/material/Tooltip";
import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
import BackgroundColorList from "./BackgroundColorList";
import ColorField from "./ColorField";
import ConfigSection from "./ConfigSection";
import FontList from "./FontList";
import JsonEditor from "./JsonEditorField";
import { useSettings } from "../contexts/SettingsContext";
import { setNestedValue } from "../utils/config";
import type {
  Color,
  RenderConfig,
  RenderConfigPath,
  TextElement,
} from "../types/config";

interface ConfigEditorProps {
  config: RenderConfig | null;
  onConfigChange: (config: RenderConfig) => void;
}

const ConfigEditor: React.FC<ConfigEditorProps> = ({ config, onConfigChange }) => {
  const { t } = useTranslation(["config", "common"]);
  const { enqueueSnackbar } = useSnackbar();
  const { settings, setEncoderAvc, setEncoderHevc } = useSettings();
  useEffect(() => {
    if (!config) return;
    const preferredEncoder = settings.encoderAvc || settings.encoderHevc;
    const currentEncoder = config.ffmpeg.encoder;
    if (
      !preferredEncoder ||
      (currentEncoder && currentEncoder !== "auto")
    ) {
      return;
    }

    onConfigChange({
      ...config,
      ffmpeg: { ...config.ffmpeg, encoder: preferredEncoder },
    });
  }, [config, onConfigChange, settings.encoderAvc, settings.encoderHevc]);

  const showAlert = (message: string, variant: "success" | "error") => {
    enqueueSnackbar(message, { variant });
  };

  const handleLoadConfig = async () => {
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: "JSON Files", extensions: ["json"] }],
      });
      if (path) {
        const loadedConfig = await invoke<RenderConfig>("load_config", { path });
        onConfigChange(loadedConfig);
        showAlert(t("config:loadSuccess"), "success");
      }
    } catch (error) {
      showAlert(`${t("config:loadFailed")}: ${error}`, "error");
    }
  };

  const handleSaveConfig = async () => {
    if (!config) return;
    try {
      const path = await save({
        filters: [{ name: "JSON Files", extensions: ["json"] }],
      });
      if (path) {
        await invoke("save_config", { path, config });
        showAlert(t("config:saveSuccess"), "success");
      }
    } catch (error) {
      showAlert(`${t("config:saveFailed")}: ${error}`, "error");
    }
  };

  const handleGetDefault = async () => {
    try {
      const defaultConfig = await invoke<RenderConfig>("get_default_config");
      onConfigChange(defaultConfig);
      showAlert(t("config:defaultLoaded"), "success");
    } catch (error) {
      showAlert(`${t("config:getDefaultFailed")}: ${error}`, "error");
    }
  };

  if (!config) {
    return (
      <Paper elevation={2} sx={{ p: 2.5, borderRadius: 3 }}>
        <Stack spacing={2.5}>
          <Typography variant="h5">{t("config:basic")}</Typography>
          <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
            <Button variant="contained" onClick={handleLoadConfig}>
              {t("config:loadConfig")}
            </Button>
            <Button variant="outlined" onClick={handleGetDefault}>
              {t("config:loadDefault")}
            </Button>
          </Stack>
          <Typography color="text.secondary">
            {t("config:emptyHint")}
          </Typography>
        </Stack>
      </Paper>
    );
  }

  const selectOutputPath = async () => {
    const path = await save({
      filters: [
        { name: "Video Files", extensions: ["mp4", "mkv", "webm", "mov"] },
      ],
    });
    if (path) updateConfig("output_path", path);
  };

  const selectMusicPath = async () => {
    const path = await open({
      multiple: false,
      filters: [
        {
          name: "Audio Files",
          extensions: ["mp3", "wav", "flac", "ogg", "m4a", "aac"],
        },
      ],
    });
    if (path && !Array.isArray(path)) updateConfig("music_path", path);
  };

  const selectFfmpegPath = async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: "FFmpeg", extensions: ["exe", "bat", "cmd"] }],
    });
    if (path && !Array.isArray(path)) updateConfig("ffmpeg.path", path);
  };

  const updateConfig = (path: RenderConfigPath, value: unknown) => {
    onConfigChange(setNestedValue(config, path, value));
  };

  const renderColorPicker = (
    path: RenderConfigPath,
    color: Color | null,
    label: string
  ) => {
    if (!color) return null;
    return (
      <ColorField
        label={label}
        value={color}
        onChange={(next) => updateConfig(path, next)}
      />
    );
  };

  const renderTextElement = (
    path: "main_text" | "bottom_text",
    title: string
  ) => {
    const element: TextElement = config[path];
    return (
      <Stack spacing={2}>
        <Typography variant="subtitle1">{title}</Typography>
        <Grid container spacing={2} sx={{ width: "100%", margin: 0 }}>
          <Grid item xs={12} sm={path === "bottom_text" ? 4 : 12}>
            <TextField
              fullWidth
              label={t("config:elementId")}
              value={element.id || ""}
              onChange={(event) => updateConfig(`${path}.id`, event.target.value)}
            />
          </Grid>
          {path === "bottom_text" && (
            <Grid item xs={12} sm={8}>
              <TextField
                fullWidth
                label={t("config:textContent")}
                value={element.content || ""}
                onChange={(event) => updateConfig(`${path}.content`, event.target.value)}
              />
            </Grid>
          )}
          <Grid item xs={6} sm={3}>
            <TextField
              fullWidth
              type="number"
              label={t("config:positionX")}
              value={element.position?.x ?? 0}
              onChange={(event) => updateConfig(`${path}.position.x`, Number(event.target.value))}
            />
          </Grid>
          <Grid item xs={6} sm={3}>
            <TextField
              fullWidth
              type="number"
              label={t("config:positionY")}
              value={element.position?.y ?? 0}
              onChange={(event) => updateConfig(`${path}.position.y`, Number(event.target.value))}
            />
          </Grid>
          <Grid item xs={6} sm={3}>
            <FormControl fullWidth>
              <InputLabel>{t("config:textAlign")}</InputLabel>
              <Select
                label={t("config:textAlign")}
                value={element.align || "left"}
                onChange={(event) => updateConfig(`${path}.align`, event.target.value)}
              >
                <MenuItem value="left">{t("config:alignLeft")}</MenuItem>
                <MenuItem value="center">{t("config:alignCenter")}</MenuItem>
                <MenuItem value="right">{t("config:alignRight")}</MenuItem>
              </Select>
            </FormControl>
          </Grid>
          <Grid item xs={6} sm={3}>
            <TextField
              fullWidth
              type="number"
              label={t("config:maxWidth")}
              value={element.max_width ?? 0}
              onChange={(event) => updateConfig(`${path}.max_width`, Number(event.target.value))}
            />
          </Grid>
        </Grid>
        <Stack direction={{ xs: "column", sm: "row" }} spacing={1}>
          <FormControlLabel
            control={
              <Switch
                checked={Boolean(element.enabled)}
                onChange={(event) => updateConfig(`${path}.enabled`, event.target.checked)}
              />
            }
            label={t("config:elementEnabled")}
          />
          <FormControlLabel
            control={
              <Switch
                checked={Boolean(element.wrap)}
                onChange={(event) => updateConfig(`${path}.wrap`, event.target.checked)}
              />
            }
            label={t("config:textWrap")}
          />
        </Stack>
        {renderColorPicker(`${path}.color`, element.color, t("config:textColor"))}
      </Stack>
    );
  };

  const backgroundColors = Array.isArray(config.background_colors)
    ? config.background_colors
    : [];

  const section = (title: string, children: React.ReactNode, defaultExpanded = false) => (
    <ConfigSection title={title} defaultExpanded={defaultExpanded}>
      {children}
    </ConfigSection>
  );

  return (
    <Paper elevation={2} sx={{ p: 2.5, borderRadius: 3 }}>
      <Stack spacing={2.5}>
        <Typography variant="h5">{t("config:basic")}</Typography>
        <Stack direction={{ xs: "column", sm: "row" }} spacing={2}>
          <Button variant="contained" onClick={handleLoadConfig}>
            {t("config:loadConfig")}
          </Button>
          <Button variant="contained" onClick={handleSaveConfig}>
            {t("config:saveConfig")}
          </Button>
          <Button variant="outlined" onClick={handleGetDefault}>
            {t("config:loadDefault")}
          </Button>
        </Stack>
        <Divider />

        <Stack spacing={1}>
            {section(
              t("config:outputVideo"),
              <Grid container spacing={2} sx={{ width: "100%", margin: 0 }}>
                <Grid item xs={12} md={6}>
                  <TextField
                    fullWidth
                    label={t("config:outputPath")}
                    value={config.output_path || ""}
                    onChange={(event) => updateConfig("output_path", event.target.value)}
                    InputProps={{
                      endAdornment: (
                        <InputAdornment position="end">
                          <Tooltip title={t("config:chooseOutputFile")}>
                            <IconButton edge="end" onClick={selectOutputPath}>
                              <FolderOpenIcon />
                            </IconButton>
                          </Tooltip>
                        </InputAdornment>
                      ),
                    }}
                  />
                </Grid>
                <Grid item xs={12} md={6}>
                  <TextField
                    fullWidth
                    label={t("config:titleField")}
                    value={config.title || ""}
                    onChange={(event) => updateConfig("title", event.target.value)}
                  />
                </Grid>
                <Grid item xs={12} md={6}>
                  <TextField
                    fullWidth
                    label={t("config:musicPath")}
                    value={config.music_path || ""}
                    onChange={(event) =>
                      updateConfig("music_path", event.target.value || null)
                    }
                    InputProps={{
                      endAdornment: (
                        <InputAdornment position="end">
                          <Tooltip title={t("config:chooseMusicFile")}>
                            <IconButton edge="end" onClick={selectMusicPath}>
                              <FolderOpenIcon />
                            </IconButton>
                          </Tooltip>
                        </InputAdornment>
                      ),
                    }}
                  />
                </Grid>
                <Grid item xs={6} md={2}>
                  <TextField
                    fullWidth
                    type="number"
                    label={t("config:width")}
                    value={config.resolution?.[0] || 1920}
                    onChange={(event) =>
                      updateConfig("resolution", [
                        parseInt(event.target.value) || 1,
                        config.resolution?.[1] || 1080,
                      ])
                    }
                  />
                </Grid>
                <Grid item xs={6} md={2}>
                  <TextField
                    fullWidth
                    type="number"
                    label={t("config:height")}
                    value={config.resolution?.[1] || 1080}
                    onChange={(event) =>
                      updateConfig("resolution", [
                        config.resolution?.[0] || 1920,
                        parseInt(event.target.value) || 1,
                      ])
                    }
                  />
                </Grid>
                <Grid item xs={12} md={2}>
                  <TextField
                    fullWidth
                    type="number"
                    label={t("config:fps")}
                    value={config.fps || 30}
                    onChange={(event) => updateConfig("fps", Number(event.target.value) || 1)}
                  />
                </Grid>
              </Grid>,
              true
            )}

            {section(
              t("config:font"),
              <Stack spacing={3}>
                <Stack spacing={2}>
                  <Typography variant="subtitle1">{t("config:mainFont")}</Typography>
                  <JsonEditor
                    label={t("config:fontFeatureSettings")}
                    value={config.main_font?.font_feature_settings || {}}
                    onCommit={(value) => updateConfig("main_font.font_feature_settings", value)}
                    invalidMessage={t("config:invalidJson")}
                    hint={t("config:fontFeatureHint")}
                  />
                  <JsonEditor
                    label={t("config:fontVariationSettings")}
                    value={config.main_font?.font_variation_settings || {}}
                    onCommit={(value) => updateConfig("main_font.font_variation_settings", value)}
                    invalidMessage={t("config:invalidJson")}
                    hint={t("config:fontVariationHint")}
                  />
                  <FormControlLabel
                    control={
                      <Switch
                        checked={config.main_font?.font_optical_sizing !== false}
                        onChange={(event) =>
                          updateConfig("main_font.font_optical_sizing", event.target.checked)
                        }
                      />
                    }
                    label={t("config:fontOpticalSizing")}
                  />
                </Stack>
                <Divider />
                <Stack spacing={2}>
                  <Typography variant="subtitle1">{t("config:bottomFont")}</Typography>
                  <TextField
                    fullWidth
                    type="number"
                    label={t("config:bottomFontSize")}
                    value={config.bottom_font?.size || 42}
                    onChange={(event) =>
                      updateConfig("bottom_font.size", Number(event.target.value) || 1)
                    }
                  />
                  <FontList
                    fonts={config.bottom_text?.fonts ?? []}
                    onChange={(fonts) => updateConfig("bottom_text.fonts", fonts)}
                    label={t("config:bottomFontPath")}
                    multiple
                  />
                  <JsonEditor
                    label={t("config:fontFeatureSettings")}
                    value={config.bottom_font?.font_feature_settings || {}}
                    onCommit={(value) => updateConfig("bottom_font.font_feature_settings", value)}
                    invalidMessage={t("config:invalidJson")}
                    hint={t("config:fontFeatureHint")}
                  />
                  <JsonEditor
                    label={t("config:fontVariationSettings")}
                    value={config.bottom_font?.font_variation_settings || {}}
                    onCommit={(value) => updateConfig("bottom_font.font_variation_settings", value)}
                    invalidMessage={t("config:invalidJson")}
                    hint={t("config:fontVariationHint")}
                  />
                  <FormControlLabel
                    control={
                      <Switch
                        checked={config.bottom_font?.font_optical_sizing !== false}
                        onChange={(event) =>
                          updateConfig("bottom_font.font_optical_sizing", event.target.checked)
                        }
                      />
                    }
                    label={t("config:fontOpticalSizing")}
                  />
                </Stack>
              </Stack>
            )}

            {section(
              t("config:textLayout"),
              <Stack spacing={3}>
                <Grid container spacing={2} sx={{ width: "100%", margin: 0 }}>
                  <Grid item xs={6}>
                    <TextField
                      fullWidth
                      type="number"
                      label={t("config:textXOffset")}
                      value={config.text_x_offset ?? 0}
                      onChange={(event) =>
                        updateConfig("text_x_offset", parseInt(event.target.value) || 0)
                      }
                    />
                  </Grid>
                  <Grid item xs={6}>
                    <TextField
                      fullWidth
                      type="number"
                      label={t("config:textYOffset")}
                      value={config.text_y_offset ?? 0}
                      onChange={(event) =>
                        updateConfig("text_y_offset", parseInt(event.target.value) || 0)
                      }
                    />
                  </Grid>
                </Grid>
                {renderTextElement("main_text", t("config:mainText"))}
                <Divider />
                {renderTextElement("bottom_text", t("config:bottomText"))}
              </Stack>
            )}

            {section(
              t("config:backgroundDisplay"),
              <Stack spacing={2}>
                <Stack direction={{ xs: "column", sm: "row" }} spacing={1}>
                  <FormControlLabel
                    control={
                      <Switch
                        checked={Boolean(config.dynamic_background)}
                        onChange={(event) =>
                          updateConfig("dynamic_background", event.target.checked)
                        }
                      />
                    }
                    label={t("config:dynamicBackground")}
                  />
                  <FormControlLabel
                    control={
                      <Switch
                        checked={Boolean(config.fixed_background)}
                        onChange={(event) =>
                          updateConfig("fixed_background", event.target.checked)
                        }
                      />
                    }
                    label={t("config:fixedBackground")}
                  />
                  <FormControlLabel
                    control={
                      <Switch
                        checked={Boolean(config.overlay_enabled)}
                        onChange={(event) =>
                          updateConfig("overlay_enabled", event.target.checked)
                        }
                      />
                    }
                    label={t("config:overlayEnabled")}
                  />
                </Stack>
                {renderColorPicker(
                  "background_color",
                  config.background_color,
                  t("config:backgroundColor")
                )}
                <BackgroundColorList
                  colors={backgroundColors}
                  onChange={(colors) => updateConfig("background_colors", colors)}
                />
              </Stack>
            )}

            {section(
              t("config:videoEncoding"),
              <Grid container spacing={2} sx={{ width: "100%", margin: 0 }}>
                <Grid item xs={12} md={6}>
                  <TextField
                    fullWidth
                    label={t("config:ffmpegPath")}
                    value={config.ffmpeg?.path || "ffmpeg"}
                    onChange={(event) => updateConfig("ffmpeg.path", event.target.value)}
                    InputProps={{
                      endAdornment: (
                        <InputAdornment position="end">
                          <Tooltip title={t("config:chooseFfmpegFile")}>
                            <IconButton edge="end" onClick={selectFfmpegPath}>
                              <FolderOpenIcon />
                            </IconButton>
                          </Tooltip>
                        </InputAdornment>
                      ),
                    }}
                  />
                </Grid>
                <Grid item xs={12} md={6}>
                  <FormControl fullWidth>
                    <InputLabel>{t("config:encoder")}</InputLabel>
                    <Select
                      value={config.ffmpeg?.encoder || "auto"}
                      label={t("config:encoder")}
                      onChange={(event) => {
                        const encoder = event.target.value;
                        updateConfig("ffmpeg.encoder", encoder);
                        if (encoder.startsWith("hevc")) setEncoderHevc(encoder);
                        else if (encoder !== "auto") setEncoderAvc(encoder);
                      }}
                    >
                      <MenuItem value="auto">auto</MenuItem>
                      <MenuItem value="h264_qsv">H.264 QSV</MenuItem>
                      <MenuItem value="h264_nvenc">H.264 NVENC</MenuItem>
                      <MenuItem value="h264_amf">H.264 AMF</MenuItem>
                      <MenuItem value="h264_vaapi">H.264 VAAPI</MenuItem>
                      <MenuItem value="hevc_qsv">HEVC QSV</MenuItem>
                      <MenuItem value="hevc_nvenc">HEVC NVENC</MenuItem>
                      <MenuItem value="hevc_amf">HEVC AMF</MenuItem>
                      <MenuItem value="hevc_vaapi">HEVC VAAPI</MenuItem>
                      <MenuItem value="libx264">libx264</MenuItem>
                      <MenuItem value="libx265">libx265</MenuItem>
                    </Select>
                  </FormControl>
                </Grid>
                <Grid item xs={12} sm={4}>
                  <TextField
                    fullWidth
                    type="number"
                    label={t("config:crf")}
                    value={config.ffmpeg?.crf ?? 18}
                    onChange={(event) =>
                      updateConfig("ffmpeg.crf", Math.max(0, parseInt(event.target.value) || 0))
                    }
                  />
                </Grid>
                <Grid item xs={12} sm={4}>
                  <TextField
                    fullWidth
                    label={t("config:preset")}
                    value={config.ffmpeg?.preset || "fast"}
                    onChange={(event) => updateConfig("ffmpeg.preset", event.target.value)}
                  />
                </Grid>
                <Grid item xs={12} sm={4}>
                  <TextField
                    fullWidth
                    label={t("config:pixelFormat")}
                    value={config.ffmpeg?.pixel_format || "yuv420p"}
                    onChange={(event) =>
                      updateConfig("ffmpeg.pixel_format", event.target.value)
                    }
                  />
                </Grid>
              </Grid>
            )}

        </Stack>
      </Stack>
    </Paper>
  );
};

export default ConfigEditor;
