import React from "react";
import { useTranslation } from "react-i18next";
import { Stack, TextField, type SxProps, type Theme } from "@mui/material";
import type { Color } from "../types/config";
import { clampByte, colorToHex, hexToRgb } from "../utils/config";

interface ColorFieldProps {
  label: string;
  value: Color;
  onChange: (color: Color) => void;
  alphaLabel?: string;
  sx?: SxProps<Theme>;
}

const ColorField: React.FC<ColorFieldProps> = ({
  label,
  value,
  onChange,
  alphaLabel,
  sx,
}) => {
  const { t } = useTranslation("config");
  const hex = colorToHex(value);

  return (
    <Stack direction="row" spacing={1.5} alignItems="center" sx={sx}>
      <TextField
        type="color"
        label={label}
        value={hex}
        onChange={(event) => {
          const rgb = hexToRgb(event.target.value);
          if (!rgb) return;
          onChange({ ...value, ...rgb });
        }}
        InputLabelProps={{ shrink: true }}
        sx={{ flex: 1 }}
      />
      <TextField
        type="number"
        label={alphaLabel ?? t("alpha")}
        value={value.a ?? 255}
        inputProps={{ min: 0, max: 255 }}
        onChange={(event) =>
          onChange({ ...value, a: clampByte(parseInt(event.target.value) || 0) })
        }
        sx={{ width: 110 }}
      />
    </Stack>
  );
};

export default ColorField;
