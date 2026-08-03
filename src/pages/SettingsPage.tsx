import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSnackbar } from "notistack";
import {
  Box,
  Button,
  Card,
  CardContent,
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
import CheckCircleIcon from "@mui/icons-material/CheckCircle";
import FolderOpenIcon from "@mui/icons-material/FolderOpen";
import AutoFixHighIcon from "@mui/icons-material/AutoFixHigh";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { SUPPORTED_LOCALES, LOCALE_LABELS } from "../i18n";
import { isSupportedLocale, isThemeMode } from "../config/settings";
import { useSettings } from "../contexts/SettingsContext";
import { isHexColor } from "../utils/config";

const PRESET_ACCENT_COLORS = [
  "#6750A4",
  "#2196F3",
  "#4CAF50",
  "#FF9800",
  "#F44336",
  "#E91E63",
  "#00BCD4",
  "#9C27B0",
];

const SettingsPage: React.FC = () => {
  const { t } = useTranslation(["settings", "common"]);
  const { enqueueSnackbar } = useSnackbar();
  const {
    settings,
    setLocale,
    setThemeMode,
    setAccentColor,
    setFollowSystem,
    resetSettings,
    followSystem,
    setOutputDir,
    setEncoderAvc,
    setEncoderHevc,
  } = useSettings();

  const [accentDraft, setAccentDraft] = useState(settings.accentColor);
  const [outputDirDraft, setOutputDirDraft] = useState(settings.outputDir);
  const [encoderAvcDraft, setEncoderAvcDraft] = useState(settings.encoderAvc);
  const [encoderHevcDraft, setEncoderHevcDraft] = useState(settings.encoderHevc);
  const [encoderLoading, setEncoderLoading] = useState<"avc" | "hevc" | null>(null);
  const draftRef = useRef(accentDraft);

  useEffect(() => {
    draftRef.current = accentDraft;
  }, [accentDraft]);

  useEffect(() => {
    setAccentDraft(settings.accentColor);
  }, [settings.accentColor]);

  useEffect(() => {
    setOutputDirDraft(settings.outputDir);
    setEncoderAvcDraft(settings.encoderAvc);
    setEncoderHevcDraft(settings.encoderHevc);
  }, [settings.outputDir, settings.encoderAvc, settings.encoderHevc]);

  const draftValid = isHexColor(accentDraft);
  const applied = accentDraft.trim().toLowerCase() === settings.accentColor.toLowerCase();

  const applyDraft = () => {
    const value = draftRef.current.trim();
    if (isHexColor(value)) {
      setAccentColor(value.startsWith("#") ? value : `#${value}`);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      applyDraft();
    }
  };

  const selectOutputDir = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") setOutputDirDraft(selected);
  };

  const getEncoder = async (hevc: boolean) => {
    setEncoderLoading(hevc ? "hevc" : "avc");
    try {
      const encoder = await invoke<string | null>("get_encoder", { hevc });
      if (encoder) {
        if (hevc) setEncoderHevcDraft(encoder);
        else setEncoderAvcDraft(encoder);
      }
      else enqueueSnackbar(t("encoderUnavailable"), { variant: "warning" });
    } catch (error) {
      enqueueSnackbar(`${t("encoderDetectFailed")}: ${error}`, { variant: "error" });
    } finally {
      setEncoderLoading(null);
    }
  };

  const testEncoder = async (encoder: string) => {
    if (!encoder.trim()) return;
    setEncoderLoading(encoder === encoderHevcDraft ? "hevc" : "avc");
    try {
      const available = await invoke<boolean>("test_encoder_command", { encoder });
      enqueueSnackbar(
        available ? t("encoderAvailable") : t("encoderUnavailable"),
        { variant: available ? "success" : "error" }
      );
    } catch (error) {
      enqueueSnackbar(`${t("encoderTestFailed")}: ${error}`, { variant: "error" });
    } finally {
      setEncoderLoading(null);
    }
  };

  const saveAdvancedSettings = () => {
    setOutputDir(outputDirDraft.trim());
    setEncoderAvc(encoderAvcDraft.trim());
    setEncoderHevc(encoderHevcDraft.trim());
  };

  return (
    <Stack spacing={3}>
      <Card elevation={2} sx={{ borderRadius: 3 }}>
        <CardContent sx={{ p: 3 }}>
          <Typography variant="h6" gutterBottom>
            {t("videoSettings")}
          </Typography>
          <Grid container spacing={2}>
            <Grid item xs={12}>
              <TextField
                fullWidth
                label={t("outputDirectory")}
                placeholder="./output/"
                value={outputDirDraft}
                onChange={(e) => setOutputDirDraft(e.target.value)}
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      <Tooltip title={t("chooseFolder")}>
                        <IconButton onClick={selectOutputDir} edge="end">
                          <FolderOpenIcon />
                        </IconButton>
                      </Tooltip>
                    </InputAdornment>
                  ),
                }}
              />
            </Grid>
            <Grid item xs={12} md={6}>
              <TextField
                fullWidth
                label={t("encoderAvc")}
                placeholder={t("encoderAuto")}
                value={encoderAvcDraft}
                onChange={(e) => setEncoderAvcDraft(e.target.value)}
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      <Tooltip title={t("encoderGetHint")}>
                        <IconButton
                          edge="end"
                          onClick={() => getEncoder(false)}
                          onContextMenu={(e) => {
                            e.preventDefault();
                            void testEncoder(encoderAvcDraft);
                          }}
                        >
                          <AutoFixHighIcon color={encoderLoading === "avc" ? "primary" : "inherit"} />
                        </IconButton>
                      </Tooltip>
                    </InputAdornment>
                  ),
                }}
              />
            </Grid>
            <Grid item xs={12} md={6}>
              <TextField
                fullWidth
                label={t("encoderHevc")}
                placeholder={t("encoderAuto")}
                value={encoderHevcDraft}
                onChange={(e) => setEncoderHevcDraft(e.target.value)}
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      <Tooltip title={t("encoderGetHint")}>
                        <IconButton
                          edge="end"
                          onClick={() => getEncoder(true)}
                          onContextMenu={(e) => {
                            e.preventDefault();
                            void testEncoder(encoderHevcDraft);
                          }}
                        >
                          <AutoFixHighIcon color={encoderLoading === "hevc" ? "primary" : "inherit"} />
                        </IconButton>
                      </Tooltip>
                    </InputAdornment>
                  ),
                }}
              />
            </Grid>
          </Grid>
          <Box sx={{ display: "flex", justifyContent: "flex-end", mt: 3 }}>
            <Button variant="contained" onClick={saveAdvancedSettings}>
              {t("saveVideoSettings")}
            </Button>
          </Box>
        </CardContent>
      </Card>

      <Card elevation={2} sx={{ borderRadius: 3 }}>
        <CardContent sx={{ p: 3 }}>
          <Typography variant="h6" gutterBottom>
            {t("appearance")}
          </Typography>

          <Grid container spacing={3} sx={{ mb: 3 }}>
            <Grid item xs={12} md={6}>
              <FormControl fullWidth>
                <InputLabel>{t("language")}</InputLabel>
                <Select
                  value={settings.locale}
                  label={t("language")}
                  onChange={(event) => {
                    const value = event.target.value;
                    if (isSupportedLocale(value)) setLocale(value);
                  }}
                >
                  {SUPPORTED_LOCALES.map((code) => (
                    <MenuItem key={code} value={code}>
                      {LOCALE_LABELS[code]}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Grid>

            <Grid item xs={12} md={6}>
              <FormControl fullWidth>
                <InputLabel>{t("theme")}</InputLabel>
                <Select
                  value={settings.themeMode}
                  label={t("theme")}
                  onChange={(event) => {
                    const value = event.target.value;
                    if (isThemeMode(value)) setThemeMode(value);
                  }}
                >
                  <MenuItem value="light">{t("themeLight")}</MenuItem>
                  <MenuItem value="dark">{t("themeDark")}</MenuItem>
                  <MenuItem value="system">{t("themeSystem")}</MenuItem>
                </Select>
              </FormControl>
            </Grid>
          </Grid>

          <FormControlLabel
            control={
              <Switch
                checked={followSystem}
                onChange={(e) => setFollowSystem(e.target.checked)}
              />
            }
            label={t("useSystemTheme")}
            sx={{ mb: 3, display: "block" }}
          />

          <Divider sx={{ my: 2 }} />

          <Typography variant="h6" gutterBottom>
            {t("accentColor")}
          </Typography>

          <Box sx={{ display: "flex", gap: 1, flexWrap: "wrap", mb: 2 }}>
            {PRESET_ACCENT_COLORS.map((color) => (
              <Box
                key={color}
                onClick={() => {
                  setAccentColor(color);
                  setAccentDraft(color);
                }}
                sx={{
                  width: 40,
                  height: 40,
                  borderRadius: "50%",
                  bgcolor: color,
                  cursor: "pointer",
                  border:
                    settings.accentColor === color
                      ? "3px solid currentColor"
                      : "3px solid transparent",
                  transition: "transform 0.15s",
                  "&:hover": { transform: "scale(1.1)" },
                }}
              />
            ))}
          </Box>

          <TextField
            label={t("accentColor")}
            value={accentDraft}
            onChange={(e) => setAccentDraft(e.target.value)}
            onBlur={applyDraft}
            onKeyDown={handleKeyDown}
            size="medium"
            error={!draftValid}
            helperText={!draftValid ? t("common:invalidHexColor") : undefined}
            sx={{ width: 240 }}
            InputProps={{
              startAdornment: (
                <Box
                  component="input"
                  type="color"
                  value={draftValid ? (accentDraft.startsWith("#") ? accentDraft : `#${accentDraft}`) : settings.accentColor}
                  onChange={(e) => {
                    setAccentDraft(e.target.value);
                  }}
                  onBlur={applyDraft}
                  sx={{
                    width: 34,
                    height: 28,
                    border: "none",
                    bgcolor: "transparent",
                    cursor: "pointer",
                    mr: 1,
                    mt: 1,
                    p: 0,
                  }}
                />
              ),
              endAdornment: (
                <InputAdornment position="end">
                  <Tooltip title={t("common:apply")}>
                    <span>
                      <IconButton
                        onClick={applyDraft}
                        disabled={!draftValid || applied}
                        size="small"
                        edge="end"
                      >
                        <CheckCircleIcon />
                      </IconButton>
                    </span>
                  </Tooltip>
                </InputAdornment>
              ),
            }}
          />

          <Divider sx={{ my: 3 }} />

          <Button variant="outlined" color="error" onClick={resetSettings}>
            {t("resetSettings")}
          </Button>
        </CardContent>
      </Card>
    </Stack>
  );
};

export default SettingsPage;
