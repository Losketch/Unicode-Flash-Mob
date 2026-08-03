import React, {
  createContext,
  useContext,
  useMemo,
  useState,
  useCallback,
  useEffect,
} from "react";
import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { RenderProgressPayload } from "../types/events";
import { loadStoredJson, saveStoredJson } from "../utils/storage";

export type TaskStatusType =
  | "pending"
  | "running"
  | "done"
  | "failed"
  | "cancelled";

export interface TaskItem {
  id: string;
  name: string;
  type: "render" | "extract";
  status: TaskStatusType;
  progress: number;
  message: string;
  outputPath?: string;
  configPath?: string;
  createdAt: number;
  updatedAt: number;
}

interface TaskContextValue {
  tasks: TaskItem[];
  addTask: (task: Omit<TaskItem, "id" | "createdAt" | "updatedAt">) => string;
  updateTask: (id: string, patch: Partial<Omit<TaskItem, "id" | "createdAt">>) => void;
  removeTask: (id: string) => void;
  clearCompleted: () => void;
}

const TaskContext = createContext<TaskContextValue | null>(null);

function generateId() {
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 9)}`;
}

const TASKS_STORAGE_KEY = "unicode-flash-mob.tasks";

const TASK_STATUSES: TaskStatusType[] = [
  "pending",
  "running",
  "done",
  "failed",
  "cancelled",
];

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

function parseStoredTask(value: unknown): TaskItem | null {
  if (!isRecord(value) || typeof value.id !== "string") return null;
  if (value.type !== "render" && value.type !== "extract") return null;
  if (!TASK_STATUSES.includes(value.status as TaskStatusType)) return null;

  const wasInterrupted = value.status === "running" || value.status === "pending";
  const now = Date.now();
  return {
    id: value.id,
    name: typeof value.name === "string" ? value.name : value.id,
    type: value.type,
    status: wasInterrupted ? "cancelled" : (value.status as TaskStatusType),
    progress:
      typeof value.progress === "number" && Number.isFinite(value.progress)
        ? Math.max(0, Math.min(1, value.progress))
        : 0,
    message: typeof value.message === "string" ? value.message : "",
    outputPath: typeof value.outputPath === "string" ? value.outputPath : undefined,
    configPath: typeof value.configPath === "string" ? value.configPath : undefined,
    createdAt:
      typeof value.createdAt === "number" && Number.isFinite(value.createdAt)
        ? value.createdAt
        : now,
    updatedAt:
      wasInterrupted
        ? now
        : typeof value.updatedAt === "number" && Number.isFinite(value.updatedAt)
          ? value.updatedAt
          : now,
  };
}

function loadTasks(): TaskItem[] {
  const parsed = loadStoredJson<unknown>(TASKS_STORAGE_KEY, []);
  if (!Array.isArray(parsed)) return [];
  return parsed.flatMap((task) => {
    const parsedTask = parseStoredTask(task);
    return parsedTask ? [parsedTask] : [];
  });
}

export const TaskProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [tasks, setTasks] = useState<TaskItem[]>(loadTasks);

  useEffect(() => {
    saveStoredJson(TASKS_STORAGE_KEY, tasks);
  }, [tasks]);

  useEffect(() => {
    if (!isTauri()) return;

    let disposed = false;
    let unlisten: (() => void) | undefined;

    const setup = async () => {
      try {
        const stopListening = await listen<RenderProgressPayload>(
          "render-progress",
          ({ payload }) => {
            const progress =
              typeof payload.progress === "number" && Number.isFinite(payload.progress)
                ? Math.max(0, Math.min(1, payload.progress))
                : 0;

            setTasks((current) => {
              let taskId = payload.taskId;
              if (!taskId) {
                const runningRenderTasks = current.filter(
                  (task) => task.type === "render" && task.status === "running"
                );
                if (runningRenderTasks.length === 1) {
                  taskId = runningRenderTasks[0].id;
                }
              }
              if (!taskId) return current;

              let changed = false;
              const updatedAt = Date.now();
              const next = current.map((task) => {
                if (task.id !== taskId || task.progress === progress) return task;
                changed = true;
                return { ...task, progress, updatedAt };
              });
              return changed ? next : current;
            });
          }
        );

        if (disposed) stopListening();
        else unlisten = stopListening;
      } catch {
        // Rendering still works if event subscription is unavailable.
      }
    };

    void setup();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const addTask = useCallback(
    (task: Omit<TaskItem, "id" | "createdAt" | "updatedAt">) => {
      const id = generateId();
      const now = Date.now();
      const newTask: TaskItem = {
        ...task,
        id,
        createdAt: now,
        updatedAt: now,
      };
      setTasks((prev) => [newTask, ...prev]);
      return id;
    },
    []
  );

  const updateTask = useCallback(
    (id: string, patch: Partial<Omit<TaskItem, "id" | "createdAt">>) => {
      setTasks((prev) =>
        prev.map((task) =>
          task.id === id
            ? { ...task, ...patch, updatedAt: Date.now() }
            : task
        )
      );
    },
    []
  );

  const removeTask = useCallback((id: string) => {
    setTasks((prev) => prev.filter((task) => task.id !== id));
  }, []);

  const clearCompleted = useCallback(() => {
    setTasks((prev) =>
      prev.filter(
        (task) =>
          task.status !== "done" &&
          task.status !== "failed" &&
          task.status !== "cancelled"
      )
    );
  }, []);

  const value = useMemo(
    () => ({
      tasks,
      addTask,
      updateTask,
      removeTask,
      clearCompleted,
    }),
    [tasks, addTask, updateTask, removeTask, clearCompleted]
  );

  return <TaskContext.Provider value={value}>{children}</TaskContext.Provider>;
};

export const useTasks = (): TaskContextValue => {
  const ctx = useContext(TaskContext);
  if (!ctx) {
    throw new Error("useTasks must be used within TaskProvider");
  }
  return ctx;
};
