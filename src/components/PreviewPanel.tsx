import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSnackbar } from "notistack";
import {
  Paper,
  Button,
  Box,
  Typography,
  LinearProgress,
  Stack,
  Card,
  CardContent,
  Alert,
} from "@mui/material";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTasks } from "../contexts/TaskContext";
import { useSettings } from "../contexts/SettingsContext";
import { applyOutputDirectory, getSidecarConfigPath } from "../utils/render";
import type { RenderConfig } from "../types/config";
import type { RenderProgressPayload } from "../types/events";
import { formatError } from "../utils/errors";

interface PreviewPanelProps {
  config: RenderConfig | null;
}

const PreviewPanel: React.FC<PreviewPanelProps> = ({ config }) => {
  const { t } = useTranslation(["render", "common", "config"]);
  const { enqueueSnackbar } = useSnackbar();
  const { addTask, updateTask } = useTasks();
  const { settings } = useSettings();
  const [isRendering, setIsRendering] = useState(false);
  const [progress, setProgress] = useState(0);
  const [status, setStatus] = useState(t("render:statusReady"));
  const [lastError, setLastError] = useState<string | null>(null);
  const currentTaskIdRef = useRef<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    const setup = async () => {
      try {
        const stopListening = await listen<RenderProgressPayload>(
          "render-progress",
          ({ payload }) => {
            const taskId = currentTaskIdRef.current;
            if (payload.taskId && payload.taskId !== taskId) return;

            const value = payload.progress ?? 0;
            setProgress(value * 100);
            setStatus(
              t("render:statusRenderingProgress", {
                current: payload.current ?? 0,
                total: payload.total ?? 0,
              })
            );
          }
        );
        if (disposed) stopListening();
        else unlisten = stopListening;
      } catch {
        setStatus(t("render:statusReady"));
      }
    };

    void setup();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [t]);

  const handleRender = async () => {
    if (!config) {
      enqueueSnackbar(t("render:noConfigError"), { variant: "error" });
      return;
    }

    setIsRendering(true);
    setStatus(t("render:statusRendering"));
    setProgress(0);
    setLastError(null);

    const renderConfig = {
      ...config,
      ffmpeg: {
        ...config.ffmpeg,
        encoder:
          config.ffmpeg?.encoder && config.ffmpeg.encoder !== "auto"
            ? config.ffmpeg.encoder
            : settings.encoderAvc || config.ffmpeg?.encoder || "auto",
      },
    };
    renderConfig.output_path = applyOutputDirectory(
      renderConfig.output_path,
      settings.outputDir
    );
    const configPath = getSidecarConfigPath(renderConfig.output_path);
    try {
      await invoke("validate_config", { config: renderConfig, requireFrames: true });
    } catch (error) {
      const message = `${t("render:preflightFailed")}: ${formatError(error)}`;
      setLastError(message);
      enqueueSnackbar(message, { variant: "error" });
      setIsRendering(false);
      setStatus(t("render:statusFailed"));
      return;
    }
    try {
      await invoke("save_config", { path: configPath, config: renderConfig });
    } catch (error) {
      const message = `${t("render:saveConfigFailed")}: ${formatError(error)}`;
      setLastError(message);
      enqueueSnackbar(message, { variant: "error" });
      setIsRendering(false);
      setStatus(t("render:statusFailed"));
      return;
    }

    const taskId = addTask({
      name: renderConfig.title || renderConfig.output_path || t("render:title"),
      type: "render",
      status: "running",
      progress: 0,
      message: t("render:statusRendering"),
      outputPath: renderConfig.output_path,
      configPath,
    });
    currentTaskIdRef.current = taskId;

    try {
      await invoke("render_video", { config: renderConfig, taskId });
      setStatus(t("render:statusDone"));
      setProgress(100);
      updateTask(taskId, {
        status: "done",
        progress: 1,
        message: t("render:statusDone"),
      });
      enqueueSnackbar(t("render:renderSuccess"), { variant: "success" });
    } catch (error) {
      const message = `${t("render:renderError")}: ${formatError(error)}`;
      setLastError(message);
      setStatus(t("render:statusFailed"));
      updateTask(taskId, {
        status: "failed",
        message,
      });
      enqueueSnackbar(message, { variant: "error" });
    } finally {
      setIsRendering(false);
      currentTaskIdRef.current = null;
    }
  };

  return (
    <Paper elevation={2} sx={{ p: 2.5, borderRadius: 3 }}>
      <Typography variant="h5" gutterBottom>
        {t("render:title")}
      </Typography>
      <Card sx={{ borderRadius: 3 }}>
        <CardContent>
          <Stack direction="row" spacing={2} alignItems="center">
            <Button
              variant="contained"
              startIcon={<PlayArrowIcon />}
              onClick={handleRender}
              disabled={isRendering || !config}
            >
              {isRendering ? t("render:rendering") : t("render:renderVideo")}
            </Button>
            <Typography variant="body2" color="text.secondary">
              {status}
            </Typography>
          </Stack>
          {(isRendering || progress > 0) && (
            <Box sx={{ width: "100%", mt: 2 }}>
              <LinearProgress
                variant={isRendering && progress === 0 ? "indeterminate" : "determinate"}
                value={progress}
              />
            </Box>
          )}
        </CardContent>
      </Card>
      {lastError && (
        <Alert severity="error" sx={{ mt: 2, whiteSpace: "pre-wrap" }}>
          {lastError}
        </Alert>
      )}
      {!config && (
        <Typography color="text.secondary" align="center" sx={{ mt: 4 }}>
          {t("render:noConfigError")}
        </Typography>
      )}
    </Paper>
  );
};

export default PreviewPanel;
