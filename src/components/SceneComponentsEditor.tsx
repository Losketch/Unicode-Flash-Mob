import { useTranslation } from "react-i18next";
import {
  Alert,
  Button,
  Chip,
  Divider,
  FormControl,
  FormControlLabel,
  Grid,
  IconButton,
  InputLabel,
  MenuItem,
  Paper,
  Select,
  Stack,
  Switch,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import ColorField from "./ColorField";
import FontList from "./FontList";
import JsonEditor from "./JsonEditorField";
import type {
  FontConfig,
  GlyphComponent,
  SceneComponent,
  TextComponent,
} from "../types/config";

interface SceneComponentsEditorProps {
  components: SceneComponent[];
  onChange: (components: SceneComponent[]) => void;
}

const defaultFont = (size: number): FontConfig => ({
  size,
  font_feature_settings: {},
  font_variation_settings: {},
  font_optical_sizing: true,
});

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

const nextId = (components: SceneComponent[], prefix: string): string => {
  const used = new Set(components.map((component) => component.id));
  let index = 1;
  while (used.has(`${prefix}_${index}`)) index += 1;
  return `${prefix}_${index}`;
};

const createGlyphComponent = (components: SceneComponent[]): GlyphComponent => ({
  type: "glyph",
  id: nextId(components, "glyph"),
  enabled: false,
  content: "{glyph}",
  position: { x: 0.5, y: 0.5 },
  color: { r: 0, g: 0, b: 0, a: 128 },
  fonts: [],
  font: defaultFont(512),
  overlay_combining_mark: true,
});

const createTextComponent = (components: SceneComponent[]): TextComponent => ({
  type: "text",
  id: nextId(components, "text"),
  enabled: false,
  content: "{char}",
  position: { x: 0.05, y: 0.1 },
  color: { r: 0, g: 0, b: 0, a: 192 },
  fonts: [],
  font: defaultFont(42),
  align: "left",
  wrap: true,
  max_width: 0.9,
});

const SceneComponentsEditor = ({
  components,
  onChange,
}: SceneComponentsEditorProps) => {
  const { t } = useTranslation("config");

  const updateAt = (index: number, component: SceneComponent) => {
    const next = [...components];
    next[index] = component;
    onChange(next);
  };

  const removeAt = (index: number) => {
    onChange(components.filter((_, current) => current !== index));
  };

  const move = (index: number, direction: -1 | 1) => {
    const target = index + direction;
    if (target < 0 || target >= components.length) return;
    const next = [...components];
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };

  const updateFont = (
    index: number,
    component: SceneComponent,
    patch: Partial<FontConfig>
  ) => updateAt(index, { ...component, font: { ...component.font, ...patch } });

  return (
    <Stack spacing={2}>
      <Stack
        direction={{ xs: "column", sm: "row" }}
        spacing={1}
        alignItems={{ xs: "stretch", sm: "center" }}
        justifyContent="space-between"
      >
        <Stack spacing={0.25}>
          <Typography variant="subtitle1">{t("sceneComponents")}</Typography>
          <Typography variant="body2" color="text.secondary">
            {t("componentOrderHint")}
          </Typography>
        </Stack>
        <Stack direction="row" spacing={1}>
          <Button
            size="small"
            variant="outlined"
            startIcon={<AddIcon />}
            onClick={() => onChange([...components, createGlyphComponent(components)])}
          >
            {t("addGlyphComponent")}
          </Button>
          <Button
            size="small"
            variant="outlined"
            startIcon={<AddIcon />}
            onClick={() => onChange([...components, createTextComponent(components)])}
          >
            {t("addTextComponent")}
          </Button>
        </Stack>
      </Stack>

      {components.map((component, index) => (
        <Paper key={index} variant="outlined" sx={{ p: 2, borderRadius: 2 }}>
          <Stack spacing={2}>
            <Stack direction="row" alignItems="center" spacing={1}>
              <Chip
                size="small"
                label={
                  component.type === "glyph"
                    ? t("glyphComponent")
                    : t("textComponent")
                }
              />
              <Typography variant="subtitle2" sx={{ flexGrow: 1 }}>
                {component.id || t("unnamedComponent")}
              </Typography>
              <Tooltip title={t("moveComponentUp")}>
                <span>
                  <IconButton size="small" disabled={index === 0} onClick={() => move(index, -1)}>
                    <ArrowUpwardIcon fontSize="small" />
                  </IconButton>
                </span>
              </Tooltip>
              <Tooltip title={t("moveComponentDown")}>
                <span>
                  <IconButton
                    size="small"
                    disabled={index === components.length - 1}
                    onClick={() => move(index, 1)}
                  >
                    <ArrowDownwardIcon fontSize="small" />
                  </IconButton>
                </span>
              </Tooltip>
              <Tooltip title={t("removeComponent")}>
                <IconButton size="small" onClick={() => removeAt(index)}>
                  <DeleteOutlineIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            </Stack>

            <Grid container spacing={2} sx={{ width: "100%", margin: 0 }}>
              <Grid item xs={12} md={4}>
                <TextField
                  fullWidth
                  label={t("elementId")}
                  value={component.id}
                  onChange={(event) =>
                    updateAt(index, { ...component, id: event.target.value })
                  }
                />
              </Grid>
              <Grid item xs={12} md={8}>
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
                    updateAt(index, { ...component, content: event.target.value })
                  }
                />
              </Grid>
              <Grid item xs={6} md={2}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("positionX")}
                  value={component.position.x}
                  onChange={(event) =>
                    updateAt(index, {
                      ...component,
                      position: { ...component.position, x: Number(event.target.value) },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} md={2}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("positionY")}
                  value={component.position.y}
                  onChange={(event) =>
                    updateAt(index, {
                      ...component,
                      position: { ...component.position, y: Number(event.target.value) },
                    })
                  }
                />
              </Grid>
              <Grid item xs={6} md={2}>
                <TextField
                  fullWidth
                  type="number"
                  label={t("fontSize")}
                  value={component.font.size}
                  onChange={(event) =>
                    updateFont(index, component, {
                      size: Number(event.target.value) || 1,
                    })
                  }
                />
              </Grid>
              {component.type === "text" && (
                <>
                  <Grid item xs={6} md={3}>
                    <FormControl fullWidth>
                      <InputLabel>{t("textAlign")}</InputLabel>
                      <Select
                        label={t("textAlign")}
                        value={component.align}
                        onChange={(event) =>
                          updateAt(index, {
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
                  <Grid item xs={6} md={3}>
                    <TextField
                      fullWidth
                      type="number"
                      label={t("maxWidth")}
                      value={component.max_width}
                      onChange={(event) =>
                        updateAt(index, {
                          ...component,
                          max_width: Number(event.target.value),
                        })
                      }
                    />
                  </Grid>
                </>
              )}
            </Grid>

            <Stack direction={{ xs: "column", sm: "row" }} spacing={1}>
              <FormControlLabel
                control={
                  <Switch
                    checked={component.enabled}
                    onChange={(event) =>
                      updateAt(index, { ...component, enabled: event.target.checked })
                    }
                  />
                }
                label={t("elementEnabled")}
              />
              {component.type === "text" ? (
                <FormControlLabel
                  control={
                    <Switch
                      checked={component.wrap}
                      onChange={(event) =>
                        updateAt(index, { ...component, wrap: event.target.checked })
                      }
                    />
                  }
                  label={t("textWrap")}
                />
              ) : (
                <FormControlLabel
                  control={
                    <Switch
                      checked={component.overlay_combining_mark}
                      onChange={(event) =>
                        updateAt(index, {
                          ...component,
                          overlay_combining_mark: event.target.checked,
                        })
                      }
                    />
                  }
                  label={t("overlayCombiningMark")}
                />
              )}
              <FormControlLabel
                control={
                  <Switch
                    checked={component.font.font_optical_sizing !== false}
                    onChange={(event) =>
                      updateFont(index, component, {
                        font_optical_sizing: event.target.checked,
                      })
                    }
                  />
                }
                label={t("fontOpticalSizing")}
              />
            </Stack>

            <ColorField
              label={t("textColor")}
              value={component.color}
              onChange={(color) => updateAt(index, { ...component, color })}
            />

            <FontList
              fonts={component.fonts}
              onChange={(fonts) => updateAt(index, { ...component, fonts })}
              label={t("componentFonts")}
              multiple
            />
            {component.fonts.length === 0 && (
              <Alert severity={component.enabled ? "error" : "warning"}>
                {t("componentMissingFont")}
              </Alert>
            )}

            <Divider />
            <Grid container spacing={2} sx={{ width: "100%", margin: 0 }}>
              <Grid item xs={12} md={6}>
                <JsonEditor
                  label={t("fontFeatureSettings")}
                  value={component.font.font_feature_settings}
                  onCommit={(value) =>
                    updateFont(index, component, {
                      font_feature_settings: asNumberRecord(value),
                    })
                  }
                  invalidMessage={t("invalidJson")}
                  hint={t("fontFeatureHint")}
                />
              </Grid>
              <Grid item xs={12} md={6}>
                <JsonEditor
                  label={t("fontVariationSettings")}
                  value={component.font.font_variation_settings}
                  onCommit={(value) =>
                    updateFont(index, component, {
                      font_variation_settings: asNumberRecord(value),
                    })
                  }
                  invalidMessage={t("invalidJson")}
                  hint={t("fontVariationHint")}
                />
              </Grid>
            </Grid>
          </Stack>
        </Paper>
      ))}
    </Stack>
  );
};

export default SceneComponentsEditor;
