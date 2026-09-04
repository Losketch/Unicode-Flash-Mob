import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
  Alert,
  Breadcrumbs,
  Chip,
  Divider,
  FormControl,
  FormControlLabel,
  Grid,
  IconButton,
  InputAdornment,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  Switch,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import FolderOpenIcon from "@mui/icons-material/FolderOpen";
import { open } from "@tauri-apps/plugin-dialog";
import ColorField from "./ColorField";
import FontList from "./FontList";
import JsonEditor from "./JsonEditorField";
import type {
  FontConfig,
  ImageComponent,
  ProgressBarComponent,
  SceneComponent,
  TextComponent,
} from "../types/config";

interface ComponentInspectorProps {
  component: SceneComponent;
  path: SceneComponent[];
  onChange: (component: SceneComponent) => void;
}

type TypographyComponent = Extract<SceneComponent, { type: "glyph" | "text" }>;

interface InspectorSectionProps {
  title: string;
  children: ReactNode;
}

const InspectorSection = ({ title, children }: InspectorSectionProps) => (
  <Stack spacing={1.5}>
    <Typography variant="overline" color="text.secondary" sx={{ letterSpacing: 0.7 }}>
      {title}
    </Typography>
    {children}
  </Stack>
);

const asNumberRecord = (value: unknown): Record<string, number> => {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    Object.values(value).some((entry) => typeof entry !== "number")
  ) {
    throw new TypeError("Expected a JSON object whose values are numbers");
  }
  return value as Record<string, number>;
};

