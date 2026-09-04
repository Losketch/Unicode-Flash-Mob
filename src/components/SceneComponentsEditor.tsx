import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Box,
  Button,
  Grid,
  IconButton,
  Menu,
  MenuItem,
  Paper,
  Stack,
  Tooltip,
  Typography,
} from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import ComponentInspector from "./ComponentInspector";
import SceneTree from "./SceneTree";
import type { SceneComponent } from "../types/config";
import {
  appendComponent,
  collectGroupIds,
  createSceneComponent,
  findComponentById,
  findComponentPath,
  findParentId,
  findSiblingPosition,
  moveComponentById,
  removeComponentById,
  type SceneComponentType,
  updateComponentById,
} from "./sceneComponentUtils";

interface SceneComponentsEditorProps {
  components: SceneComponent[];
  onChange: (components: SceneComponent[]) => void;
}

const SceneComponentsEditor = ({
  components,
  onChange,
}: SceneComponentsEditorProps) => {
  const { t } = useTranslation("config");
  const [selectedId, setSelectedId] = useState<string | null>(
    () => components[0]?.id ?? null
  );
  const [expandedIds, setExpandedIds] = useState<Set<string>>(
    () => collectGroupIds(components)
  );
  const [addMenuAnchor, setAddMenuAnchor] = useState<HTMLElement | null>(null);

  const selectedComponent = useMemo(
    () => findComponentById(components, selectedId),
    [components, selectedId]
  );
  const selectedPath = useMemo(
    () =>
      selectedId !== null ? findComponentPath(components, selectedId) ?? [] : [],
    [components, selectedId]
  );
  const siblingPosition = useMemo(
    () =>
      selectedId !== null ? findSiblingPosition(components, selectedId) : undefined,
    [components, selectedId]
  );

  useEffect(() => {
    if (selectedId !== null && findComponentById(components, selectedId)) return;
    setSelectedId(components[0]?.id ?? null);
  }, [components, selectedId]);

  const toggleExpanded = (id: string) => {
    setExpandedIds((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const updateSelected = (next: SceneComponent) => {
    if (selectedId === null) return;
    const previousId = selectedId;
    onChange(updateComponentById(components, previousId, next));
    if (next.id !== previousId) {
      setSelectedId(next.id);
      setExpandedIds((current) => {
        if (!current.has(previousId)) return current;
        const updated = new Set(current);
        updated.delete(previousId);
        updated.add(next.id);
        return updated;
      });
    }
  };

  const toggleEnabled = (id: string, enabled: boolean) => {
    const component = findComponentById(components, id);
    if (!component) return;
    onChange(updateComponentById(components, id, { ...component, enabled }));
  };

  const addComponent = (type: SceneComponentType) => {
    const targetGroupId = selectedComponent?.type === "group" ? selectedComponent.id : null;
    const component = createSceneComponent(type, components);
    onChange(appendComponent(components, targetGroupId, component));
    setSelectedId(component.id);
    setExpandedIds((current) => {
      const next = new Set(current);
      if (targetGroupId) next.add(targetGroupId);
      if (component.type === "group") next.add(component.id);
      return next;
    });
    setAddMenuAnchor(null);
  };

  const moveSelected = (direction: -1 | 1) => {
    if (selectedId === null) return;
    onChange(moveComponentById(components, selectedId, direction));
  };

  const removeSelected = () => {
    if (selectedId === null) return;
    const parentId = findParentId(components, selectedId);
    const next = removeComponentById(components, selectedId);
    onChange(next);
    setSelectedId(
      typeof parentId === "string" ? parentId : next[0]?.id ?? null
    );
  };

  const addTargetLabel =
    selectedComponent?.type === "group"
      ? t("addTargetSelectedGroup", { id: selectedComponent.id })
      : t("addTargetSceneRoot");

  const canMoveUp = Boolean(siblingPosition && siblingPosition.index > 0);
  const canMoveDown = Boolean(
    siblingPosition && siblingPosition.index < siblingPosition.count - 1
  );

  return (
    <Stack spacing={2}>
      <Stack
        direction={{ xs: "column", md: "row" }}
        spacing={1}
        alignItems={{ xs: "stretch", md: "center" }}
        justifyContent="space-between"
      >
        <Stack spacing={0.25}>
          <Typography variant="body2" color="text.secondary">
            {t("componentOrderHint")}
          </Typography>
          <Typography variant="caption" color="text.secondary">
            {addTargetLabel}
          </Typography>
        </Stack>

        <Stack direction="row" spacing={0.5} alignItems="center">
          <Button
            size="small"
            variant="outlined"
            startIcon={<AddIcon />}
            onClick={(event) => setAddMenuAnchor(event.currentTarget)}
          >
            {t("addComponent")}
          </Button>
          <Menu
            anchorEl={addMenuAnchor}
            open={Boolean(addMenuAnchor)}
            onClose={() => setAddMenuAnchor(null)}
          >
            <MenuItem onClick={() => addComponent("glyph")}>
              {t("addGlyphComponent")}
            </MenuItem>
            <MenuItem onClick={() => addComponent("text")}>
              {t("addTextComponent")}
            </MenuItem>
            <MenuItem onClick={() => addComponent("image")}>
              {t("addImageComponent")}
            </MenuItem>
            <MenuItem onClick={() => addComponent("progress_bar")}>
              {t("addProgressBarComponent")}
            </MenuItem>
            <MenuItem onClick={() => addComponent("group")}>
              {t("addGroupComponent")}
            </MenuItem>
          </Menu>

          <Tooltip title={t("moveComponentUp")}>
            <span>
              <IconButton
                size="small"
                disabled={!canMoveUp}
                onClick={() => moveSelected(-1)}
              >
                <ArrowUpwardIcon fontSize="small" />
              </IconButton>
            </span>
          </Tooltip>
          <Tooltip title={t("moveComponentDown")}>
            <span>
              <IconButton
                size="small"
                disabled={!canMoveDown}
                onClick={() => moveSelected(1)}
              >
                <ArrowDownwardIcon fontSize="small" />
              </IconButton>
            </span>
          </Tooltip>
          <Tooltip title={t("removeComponent")}>
            <span>
              <IconButton
                size="small"
                color="error"
                disabled={!selectedComponent}
                onClick={removeSelected}
              >
                <DeleteOutlineIcon fontSize="small" />
              </IconButton>
            </span>
          </Tooltip>
        </Stack>
      </Stack>

      <Grid container spacing={2}>
        <Grid item xs={12} md={4} lg={3}>
          <Paper
            variant="outlined"
            sx={{
              height: { md: 620 },
              maxHeight: { xs: 360, md: 620 },
              overflow: "auto",
              p: 1,
              borderRadius: 2,
            }}
          >
            <Stack spacing={1}>
              <Typography variant="subtitle2" sx={{ px: 1, pt: 0.5 }}>
                {t("sceneTree")}
              </Typography>
              <SceneTree
                components={components}
                selectedId={selectedId}
                expandedIds={expandedIds}
                onSelect={setSelectedId}
                onToggleExpanded={toggleExpanded}
                onToggleEnabled={toggleEnabled}
              />
            </Stack>
          </Paper>
        </Grid>

        <Grid item xs={12} md={8} lg={9}>
          <Paper
            variant="outlined"
            sx={{
              minHeight: { xs: 320, md: 620 },
              p: { xs: 1.5, sm: 2 },
              borderRadius: 2,
            }}
          >
            {selectedComponent ? (
              <ComponentInspector
                component={selectedComponent}
                path={selectedPath}
                onChange={updateSelected}
              />
            ) : (
              <Box
                sx={{
                  minHeight: 280,
                  display: "grid",
                  placeItems: "center",
                  textAlign: "center",
                  px: 2,
                }}
              >
                <Typography variant="body2" color="text.secondary">
                  {t("noComponentSelected")}
                </Typography>
              </Box>
            )}
          </Paper>
        </Grid>
      </Grid>
    </Stack>
  );
};

export default SceneComponentsEditor;
