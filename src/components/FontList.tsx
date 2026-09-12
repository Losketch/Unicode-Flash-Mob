import React from "react";
import { useTranslation } from "react-i18next";
import {
  Box,
  Button,
  IconButton,
  List,
  ListItem,
  ListItemText,
  Stack,
  SxProps,
  TextField,
  Tooltip,
  Theme,
  Typography,
} from "@mui/material";
import DeleteIcon from "@mui/icons-material/Delete";
import UploadFileIcon from "@mui/icons-material/UploadFile";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import DragIndicatorIcon from "@mui/icons-material/DragIndicator";
import {
  DragDropContext,
  Droppable,
  Draggable,
  type DropResult,
} from "@hello-pangea/dnd";
import { open } from "@tauri-apps/plugin-dialog";
import { moveItem } from "../utils/config";
import type { FontSource } from "../types/config";

interface FontListProps {
  fonts: FontSource[];
  onChange: (fonts: FontSource[]) => void;
  label?: string;
  multiple?: boolean;
  disabled?: boolean;
  sx?: SxProps<Theme>;
}

const FontList: React.FC<FontListProps> = ({
  fonts,
  onChange,
  label,
  multiple = true,
  disabled = false,
  sx,
}) => {
  const { t } = useTranslation(["common", "extract"]);
  const fontPath = (font: FontSource) =>
    typeof font === "string" ? font : font.path;
  const faceIndex = (font: FontSource) =>
    typeof font === "string" ? 0 : font.face_index ?? 0;
  const displayName = (font: FontSource) => {
    const path = fontPath(font);
    return path.split(/[\\/]/).pop() || path;
  };
  const isCollection = (font: FontSource) =>
    /\.(?:ttc|otc)$/i.test(fontPath(font)) || typeof font !== "string";
  const withFaceIndex = (font: FontSource, value: number): FontSource => {
    const index = Math.max(0, Math.trunc(Number.isFinite(value) ? value : 0));
    return index === 0 ? fontPath(font) : { path: fontPath(font), face_index: index };
  };

  const handleAdd = async () => {
    try {
      const path = await open({
        multiple,
        filters: [
          {
            name: t("extract:fontFiles"),
            extensions: ["ttf", "otf", "ttc", "otc"],
          },
        ],
      });
      if (path) {
        const added = Array.isArray(path) ? path : [path];
        const next = multiple ? [...fonts, ...added] : [added[0]];
        onChange(next);
      }
    } catch {
    }
  };

  const handleRemove = (index: number) => {
    onChange(fonts.filter((_, i) => i !== index));
  };

  const handleMoveUp = (index: number) => {
    onChange(moveItem(fonts, index, index - 1));
  };

  const handleMoveDown = (index: number) => {
    onChange(moveItem(fonts, index, index + 1));
  };

  const handleFaceIndexChange = (index: number, value: number) => {
    const next = [...fonts];
    next[index] = withFaceIndex(next[index], value);
    onChange(next);
  };

  const handleDragEnd = (result: DropResult) => {
    if (!result.destination) return;
    const next = Array.from(fonts);
    const [removed] = next.splice(result.source.index, 1);
    next.splice(result.destination.index, 0, removed);
    onChange(next);
  };

  return (
    <Box sx={sx}>
      {label && (
        <Typography variant="subtitle2" gutterBottom>
          {label}
        </Typography>
      )}
      <Button
        variant="outlined"
        startIcon={<UploadFileIcon />}
        onClick={handleAdd}
        disabled={disabled}
        size="small"
        sx={{ mb: 1 }}
      >
        {multiple ? t("extract:addFonts") : t("extract:addFont")}
      </Button>

      {fonts.length > 0 && (
        <DragDropContext onDragEnd={handleDragEnd}>
          <Droppable droppableId={`font-list-${label ?? "default"}`}>
            {(provided) => (
              <List
                {...provided.droppableProps}
                ref={provided.innerRef}
                sx={{ bgcolor: "action.hover", borderRadius: 2 }}
              >
                {fonts.map((font, index) => (
                  <Draggable
                    key={`${fontPath(font)}-${faceIndex(font)}-${index}`}
                    draggableId={`${fontPath(font)}-${faceIndex(font)}-${index}`}
                    index={index}
                  >
                    {(providedItem, snapshot) => (
                      <ListItem
                        ref={providedItem.innerRef}
                        {...providedItem.draggableProps}
                        sx={{
                          bgcolor: snapshot.isDragging
                            ? "action.selected"
                            : "transparent",
                          borderRadius: 1,
                          pr: 1,
                        }}
                      >
                        <Box
                          {...providedItem.dragHandleProps}
                          sx={{ mr: 1, color: "text.secondary" }}
                        >
                          <DragIndicatorIcon />
                        </Box>
                        <ListItemText
                          primary={displayName(font)}
                          secondary={fontPath(font)}
                          primaryTypographyProps={{
                            noWrap: true,
                            title: displayName(font),
                          }}
                          secondaryTypographyProps={{
                            noWrap: true,
                            title: fontPath(font),
                          }}
                        />
                        {isCollection(font) && (
                          <Tooltip title={t("extract:fontFaceIndexHint")} arrow>
                            <TextField
                              label={t("extract:fontFaceIndex")}
                              type="number"
                              value={faceIndex(font)}
                              onChange={(event) =>
                                handleFaceIndexChange(index, Number(event.target.value))
                              }
                              disabled={disabled}
                              size="small"
                              inputProps={{ min: 0, step: 1 }}
                              sx={{ width: 104, mr: 0.5 }}
                            />
                          </Tooltip>
                        )}
                        <Stack direction="row" spacing={0.5} sx={{ flexShrink: 0 }}>
                          <IconButton
                            onClick={() => handleMoveUp(index)}
                            disabled={index === 0}
                            size="small"
                          >
                            <ArrowUpwardIcon />
                          </IconButton>
                          <IconButton
                            onClick={() => handleMoveDown(index)}
                            disabled={index === fonts.length - 1}
                            size="small"
                          >
                            <ArrowDownwardIcon />
                          </IconButton>
                          <IconButton
                            onClick={() => handleRemove(index)}
                            size="small"
                            color="error"
                          >
                            <DeleteIcon />
                          </IconButton>
                        </Stack>
                      </ListItem>
                    )}
                  </Draggable>
                ))}
                {provided.placeholder}
              </List>
            )}
          </Droppable>
        </DragDropContext>
      )}
    </Box>
  );
};

export default FontList;