const ComponentInspector = ({
  component,
  path,
  onChange,
}: ComponentInspectorProps) => {
  const { t } = useTranslation("config");

  const componentLabel = () => {
    switch (component.type) {
      case "glyph":
        return t("glyphComponent");
      case "text":
        return t("textComponent");
      case "image":
        return t("imageComponent");
      case "progress_bar":
        return t("progressBarComponent");
      case "group":
        return t("groupComponent");
    }
  };

  const updateFont = (current: TypographyComponent, patch: Partial<FontConfig>) => {
    onChange({ ...current, font: { ...current.font, ...patch } });
  };

  const selectImageSource = async (current: ImageComponent) => {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: t("imageFiles"),
          extensions: ["png", "jpg", "jpeg", "webp"],
        },
      ],
    });
    if (selected && !Array.isArray(selected)) {
      onChange({ ...current, source: selected });
    }
  };

  return (
    <Stack spacing={2.5}>
      <Stack spacing={1}>
        <Breadcrumbs separator="/" aria-label={t("componentBreadcrumb")}>
          <Typography variant="caption" color="text.secondary">
            {t("sceneRoot")}
          </Typography>
          {path.map((entry) => (
            <Typography
              key={entry.id}
              variant="caption"
              color={entry.id === component.id ? "text.primary" : "text.secondary"}
            >
              {entry.id || t("unnamedComponent")}
            </Typography>
          ))}
        </Breadcrumbs>
        <Stack direction="row" alignItems="center" spacing={1}>
          <Typography variant="h6" sx={{ minWidth: 0, overflowWrap: "anywhere" }}>
            {component.id || t("unnamedComponent")}
          </Typography>
          <Chip size="small" label={componentLabel()} />
        </Stack>
      </Stack>

      <Divider />

      <InspectorSection title={t("inspectorGeneral")}>
        <Grid container spacing={2}>
          <Grid item xs={12} sm={8}>
            <TextField
              fullWidth
              label={t("elementId")}
              value={component.id}
              onChange={(event) => onChange({ ...component, id: event.target.value })}
            />
          </Grid>
          <Grid item xs={12} sm={4}>
            <FormControlLabel
              control={
                <Switch
                  checked={component.enabled}
                  onChange={(event) =>
                    onChange({ ...component, enabled: event.target.checked })
                  }
                />
              }
              label={t("elementEnabled")}
            />
          </Grid>
        </Grid>
      </InspectorSection>

      {component.type !== "group" && (
        <>
          <Divider />
          <InspectorSection title={t("inspectorLayout")}>
            <Grid container spacing={2}>
              <Grid item xs={6}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("positionX")}
                  value={component.position.x}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      position: {
                        ...component.position,
                        x: Number(event.target.value),
                      },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("positionY")}
                  value={component.position.y}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      position: {
                        ...component.position,
                        y: Number(event.target.value),
                      },
                    })
                  }
                />
              </Grid>
            </Grid>
          </InspectorSection>
        </>
      )}

      {(component.type === "glyph" || component.type === "text") && (
        <>
          <Divider />
          <InspectorSection title={t("inspectorContent")}>
            <TextField
              fullWidth
              label={t("textContent")}
              value={component.content}
              helperText={
                component.type === "glyph"
                  ? t("glyphContentHint")
                  : t("textContentHint")
              }
              onChange={(event) =>
                onChange({ ...component, content: event.target.value })
              }
            />

            {component.type === "text" && (
              <Grid container spacing={2}>
                <Grid item xs={12} sm={4}>
                  <FormControl fullWidth>
                    <InputLabel>{t("textAlign")}</InputLabel>
                    <Select
                      label={t("textAlign")}
                      value={component.align}
                      onChange={(event) =>
                        onChange({
                          ...component,
                          align: event.target.value as TextComponent["align"],
                        })
                      }
                    >
                      <MenuItem value="left">{t("alignLeft")}</MenuItem>
                      <MenuItem value="center">{t("alignCenter")}</MenuItem>
                      <MenuItem value="right">{t("alignRight")}</MenuItem>
                    </Select>
                  </FormControl>
                </Grid>
                <Grid item xs={12} sm={4}>
                  <TextField
                    fullWidth
                    type="number"
                    label={t("maxWidth")}
                    value={component.max_width}
                    onChange={(event) =>
                      onChange({ ...component, max_width: Number(event.target.value) })
                    }
                  />
                </Grid>
                <Grid item xs={12} sm={4}>
                  <FormControlLabel
                    control={
                      <Switch
                        checked={component.wrap}
                        onChange={(event) =>
                          onChange({ ...component, wrap: event.target.checked })
                        }
                      />
                    }
                    label={t("textWrap")}
                  />
                </Grid>
              </Grid>
            )}

            {component.type === "glyph" && (
              <FormControlLabel
                control={
                  <Switch
                    checked={component.overlay_combining_mark}
                    onChange={(event) =>
                      onChange({
                        ...component,
                        overlay_combining_mark: event.target.checked,
                      })
                    }
                  />
                }
                label={t("overlayCombiningMark")}
              />
            )}
          </InspectorSection>

          <Divider />
          <InspectorSection title={t("inspectorAppearance")}>
            <ColorField
              label={t("textColor")}
              value={component.color}
              onChange={(color) => onChange({ ...component, color })}
            />
          </InspectorSection>

          <Divider />
          <InspectorSection title={t("inspectorTypography")}>
            <Grid container spacing={2}>
              <Grid item xs={12} sm={6}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("fontSize")}
                  value={component.font.size}
                  onChange={(event) =>
                    updateFont(component, {
                      size: Number(event.target.value) || 1,
                    })
                  }
                />
              </Grid>
              <Grid item xs={12} sm={6}>
                <FormControlLabel
                  control={
                    <Switch
                      checked={component.font.font_optical_sizing !== false}
                      onChange={(event) =>
                        updateFont(component, {
                          font_optical_sizing: event.target.checked,
                        })
                      }
                    />
                  }
                  label={t("fontOpticalSizing")}
                />
              </Grid>
            </Grid>

            <FontList
              fonts={component.fonts}
              onChange={(fonts) => onChange({ ...component, fonts })}
              label={t("componentFonts")}
              multiple
            />
            {component.fonts.length === 0 && (
              <Alert severity={component.enabled ? "error" : "warning"}>
                {t("componentMissingFont")}
              </Alert>
            )}

            <Grid container spacing={2}>
              <Grid item xs={12} lg={6}>
                <JsonEditor
                  label={t("fontFeatureSettings")}
                  value={component.font.font_feature_settings}
                  onCommit={(value) =>
                    updateFont(component, {
                      font_feature_settings: asNumberRecord(value),
                    })
                  }
                  invalidMessage={t("invalidJson")}
                  hint={t("fontFeatureHint")}
                />
              </Grid>
              <Grid item xs={12} lg={6}>
                <JsonEditor
                  label={t("fontVariationSettings")}
                  value={component.font.font_variation_settings}
                  onCommit={(value) =>
                    updateFont(component, {
                      font_variation_settings: asNumberRecord(value),
                    })
                  }
                  invalidMessage={t("invalidJson")}
                  hint={t("fontVariationHint")}
                />
              </Grid>
            </Grid>
          </InspectorSection>
        </>
      )}

      {component.type === "image" && (
        <>
          <Divider />
          <InspectorSection title={t("inspectorSource")}>
            <TextField
              fullWidth
              label={t("imageSource")}
              value={component.source}
              helperText={t("imageSourceHint")}
              onChange={(event) =>
                onChange({ ...component, source: event.target.value })
              }
              InputProps={{
                endAdornment: (
                  <InputAdornment position="end">
                    <Tooltip title={t("chooseImageFile")}>
                      <IconButton onClick={() => selectImageSource(component)}>
                        <FolderOpenIcon />
                      </IconButton>
                    </Tooltip>
                  </InputAdornment>
                ),
              }}
            />
            {!component.source && (
              <Alert severity={component.enabled ? "error" : "warning"}>
                {t("componentMissingImage")}
              </Alert>
            )}
          </InspectorSection>

          <Divider />
          <InspectorSection title={t("inspectorAppearance")}>
            <Grid container spacing={2}>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("componentWidth")}
                  value={component.size.width}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      size: { ...component.size, width: Number(event.target.value) },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("componentHeight")}
                  value={component.size.height}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      size: { ...component.size, height: Number(event.target.value) },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  inputProps={{ min: 0, max: 1, step: 0.05 }}
                  label={t("componentOpacity")}
                  value={component.opacity}
                  onChange={(event) =>
                    onChange({ ...component, opacity: Number(event.target.value) })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <FormControl fullWidth>
                  <InputLabel>{t("imageFit")}</InputLabel>
                  <Select
                    label={t("imageFit")}
                    value={component.fit}
                    onChange={(event) =>
                      onChange({
                        ...component,
                        fit: event.target.value as ImageComponent["fit"],
                      })
                    }
                  >
                    <MenuItem value="contain">{t("imageFitContain")}</MenuItem>
                    <MenuItem value="cover">{t("imageFitCover")}</MenuItem>
                    <MenuItem value="stretch">{t("imageFitStretch")}</MenuItem>
                  </Select>
                </FormControl>
              </Grid>
            </Grid>
          </InspectorSection>
        </>
      )}

      {component.type === "progress_bar" && (
        <>
          <Divider />
          <InspectorSection title={t("inspectorProgress")}>
            <Grid container spacing={2}>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("componentWidth")}
                  value={component.size.width}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      size: { ...component.size, width: Number(event.target.value) },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("componentHeight")}
                  value={component.size.height}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      size: { ...component.size, height: Number(event.target.value) },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  inputProps={{ min: 0, max: 1, step: 0.05 }}
                  label={t("progressValue")}
                  value={component.progress}
                  onChange={(event) =>
                    onChange({ ...component, progress: Number(event.target.value) })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <FormControl fullWidth>
                  <InputLabel>{t("progressDirection")}</InputLabel>
                  <Select
                    label={t("progressDirection")}
                    value={component.direction}
                    onChange={(event) =>
                      onChange({
                        ...component,
                        direction: event.target.value as ProgressBarComponent["direction"],
                      })
                    }
                  >
                    <MenuItem value="left_to_right">{t("leftToRight")}</MenuItem>
                    <MenuItem value="right_to_left">{t("rightToLeft")}</MenuItem>
                    <MenuItem value="top_to_bottom">{t("topToBottom")}</MenuItem>
                    <MenuItem value="bottom_to_top">{t("bottomToTop")}</MenuItem>
                  </Select>
                </FormControl>
              </Grid>
            </Grid>
          </InspectorSection>

          <Divider />
          <InspectorSection title={t("inspectorAppearance")}>
            <ColorField
              label={t("progressBackgroundColor")}
              value={component.background_color}
              onChange={(background_color) =>
                onChange({ ...component, background_color })
              }
            />
            <ColorField
              label={t("progressFillColor")}
              value={component.fill_color}
              onChange={(fill_color) => onChange({ ...component, fill_color })}
            />
            <FormControlLabel
              control={
                <Switch
                  checked={component.border != null}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      border: event.target.checked
                        ? { color: { r: 255, g: 255, b: 255, a: 255 }, width: 1 }
                        : null,
                    })
                  }
                />
              }
              label={t("progressBorderEnabled")}
            />
            {component.border && (
              <Grid container spacing={2}>
                <Grid item xs={12} sm={8}>
                  <ColorField
                    label={t("progressBorderColor")}
                    value={component.border.color}
                    onChange={(color) =>
                      onChange({
                        ...component,
                        border: component.border
                          ? { ...component.border, color }
                          : null,
                      })
                    }
                  />
                </Grid>
                <Grid item xs={12} sm={4}>
                  <TextField
                    fullWidth
                    type="number"
                    inputProps={{ min: 1, step: 1 }}
                    label={t("progressBorderWidth")}
                    value={component.border.width}
                    onChange={(event) =>
                      onChange({
                        ...component,
                        border: component.border
                          ? {
                              ...component.border,
                              width: Math.max(
                                1,
                                Math.trunc(Number(event.target.value) || 1)
                              ),
                            }
                          : null,
                      })
                    }
                  />
                </Grid>
              </Grid>
            )}
          </InspectorSection>
        </>
      )}

      {component.type === "group" && (
        <>
          <Divider />
          <InspectorSection title={t("inspectorTransform")}>
            <Grid container spacing={2}>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("transformTranslationX")}
                  value={component.transform.translation.x}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      transform: {
                        ...component.transform,
                        translation: {
                          ...component.transform.translation,
                          x: Number(event.target.value),
                        },
                      },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("transformTranslationY")}
                  value={component.transform.translation.y}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      transform: {
                        ...component.transform,
                        translation: {
                          ...component.transform.translation,
                          y: Number(event.target.value),
                        },
                      },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("transformScaleX")}
                  value={component.transform.scale.x}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      transform: {
                        ...component.transform,
                        scale: {
                          ...component.transform.scale,
                          x: Number(event.target.value),
                        },
                      },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("transformScaleY")}
                  value={component.transform.scale.y}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      transform: {
                        ...component.transform,
                        scale: {
                          ...component.transform.scale,
                          y: Number(event.target.value),
                        },
                      },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("transformRotation")}
                  value={component.transform.rotation}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      transform: {
                        ...component.transform,
                        rotation: Number(event.target.value),
                      },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  inputProps={{ min: 0, max: 1, step: 0.05 }}
                  label={t("componentOpacity")}
                  value={component.opacity}
                  onChange={(event) =>
                    onChange({ ...component, opacity: Number(event.target.value) })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("transformAnchorX")}
                  value={component.transform.anchor.x}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      transform: {
                        ...component.transform,
                        anchor: {
                          ...component.transform.anchor,
                          x: Number(event.target.value),
                        },
                      },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} sm={3}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("transformAnchorY")}
                  value={component.transform.anchor.y}
                  onChange={(event) =>
                    onChange({
                      ...component,
                      transform: {
                        ...component.transform,
                        anchor: {
                          ...component.transform.anchor,
                          y: Number(event.target.value),
                        },
                      },
                    })
                  }
                />
              </Grid>
            </Grid>
          </InspectorSection>

          <Divider />
          <InspectorSection title={t("groupChildren")}>
            <Typography variant="body2">
              {t("childComponentCount", { count: component.children.length })}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              {t("manageGroupChildrenHint")}
            </Typography>
          </InspectorSection>
        </>
      )}
    </Stack>
  );
};

export default ComponentInspector;
