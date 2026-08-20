import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useSnackbar } from "notistack";
import {
  Box,
  Button,
  Card,
  CardContent,
  Fade,
  Stack,
  Typography,
} from "@mui/material";
import AddIcon from "@mui/icons-material/Add";
import FolderOpenIcon from "@mui/icons-material/FolderOpen";
import UploadFileIcon from "@mui/icons-material/UploadFile";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import type { RenderConfig } from "../types/config";
import { normalizeRenderConfig } from "../utils/config";
import type { DragDropPayload } from "../types/events";

const StartPage: React.FC = () => {
  const { t } = useTranslation(["start", "common"]);
  const navigate = useNavigate();
  const { enqueueSnackbar } = useSnackbar();
  const [dragging, setDragging] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const loadConfig = useCallback(
    async (path: string) => {
      try {
        const config = await invoke<RenderConfig>("load_config", { path });
        navigate("/editor", { state: { config } });
      } catch (error) {
        enqueueSnackbar(`${t("start:loadFailed")}: ${error}`, {
          variant: "error",
        });
      }
    },
    [enqueueSnackbar, navigate, t]
  );

  useEffect(() => {
    if (!isTauri()) return;

    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const register = async (listener: Promise<() => void>) => {
      try {
        const unlisten = await listener;
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      } catch {
        if (!disposed) setDragging(false);
      }
    };

    void register(listen("tauri://drag-enter", () => setDragging(true)));
    void register(listen("tauri://drag-leave", () => setDragging(false)));
    void register(
      listen<DragDropPayload>("tauri://drag-drop", async ({ payload }) => {
        setDragging(false);
        const [path] = payload.paths ?? [];
        if (path) await loadConfig(path);
      })
    );

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [loadConfig]);

  const handleNewConfig = async () => {
    try {
      const config = await invoke<RenderConfig>("get_default_config");
      navigate("/editor", { state: { config } });
    } catch (e) {
      enqueueSnackbar(`${t("start:newFailed")}: ${e}`, { variant: "error" });
    }
  };

  const handleLoadConfig = async () => {
    if (!isTauri()) {
      fileInputRef.current?.click();
      return;
    }
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: "JSON Files", extensions: ["json"] }],
      });
      if (path) {
        await loadConfig(path);
      }
    } catch (e) {
      enqueueSnackbar(`${t("start:loadFailed")}: ${e}`, { variant: "error" });
    }
  };

  const handleFileInput = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      const config = normalizeRenderConfig(JSON.parse(text) as RenderConfig);
      navigate("/editor", { state: { config } });
    } catch (e) {
      enqueueSnackbar(`${t("start:loadFailed")}: ${e}`, { variant: "error" });
    } finally {
      e.target.value = "";
    }
  };

  return (
    <Box sx={{ position: "relative", minHeight: "60vh" }}>
      <Fade in={dragging} timeout={200}>
        <Box
          sx={{
            position: "fixed",
            inset: 0,
            zIndex: 1300,
            bgcolor: "rgba(103, 80, 164, 0.15)",
            backdropFilter: "blur(4px)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            border: "4px dashed",
            borderColor: "primary.main",
          }}
        >
          <Typography variant="h3" color="primary.main" sx={{ fontWeight: 700 }}>
            {t("start:dropHere")}
          </Typography>
        </Box>
      </Fade>

      <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
        {t("start:subtitle")}
      </Typography>

      <Stack
        direction={{ xs: "column", md: "row" }}
        spacing={2}
        sx={{ mb: 3 }}
      >
        <Card
          sx={{
            flex: 1,
            borderRadius: 3,
            transition: "transform 0.2s, box-shadow 0.2s",
            "&:hover": { transform: "translateY(-4px)", boxShadow: 6 },
          }}
        >
          <CardContent sx={{ p: 3, textAlign: "center" }}>
            <Box
              sx={{
                width: 48,
                height: 48,
                borderRadius: "50%",
                bgcolor: "secondary.main",
                color: "secondary.contrastText",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                mx: "auto",
                mb: 2,
              }}
            >
              <AddIcon fontSize="medium" />
            </Box>
            <Typography variant="h6" gutterBottom>
              {t("start:newConfig")}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {t("start:newConfigDesc")}
            </Typography>
            <Button
              variant="contained"
              fullWidth
              onClick={handleNewConfig}
              startIcon={<UploadFileIcon />}
            >
              {t("start:createNew")}
            </Button>
          </CardContent>
        </Card>

        <Card
          sx={{
            flex: 1,
            borderRadius: 3,
            transition: "transform 0.2s, box-shadow 0.2s",
            "&:hover": { transform: "translateY(-4px)", boxShadow: 6 },
          }}
        >
          <CardContent sx={{ p: 3, textAlign: "center" }}>
            <Box
              sx={{
                width: 48,
                height: 48,
                borderRadius: "50%",
                bgcolor: "secondary.main",
                color: "secondary.contrastText",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                mx: "auto",
                mb: 2,
              }}
            >
              <FolderOpenIcon fontSize="medium" />
            </Box>
            <Typography variant="h6" gutterBottom>
              {t("start:loadConfig")}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {t("start:loadConfigDesc")}
            </Typography>
            <Button
              variant="contained"
              fullWidth
              onClick={handleLoadConfig}
              startIcon={<UploadFileIcon />}
            >
              {t("start:selectFile")}
            </Button>
            <input
              type="file"
              accept=".json,application/json"
              ref={fileInputRef}
              onChange={handleFileInput}
              style={{ display: "none" }}
            />
          </CardContent>
        </Card>
      </Stack>

      <Typography variant="body2" color="text.secondary" align="center">
        {t("start:dropHint")}
      </Typography>
    </Box>
  );
};

export default StartPage;
