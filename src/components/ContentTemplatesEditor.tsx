import { useTranslation } from "react-i18next";
import {
  Button,
  Chip,
  IconButton,
  InputAdornment,
  Paper,
  Stack,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import FolderOpenIcon from "@mui/icons-material/FolderOpen";
import { open } from "@tauri-apps/plugin-dialog";
import type { ContentTemplate } from "../types/config";

interface ContentTemplatesEditorProps {
  templates: Record<string, ContentTemplate>;
  onChange: (templates: Record<string, ContentTemplate>) => void;
}

const nextTemplateName = (
  templates: Record<string, ContentTemplate>,
  prefix: string
): string => {
  let index = 1;
  while (`${prefix}_${index}` in templates) index += 1;
  return `${prefix}_${index}`;
};

const ContentTemplatesEditor = ({
  templates,
  onChange,
}: ContentTemplatesEditorProps) => {
  const { t } = useTranslation("config");
  const entries = Object.entries(templates);

  const addTemplate = (type: ContentTemplate["type"]) => {
    const name = nextTemplateName(templates, type === "text" ? "text" : "external");
    const template: ContentTemplate =
      type === "text"
        ? { type: "text", value: "{char}" }
        : { type: "external", executable: "", args: ["{code}"] };
    onChange({ ...templates, [name]: template });
  };

  const renameTemplate = (oldName: string, newName: string) => {
    const normalized = newName.trim();
    if (!normalized || normalized === oldName || normalized in templates) return;
    const next: Record<string, ContentTemplate> = {};
    for (const [name, template] of Object.entries(templates)) {
      next[name === oldName ? normalized : name] = template;
    }
    onChange(next);
  };

  const updateTemplate = (name: string, template: ContentTemplate) => {
    onChange({ ...templates, [name]: template });
  };

  const removeTemplate = (name: string) => {
    const next = { ...templates };
    delete next[name];
    onChange(next);
  };

  const chooseExecutable = async (name: string, template: ContentTemplate) => {
    if (template.type !== "external") return;
    const path = await open({ multiple: false });
    if (typeof path === "string") {
      updateTemplate(name, { ...template, executable: path });
    }
  };

  return (
    <Stack spacing={2}>
      <Stack
        direction={{ xs: "column", sm: "row" }}
        spacing={1}
        alignItems={{ xs: "stretch", sm: "center" }}
        justifyContent="space-between"
      >
        <Stack spacing={0.25}>
          <Typography variant="subtitle1">{t("contentTemplates")}</Typography>
          <Typography variant="body2" color="text.secondary">
            {t("contentTemplatesHint")}
          </Typography>
        </Stack>
        <Stack direction="row" spacing={1}>
          <Button
            size="small"
            variant="outlined"
            startIcon={<AddIcon />}
            onClick={() => addTemplate("text")}
          >
            {t("addTextTemplate")}
          </Button>
          <Button
            size="small"
            variant="outlined"
            startIcon={<AddIcon />}
            onClick={() => addTemplate("external")}
          >
            {t("addExternalTemplate")}
          </Button>
        </Stack>
      </Stack>

      {entries.length === 0 && (
        <Typography variant="body2" color="text.secondary">
          {t("noContentTemplates")}
        </Typography>
      )}

      {entries.map(([name, template]) => (
        <Paper key={name} variant="outlined" sx={{ p: 2, borderRadius: 2 }}>
          <Stack spacing={2}>
            <Stack direction="row" spacing={1} alignItems="center">
              <Chip
                size="small"
                label={
                  template.type === "text"
                    ? t("textTemplate")
                    : t("externalTemplate")
                }
              />
              <TextField
                size="small"
                label={t("templateName")}
                defaultValue={name}
                onBlur={(event) => renameTemplate(name, event.target.value)}
                sx={{ flexGrow: 1 }}
              />
              <Tooltip title={t("removeTemplate")}>
                <IconButton size="small" onClick={() => removeTemplate(name)}>
                  <DeleteOutlineIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            </Stack>

            {template.type === "text" ? (
              <TextField
                fullWidth
                multiline
                minRows={2}
                label={t("templateValue")}
                value={template.value}
                helperText={t("templateReferenceHint")}
                onChange={(event) =>
                  updateTemplate(name, { ...template, value: event.target.value })
                }
              />
            ) : (
              <>
                <TextField
                  fullWidth
                  label={t("templateExecutable")}
                  value={template.executable}
                  onChange={(event) =>
                    updateTemplate(name, {
                      ...template,
                      executable: event.target.value,
                    })
                  }
                  InputProps={{
                    endAdornment: (
                      <InputAdornment position="end">
                        <Tooltip title={t("chooseTemplateExecutable")}>
                          <IconButton
                            edge="end"
                            onClick={() => chooseExecutable(name, template)}
                          >
                            <FolderOpenIcon />
                          </IconButton>
                        </Tooltip>
                      </InputAdornment>
                    ),
                  }}
                />
                <TextField
                  fullWidth
                  multiline
                  minRows={2}
                  label={t("templateArguments")}
                  value={template.args.join("\n")}
                  helperText={t("templateArgumentsHint")}
                  onChange={(event) =>
                    updateTemplate(name, {
                      ...template,
                      args: event.target.value.split("\n"),
                    })
                  }
                />
              </>
            )}
          </Stack>
        </Paper>
      ))}
    </Stack>
  );
};

export default ContentTemplatesEditor;
