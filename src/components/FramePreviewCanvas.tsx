import React, { useEffect, useRef, useState } from "react";
import { Box, CircularProgress, Paper, Typography } from "@mui/material";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import type { RenderConfig } from "../types/config";

interface FramePreviewCanvasProps {
  config: RenderConfig | null;
  entryIndex: number;
}

const FramePreviewCanvas: React.FC<FramePreviewCanvasProps> = ({
  config,
  entryIndex,
}) => {
  const { t } = useTranslation("editor");
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const activeUrlRef = useRef<string | null>(null);

  useEffect(() => () => {
    if (activeUrlRef.current) URL.revokeObjectURL(activeUrlRef.current);
  }, []);

  useEffect(() => {
    if (!config?.characters?.[entryIndex]) {
      setImageUrl(null);
      setError(null);
      return;
    }
    let disposed = false;
    const timer = window.setTimeout(async () => {
      setLoading(true);
      try {
        const bytes = await invoke<number[]>("render_frame_preview", {
          config,
          entryIndex,
          maxDimension: 960,
        });
        if (disposed) return;
        const createdUrl = URL.createObjectURL(
          new Blob([new Uint8Array(bytes)], { type: "image/png" })
        );
        if (activeUrlRef.current) URL.revokeObjectURL(activeUrlRef.current);
        activeUrlRef.current = createdUrl;
        setImageUrl(createdUrl);
        setError(null);
      } catch (reason) {
        if (!disposed) setError(String(reason));
      } finally {
        if (!disposed) setLoading(false);
      }
    }, 260);
    return () => {
      disposed = true;
      window.clearTimeout(timer);
    };
  }, [config, entryIndex]);

  return (
    <Paper
      elevation={2}
      sx={{
        borderRadius: 3,
        minHeight: 360,
        p: 1.5,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        position: "sticky",
        top: 56,
        overflow: "hidden",
        bgcolor: "action.hover",
      }}
    >
      {imageUrl ? (
        <Box
          component="img"
          src={imageUrl}
          alt={t("currentFrame")}
          sx={{ maxWidth: "100%", maxHeight: "72vh", objectFit: "contain", borderRadius: 2 }}
        />
      ) : (
        <Typography color="text.secondary" align="center">
          {error || t("previewEmpty")}
        </Typography>
      )}
      {loading && (
        <Box
          sx={{
            position: "absolute",
            inset: 0,
            display: "grid",
            placeItems: "center",
            bgcolor: "rgba(0, 0, 0, 0.12)",
          }}
        >
          <CircularProgress />
        </Box>
      )}
    </Paper>
  );
};

export default FramePreviewCanvas;
