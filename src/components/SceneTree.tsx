import { useTranslation } from "react-i18next";
import {
  Box,
  IconButton,
  List,
  ListItemButton,
  Stack,
  Switch,
  Tooltip,
  Typography,
} from "@mui/material";
import ChevronRightIcon from "@mui/icons-material/ChevronRight";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";
import FolderIcon from "@mui/icons-material/Folder";
import FontDownloadIcon from "@mui/icons-material/FontDownload";
import ImageIcon from "@mui/icons-material/Image";
import LinearScaleIcon from "@mui/icons-material/LinearScale";
import TextFieldsIcon from "@mui/icons-material/TextFields";
import type { SceneComponent } from "../types/config";

interface SceneTreeProps {
  components: SceneComponent[];
  selectedId: string | null;
  expandedIds: Set<string>;
  onSelect: (id: string) => void;
  onToggleExpanded: (id: string) => void;
  onToggleEnabled: (id: string, enabled: boolean) => void;
}

const componentIcon = (component: SceneComponent) => {
  switch (component.type) {
    case "glyph":
      return <FontDownloadIcon fontSize="small" />;
    case "text":
      return <TextFieldsIcon fontSize="small" />;
    case "image":
      return <ImageIcon fontSize="small" />;
    case "progress_bar":
      return <LinearScaleIcon fontSize="small" />;
    case "group":
      return <FolderIcon fontSize="small" />;
  }
};

const SceneTree = ({
  components,
  selectedId,
  expandedIds,
  onSelect,
  onToggleExpanded,
  onToggleEnabled,
}: SceneTreeProps) => {
  const { t } = useTranslation("config");

  const componentLabel = (component: SceneComponent) => {
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

  const renderNodes = (nodes: SceneComponent[], depth = 0) =>
    nodes.map((component) => {
      const isGroup = component.type === "group";
      const expanded = isGroup && expandedIds.has(component.id);

      return (
        <Box key={component.id}>
          <ListItemButton
            selected={selectedId === component.id}
            onClick={() => onSelect(component.id)}
            sx={{
              minHeight: 40,
              pl: 1 + depth * 2,
              pr: 0.5,
              borderRadius: 1,
              opacity: component.enabled ? 1 : 0.62,
            }}
          >
            <Box sx={{ width: 28, display: "flex", justifyContent: "center" }}>
              {isGroup && (
                <IconButton
                  size="small"
                  aria-label={expanded ? t("collapseGroup") : t("expandGroup")}
                  onClick={(event) => {
                    event.stopPropagation();
                    onToggleExpanded(component.id);
                  }}
                >
                  {expanded ? (
                    <ExpandMoreIcon fontSize="small" />
                  ) : (
                    <ChevronRightIcon fontSize="small" />
                  )}
                </IconButton>
              )}
            </Box>

            <Box sx={{ width: 24, display: "flex", justifyContent: "center" }}>
              {componentIcon(component)}
            </Box>

            <Stack spacing={0} sx={{ minWidth: 0, flexGrow: 1, ml: 0.5 }}>
              <Typography variant="body2" noWrap fontWeight={500}>
                {component.id || t("unnamedComponent")}
              </Typography>
              <Typography variant="caption" color="text.secondary" noWrap>
                {componentLabel(component)}
              </Typography>
            </Stack>

            <Tooltip title={component.enabled ? t("disableComponent") : t("enableComponent")}>
              <Switch
                size="small"
                checked={component.enabled}
                inputProps={{
                  "aria-label": component.enabled
                    ? t("disableComponent")
                    : t("enableComponent"),
                }}
                onClick={(event) => event.stopPropagation()}
                onChange={(event) =>
                  onToggleEnabled(component.id, event.target.checked)
                }
              />
            </Tooltip>
          </ListItemButton>

          {isGroup && expanded && component.children.length > 0 && (
            <Box>{renderNodes(component.children, depth + 1)}</Box>
          )}
        </Box>
      );
    });

  if (components.length === 0) {
    return (
      <Box sx={{ py: 5, px: 2, textAlign: "center" }}>
        <Typography variant="body2" color="text.secondary">
          {t("noSceneComponents")}
        </Typography>
      </Box>
    );
  }

  return <List disablePadding>{renderNodes(components)}</List>;
};

export default SceneTree;
