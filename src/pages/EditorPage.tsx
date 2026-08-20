import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation } from "react-router-dom";
import { useSnackbar } from "notistack";
import {
  Button,
  Grid,
  IconButton,
  Paper,
  Stack,
  Switch,
  TextField,
  FormControlLabel,
  InputAdornment,
  List,
  ListItemButton,
  ListItemText,
  Chip,
  Tooltip,
  Typography,
} from "@mui/material";
import AutoFixHighIcon from "@mui/icons-material/AutoFixHigh";
import ChevronLeftIcon from "@mui/icons-material/ChevronLeft";
import ChevronRightIcon from "@mui/icons-material/ChevronRight";
import InfoOutlinedIcon from "@mui/icons-material/InfoOutlined";
import { invoke } from "@tauri-apps/api/core";
import ColorField from "../components/ColorField";
import ConfigEditor from "../components/ConfigEditor";
import FontList from "../components/FontList";
import FramePreviewCanvas from "../components/FramePreviewCanvas";
import PreviewPanel from "../components/PreviewPanel";
import { useWorkflow } from "../contexts/WorkflowContext";
import { mergeExtractedConfig } from "../utils/config";
import { getPrimaryGlyphComponent } from "../types/config";
import type {
  CharEntry,
  Color,
  GlyphComponent,
  RenderConfig,
} from "../types/config";

