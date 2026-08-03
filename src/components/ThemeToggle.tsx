import React from "react";
import { useTranslation } from "react-i18next";
import { IconButton, Menu, MenuItem, Tooltip } from "@mui/material";
import Brightness4Icon from "@mui/icons-material/Brightness4";
import Brightness7Icon from "@mui/icons-material/Brightness7";
import BrightnessAutoIcon from "@mui/icons-material/BrightnessAuto";
import { useSettings, type ThemeMode } from "../contexts/SettingsContext";

const ThemeToggle: React.FC = () => {
  const { t } = useTranslation("settings");
  const { settings, effectiveMode, setThemeMode } = useSettings();
  const [anchorEl, setAnchorEl] = React.useState<null | HTMLElement>(null);

  const handleOpen = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget);
  };

  const handleClose = () => {
    setAnchorEl(null);
  };

  const handleSelect = (mode: ThemeMode) => {
    setThemeMode(mode);
    handleClose();
  };

  const icon =
    settings.themeMode === "system" ? (
      <BrightnessAutoIcon />
    ) : effectiveMode === "dark" ? (
      <Brightness7Icon />
    ) : (
      <Brightness4Icon />
    );

  return (
    <>
      <Tooltip title={t("theme")}>
        <IconButton color="inherit" onClick={handleOpen}>
          {icon}
        </IconButton>
      </Tooltip>
      <Menu anchorEl={anchorEl} open={Boolean(anchorEl)} onClose={handleClose}>
        <MenuItem
          selected={settings.themeMode === "light"}
          onClick={() => handleSelect("light")}
        >
          {t("themeLight")}
        </MenuItem>
        <MenuItem
          selected={settings.themeMode === "dark"}
          onClick={() => handleSelect("dark")}
        >
          {t("themeDark")}
        </MenuItem>
        <MenuItem
          selected={settings.themeMode === "system"}
          onClick={() => handleSelect("system")}
        >
          {t("themeSystem")}
        </MenuItem>
      </Menu>
    </>
  );
};

export default ThemeToggle;
