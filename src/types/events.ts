export interface RenderProgressPayload {
  taskId?: string | null;
  progress?: number;
  current?: number;
  total?: number;
}

export interface DragDropPayload {
  paths?: string[];
}
