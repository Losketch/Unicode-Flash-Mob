import React, { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useSnackbar } from "notistack";
import {
  Alert,
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
import SceneComponentsEditor from "./SceneComponentsEditor";
import ContentTemplatesEditor from "./ContentTemplatesEditor";
import { useSettings } from "../contexts/SettingsContext";
import { setNestedValue } from "../utils/config";
import type {
  Color,
  RenderConfig,
  RenderConfigPath,
} from "../types/config";
import { formatError } from "../utils/errors";

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
      showAlert(`${t("config:loadFailed")}: ${formatError(error)}`, "error");
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
      showAlert(`${t("config:saveFailed")}: ${formatError(error)}`, "error");
    }
  };

  const handleGetDefault = async () => {
    try {
      const defaultConfig = await invoke<RenderConfig>("get_default_config");
      onConfigChange(defaultConfig);
      showAlert(t("config:defaultLoaded"), "success");
    } catch (error) {
      showAlert(`${t("config:getDefaultFailed")}: ${formatError(error)}`, "error");
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
              t("config:sceneComponents"),
              <Stack spacing={3}>
                <SceneComponentsEditor
                  components={config.components ?? []}
                  onChange={(components) =>
                    onConfigChange({ ...config, components })
                  }
                />
                <Divider />
                <ContentTemplatesEditor
                  templates={config.content_templates ?? {}}
                  onChange={(content_templates) =>
                    onConfigChange({ ...config, content_templates })
                  }
                />
                <Divider />
                <Stack spacing={1}>
                  <Typography variant="subtitle2">
                    {t("config:legacyTextOffsets")}
                  </Typography>
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
              </Stack>,
              true
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
                <Grid item xs={12}>
                  <Divider sx={{ my: 1 }} />
                  <Stack spacing={1}>
                    <Typography variant="subtitle1" color="error.main" fontWeight={700}>
                      {t("config:encodingDangerZone")}
                    </Typography>
                    <Alert severity="warning">
                      {t("config:encodingDangerWarning")}
                    </Alert>
                  </Stack>
                </Grid>
                <Grid item xs={12} sm={4}>
                  <TextField
                    fullWidth
                    type="number"
                    label={t("config:parallelWorkers")}
                    value={config.ffmpeg?.parallel_workers ?? 0}
                    inputProps={{ min: 0, step: 1 }}
                    helperText={t("config:parallelWorkersHint")}
                    onChange={(event) =>
                      updateConfig(
                        "ffmpeg.parallel_workers",
                        Math.max(0, Number.parseInt(event.target.value, 10) || 0)
                      )
                    }
                  />
                </Grid>
                <Grid item xs={12} sm={4}>
                  <TextField
                    fullWidth
                    type="number"
                    label={t("config:maxInflightFrames")}
                    value={config.ffmpeg?.max_inflight_frames ?? 2}
                    inputProps={{ min: 1, step: 1 }}
                    helperText={t("config:maxInflightFramesHint")}
                    onChange={(event) =>
                      updateConfig(
                        "ffmpeg.max_inflight_frames",
                        Math.max(1, Number.parseInt(event.target.value, 10) || 1)
                      )
                    }
                  />
                </Grid>
                <Grid item xs={12} sm={4}>
                  <TextField
                    fullWidth
                    type="number"
                    label={t("config:encodingProcesses")}
                    value={config.ffmpeg?.encoding_processes ?? 1}
                    inputProps={{ min: 1, step: 1 }}
                    helperText={t("config:encodingProcessesHint")}
                    onChange={(event) =>
                      updateConfig(
                        "ffmpeg.encoding_processes",
                        Math.max(1, Number.parseInt(event.target.value, 10) || 1)
                      )
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
