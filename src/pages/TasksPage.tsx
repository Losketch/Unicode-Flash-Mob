import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useSnackbar } from "notistack";
import {
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  IconButton,
  LinearProgress,
  Stack,
  Tooltip,
  Typography,
} from "@mui/material";
import DeleteIcon from "@mui/icons-material/Delete";
import FolderOpenIcon from "@mui/icons-material/FolderOpen";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import { invoke } from "@tauri-apps/api/core";
import { useTasks, type TaskItem } from "../contexts/TaskContext";
import { formatError } from "../utils/errors";

const statusColors: Record<TaskItem["status"], "default" | "primary" | "success" | "error" | "warning"> = {
  pending: "default",
  running: "primary",
  done: "success",
  failed: "error",
  cancelled: "warning",
};

const TasksPage: React.FC = () => {
  const { t } = useTranslation("tasks");
  const { tasks, removeTask, clearCompleted } = useTasks();
  const navigate = useNavigate();
  const { enqueueSnackbar } = useSnackbar();

  const openTaskConfig = async (task: TaskItem) => {
    if (!task.configPath) return;
    try {
      const config = await invoke("load_config", { path: task.configPath });
      navigate("/editor", { state: { config } });
    } catch (error) {
      enqueueSnackbar(`${t("loadConfigFailed")}: ${formatError(error)}`, { variant: "error" });
    }
  };

  return (
    <Box>
      <Stack
        direction="row"
        justifyContent="space-between"
        alignItems="center"
        sx={{ mb: 3 }}
      >
        <Button variant="outlined" onClick={clearCompleted}>
          {t("clearCompleted")}
        </Button>
      </Stack>

      {tasks.length === 0 ? (
        <Typography color="text.secondary" align="center" sx={{ mt: 6 }}>
          {t("empty")}
        </Typography>
      ) : (
        <Stack spacing={2}>
          {tasks.map((task) => (
            <Card
              key={task.id}
              onClick={() => void openTaskConfig(task)}
              role={task.configPath ? "button" : undefined}
              tabIndex={task.configPath ? 0 : undefined}
              onKeyDown={(event) => {
                if (task.configPath && (event.key === "Enter" || event.key === " ")) {
                  event.preventDefault();
                  void openTaskConfig(task);
                }
              }}
              sx={{
                borderRadius: 3,
                cursor: task.configPath ? "pointer" : "default",
                "&:hover": task.configPath ? { bgcolor: "action.hover" } : undefined,
              }}
            >
              <CardContent>
                <Stack
                  direction={{ xs: "column", sm: "row" }}
                  justifyContent="space-between"
                  alignItems={{ xs: "flex-start", sm: "center" }}
                  spacing={2}
                >
                  <Box>
                    <Stack direction="row" spacing={1} alignItems="center">
                      {task.type === "render" ? (
                        <PlayArrowIcon color="primary" />
                      ) : (
                        <FolderOpenIcon color="secondary" />
                      )}
                      <Typography variant="h6">{task.name}</Typography>
                      <Chip
                        label={t(`status.${task.status}`)}
                        color={statusColors[task.status]}
                        size="small"
                      />
                    </Stack>
                    <Typography variant="body2" color="text.secondary">
                      {task.message}
                    </Typography>
                    {task.outputPath && (
                      <Typography variant="caption" color="text.secondary">
                        {task.outputPath}
                      </Typography>
                    )}
                  </Box>
                  <Stack direction="row" spacing={0.5} alignItems="center">
                    <Tooltip title={t("deleteTask")} arrow>
                      <IconButton
                        onClick={(event) => {
                          event.stopPropagation();
                          removeTask(task.id);
                        }}
                        color="error"
                        size="small"
                        aria-label={t("deleteTask")}
                      >
                        <DeleteIcon />
                      </IconButton>
                    </Tooltip>
                  </Stack>
                </Stack>

                {task.status === "running" && (
                  <Box sx={{ mt: 2 }}>
                    <LinearProgress
                      variant="determinate"
                      value={task.progress * 100}
                    />
                    <Typography variant="caption" color="text.secondary">
                      {(task.progress * 100).toFixed(1)}%
                    </Typography>
                  </Box>
                )}

                <Typography variant="caption" color="text.secondary" display="block" sx={{ mt: 1 }}>
                  {new Date(task.createdAt).toLocaleString()}
                </Typography>
              </CardContent>
            </Card>
          ))}
        </Stack>
      )}
    </Box>
  );
};

export default TasksPage;
