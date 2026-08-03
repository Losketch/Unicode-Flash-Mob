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
  TextField,
  Typography,
} from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import DragIndicatorIcon from "@mui/icons-material/DragIndicator";
import {
  DragDropContext,
  Draggable,
  Droppable,
  type DropResult,
} from "@hello-pangea/dnd";
import {
  clampByte,
  colorToHex,
  hexToRgb,
  moveItem,
  type RgbaColor,
} from "../utils/config";

interface BackgroundColorListProps {
  colors: RgbaColor[];
  onChange: (colors: RgbaColor[]) => void;
}

const BackgroundColorList: React.FC<BackgroundColorListProps> = ({
  colors,
  onChange,
}) => {
  const { t } = useTranslation("config");

  const updateColor = (index: number, patch: Partial<RgbaColor>) => {
    onChange(
      colors.map((color, colorIndex) =>
        colorIndex === index ? { ...color, ...patch } : color
      )
    );
  };

  const moveColor = (from: number, to: number) => {
    if (to < 0 || to >= colors.length) return;
    onChange(moveItem(colors, from, to));
  };

  const handleDragEnd = (result: DropResult) => {
    if (result.destination) moveColor(result.source.index, result.destination.index);
  };

  return (
    <Stack spacing={1.25}>
      <Typography variant="subtitle2">{t("backgroundColors")}</Typography>
      <DragDropContext onDragEnd={handleDragEnd}>
        <Droppable droppableId="background-color-list">
          {(provided) => (
            <List
              {...provided.droppableProps}
              ref={provided.innerRef}
              sx={{ bgcolor: "action.hover", borderRadius: 2 }}
            >
              {colors.map((color, index) => {
                const hex = colorToHex(color);
                return (
                  <Draggable
                    key={`${hex}-${index}`}
                    draggableId={`background-color-${index}`}
                    index={index}
                  >
                    {(item, snapshot) => (
                      <ListItem
                        ref={item.innerRef}
                        {...item.draggableProps}
                        sx={{
                          bgcolor: snapshot.isDragging ? "action.selected" : "transparent",
                          borderRadius: 1,
                          pr: 1,
                        }}
                      >
                        <Box
                          {...item.dragHandleProps}
                          sx={{ mr: 1, color: "text.secondary", display: "flex" }}
                        >
                          <DragIndicatorIcon />
                        </Box>
                        <Box
                          component="input"
                          type="color"
                          value={hex}
                          aria-label={`${t("backgroundColor")} ${index + 1}`}
                          onChange={(event) => {
                            const rgb = hexToRgb(event.target.value);
                            if (rgb) updateColor(index, rgb);
                          }}
                          sx={{
                            width: 36,
                            height: 30,
                            p: 0,
                            border: 0,
                            bgcolor: "transparent",
                          }}
                        />
                        <ListItemText
                          primary={hex.toUpperCase()}
                          secondary={`rgba(${color.r}, ${color.g}, ${color.b}, ${color.a ?? 255})`}
                          sx={{ ml: 1 }}
                        />
                        <TextField
                          type="number"
                          size="small"
                          label={t("alpha")}
                          value={color.a ?? 255}
                          inputProps={{ min: 0, max: 255 }}
                          onChange={(event) =>
                            updateColor(index, {
                              a: clampByte(parseInt(event.target.value) || 0),
                            })
                          }
                          sx={{ width: 105 }}
                        />
                        <Stack direction="row" spacing={0.5} sx={{ flexShrink: 0 }}>
                          <IconButton
                            size="small"
                            onClick={() => moveColor(index, index - 1)}
                            disabled={index === 0}
                            aria-label={t("moveColorUp")}
                          >
                            <ArrowUpwardIcon />
                          </IconButton>
                          <IconButton
                            size="small"
                            onClick={() => moveColor(index, index + 1)}
                            disabled={index === colors.length - 1}
                            aria-label={t("moveColorDown")}
                          >
                            <ArrowDownwardIcon />
                          </IconButton>
                          <IconButton
                            size="small"
                            color="error"
                            onClick={() =>
                              onChange(
                                colors.filter((_, itemIndex) => itemIndex !== index)
                              )
                            }
                            aria-label={t("removeColor")}
                          >
                            <DeleteOutlineIcon />
                          </IconButton>
                        </Stack>
                      </ListItem>
                    )}
                  </Draggable>
                );
              })}
              {provided.placeholder}
            </List>
          )}
        </Droppable>
      </DragDropContext>
      <Button
        variant="outlined"
        size="small"
        startIcon={<AddIcon />}
        onClick={() => onChange([...colors, { r: 0, g: 0, b: 0, a: 255 }])}
      >
        {t("addColor")}
      </Button>
    </Stack>
  );
};

export default BackgroundColorList;
