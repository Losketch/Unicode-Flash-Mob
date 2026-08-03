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

interface FontListProps {
  fonts: string[];
  onChange: (fonts: string[]) => void;
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
  const displayName = (path: string) => path.split(/[\\/]/).pop() || path;

  const handleAdd = async () => {
    try {
      const path = await open({
        multiple,
        filters: [
          {
            name: t("extract:fontFiles"),
            extensions: ["ttf", "otf"],
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
                  <Draggable key={`${font}-${index}`} draggableId={`${font}-${index}`} index={index}>
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
                          secondary={font}
                          primaryTypographyProps={{
                            noWrap: true,
                            title: displayName(font),
                          }}
                          secondaryTypographyProps={{
                            noWrap: true,
                            title: font,
                          }}
                        />
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