const EditorPage: React.FC = () => {
  const { t } = useTranslation("editor");
  const location = useLocation();
  const { enqueueSnackbar } = useSnackbar();
  const { config, setConfig } = useWorkflow();
  const initialConfig = (location.state as { config?: RenderConfig })?.config;
  const initializedRef = useRef(false);
  const requestedDefaultRef = useRef(false);
  const [entryIndex, setEntryIndex] = useState(0);
  const [extracting, setExtracting] = useState(false);

  useEffect(() => {
    if (initializedRef.current || !initialConfig) return;
    initializedRef.current = true;
    setConfig(initialConfig);
    window.history.replaceState({}, document.title);
  }, [initialConfig, setConfig]);

  useEffect(() => {
    if (config || initialConfig || requestedDefaultRef.current) return;
    requestedDefaultRef.current = true;
    invoke<RenderConfig>("get_default_config")
      .then(setConfig)
      .catch((reason) =>
        enqueueSnackbar(`${t("createDefaultFailed")}: ${reason}`, { variant: "error" })
      );
  }, [config, enqueueSnackbar, initialConfig, setConfig, t]);

  useEffect(() => {
    const length = config?.characters?.length ?? 0;
    if (length === 0 && entryIndex !== 0) setEntryIndex(0);
    else if (length > 0 && entryIndex >= length) setEntryIndex(length - 1);
  }, [config?.characters?.length, entryIndex]);

  const updateConfig = (next: RenderConfig) => setConfig(next);

  const updatePrimaryGlyph = (patch: Partial<GlyphComponent>) => {
    if (!config) return;
    let updated = false;
    const components = config.components.map((component) => {
      if (!updated && component.type === "glyph") {
        updated = true;
        return { ...component, ...patch };
      }
      return component;
    });
    updateConfig({ ...config, components });
  };

  const updateCurrentEntry = (patch: Partial<CharEntry>) => {
    if (!config?.characters?.[entryIndex]) return;
    const characters = [...config.characters];
    characters[entryIndex] = { ...characters[entryIndex], ...patch };
    updateConfig({ ...config, characters });
  };

  const updateCurrentCodePoint = (codePoint: string) => {
    updateCurrentEntry({ code_point: codePoint });
  };

  const setCurrentColor = (
    field: "background_color" | "text_color",
    color: Color
  ) => {
    updateCurrentEntry({ [field]: color });
  };

  const handleExtract = async () => {
    if (!config) return;
    const fonts = getPrimaryGlyphComponent(config)?.fonts ?? [];
    if (!fonts.length) {
      enqueueSnackbar(t("addFontFirst"), { variant: "error" });
      return;
    }
    setExtracting(true);
    try {
      const extracted = await invoke<RenderConfig>("extract_characters", {
        fontFiles: fonts,
        outputPath: "unicode_config.json",
        videoOutput: config.output_path || null,
      });
      updateConfig(mergeExtractedConfig(extracted, config));
      setEntryIndex(0);
      enqueueSnackbar(t("extractionSuccess"), { variant: "success" });
    } catch (reason) {
      enqueueSnackbar(`${t("extractionFailed")}: ${reason}`, { variant: "error" });
    } finally {
      setExtracting(false);
    }
  };

  const entry = config?.characters?.[entryIndex];
  const charactersCount = config?.characters?.length ?? 0;
  const primaryGlyph = getPrimaryGlyphComponent(config);

  return (
    <Stack spacing={3}>
      <Grid container sx={{ width: "100%", margin: 0 }}>
        <Grid item xs={12} md={7}>
          <FramePreviewCanvas config={config} entryIndex={entryIndex} />
        </Grid>
        <Grid
          item
          xs={12}
          md={5}
          sx={{ paddingLeft: { xs: 0, md: 3 }, paddingTop: { xs: 3, md: 0 } }}
        >
          <Stack spacing={3}>
            <Paper elevation={2} sx={{ p: 2.5, borderRadius: 3 }}>
              <Typography variant="h6" gutterBottom>
                {t("previewMainFont")}
              </Typography>
              <Stack spacing={2}>
                <FontList
                  fonts={primaryGlyph?.fonts ?? []}
                  onChange={(fonts) => updatePrimaryGlyph({ fonts })}
                  label={t("mainFonts")}
                  multiple
                />
                <Button
                  variant="outlined"
                  startIcon={<AutoFixHighIcon />}
                  onClick={handleExtract}
                  disabled={extracting || !primaryGlyph?.fonts?.length}
                >
                  {extracting ? t("extracting") : t("extractFromFonts")}
                </Button>
                <TextField
                  fullWidth
                  type="number"
                  label={t("mainFontSize")}
                  value={primaryGlyph?.font.size ?? 512}
                  onChange={(event) =>
                    primaryGlyph &&
                    updatePrimaryGlyph({
                      font: {
                        ...primaryGlyph.font,
                        size: Number(event.target.value) || 1,
                      },
                    })
                  }
                />
                <Stack spacing={0.5}>
                  <Typography variant="subtitle2">
                    {t("characterList")} ({charactersCount})
                  </Typography>
                  <List
                    dense
                    sx={{ maxHeight: 220, overflow: "auto", bgcolor: "action.hover", borderRadius: 2 }}
                  >
                    {(config?.characters ?? []).slice(0, 20).map((character, index) => (
                      <ListItemButton
                        key={`${character.code_point}-${index}`}
                        selected={index === entryIndex}
                        onClick={() => setEntryIndex(index)}
                      >
                        <Chip label={character.code_point} size="small" sx={{ mr: 1 }} />
                        <ListItemText
                          primary={character.description?.split("\n")[0] || ""}
                          secondary={character.description?.split("\n")[1] || ""}
                        />
                      </ListItemButton>
                    ))}
                  </List>
                </Stack>
                <Stack direction="row" spacing={0.5} alignItems="center">
                  <Tooltip title={t("previous")}>
                    <span>
                      <IconButton onClick={() => setEntryIndex(Math.max(0, entryIndex - 1))} disabled={entryIndex === 0}>
                        <ChevronLeftIcon />
                      </IconButton>
                    </span>
                  </Tooltip>
                  <TextField
                    fullWidth
                    label={`${t("currentFrame")} (${charactersCount ? `${entryIndex + 1} / ${charactersCount}` : t("noCharacters")})`}
                    value={entry?.code_point ?? ""}
                    onChange={(event) => updateCurrentCodePoint(event.target.value)}
                    InputProps={{
                      endAdornment: (
                        <InputAdornment position="end">
                          <Tooltip title={t("expressionHint")} arrow>
                            <IconButton size="small" edge="end" aria-label={t("expressionHint")}>
                              <InfoOutlinedIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                        </InputAdornment>
                      ),
                    }}
                    disabled={!entry}
                  />
                  <Tooltip title={t("next")}>
                    <span>
                      <IconButton onClick={() => setEntryIndex(Math.min(charactersCount - 1, entryIndex + 1))} disabled={!charactersCount || entryIndex >= charactersCount - 1}>
                        <ChevronRightIcon />
                      </IconButton>
                    </span>
                  </Tooltip>
                </Stack>
                <TextField
                  fullWidth
                  multiline
                  minRows={2}
                  label={t("characterDescription")}
                  value={entry?.description ?? ""}
                  onChange={(event) => updateCurrentEntry({ description: event.target.value })}
                  disabled={!entry}
                />
                <TextField
                  fullWidth
                  type="number"
                  label={t("durationFrames")}
                  value={entry?.duration_frames ?? ""}
                  onChange={(event) =>
                    updateCurrentEntry({
                      duration_frames: event.target.value
                        ? Math.max(1, parseInt(event.target.value) || 1)
                        : null,
                    })
                  }
                  disabled={!entry}
                />
                <Stack direction={{ xs: "column", sm: "row" }} spacing={1}>
                  <FormControlLabel
                    control={
                      <Switch
                        checked={Boolean(entry?.background_color)}
                        onChange={(event) =>
                          updateCurrentEntry({
                            background_color: event.target.checked
                              ? { r: 0, g: 0, b: 0, a: 255 }
                              : null,
                          })
                        }
                        disabled={!entry}
                      />
                    }
                    label={t("overrideBackgroundColor")}
                  />
                  <FormControlLabel
                    control={
                      <Switch
                        checked={Boolean(entry?.text_color)}
                        onChange={(event) =>
                          updateCurrentEntry({
                            text_color: event.target.checked
                              ? { r: 0, g: 0, b: 0, a: 255 }
                              : null,
                          })
                        }
                        disabled={!entry}
                      />
                    }
                    label={t("overrideTextColor")}
                  />
                </Stack>
                {(["background_color", "text_color"] as const).map((field) => {
                  const color = entry?.[field];
                  if (!color) return null;
                  return (
                    <ColorField
                      key={field}
                      label={
                        field === "background_color"
                          ? t("overrideBackgroundColor")
                          : t("overrideTextColor")
                      }
                      value={color}
                      onChange={(next) => setCurrentColor(field, next)}
                    />
                  );
                })}
                <FormControlLabel
                  control={
                    <Switch
                      checked={Boolean(entry?.position)}
                      onChange={(event) =>
                        updateCurrentEntry({
                          position: event.target.checked ? { x: 0.5, y: 0.5 } : null,
                        })
                      }
                      disabled={!entry}
                    />
                  }
                  label={t("overridePosition")}
                />
                {entry?.position && (
                  <Stack direction="row" spacing={2}>
                    <TextField
                      fullWidth
                      type="number"
                      label={t("positionX")}
                      value={entry.position.x}
                      onChange={(event) =>
                        updateCurrentEntry({
                          position: {
                            x: Number(event.target.value),
                            y: entry.position?.y ?? 0.5,
                          },
                        })
                      }
                    />
                    <TextField
                      fullWidth
                      type="number"
                      label={t("positionY")}
                      value={entry.position.y}
                      onChange={(event) =>
                        updateCurrentEntry({
                          position: {
                            x: entry.position?.x ?? 0.5,
                            y: Number(event.target.value),
                          },
                        })
                      }
                    />
                  </Stack>
                )}
              </Stack>
            </Paper>

          </Stack>
        </Grid>
      </Grid>

      <ConfigEditor config={config} onConfigChange={updateConfig} />
      <PreviewPanel config={config} />
    </Stack>
  );
};

export default EditorPage;
