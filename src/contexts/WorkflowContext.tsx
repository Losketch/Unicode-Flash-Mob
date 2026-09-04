import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { loadStoredJson, saveStoredJson } from "../utils/storage";
import type { RenderConfig } from "../types/config";
import { normalizeRenderConfig } from "../utils/config";

export interface WorkflowState {
  config: RenderConfig | null;
  activeStep: number;
}

const STORAGE_KEY = "ufm-workflow";

const DEFAULT_STATE: WorkflowState = {
  config: null,
  activeStep: 0,
};

function loadState(): WorkflowState {
  const stored = loadStoredJson<Partial<WorkflowState>>(STORAGE_KEY, {});
  return {
    ...DEFAULT_STATE,
    ...stored,
    config: stored.config ? normalizeRenderConfig(stored.config) : null,
  };
}

interface WorkflowContextValue extends WorkflowState {
  setConfig: (config: RenderConfig) => void;
  setActiveStep: (step: number) => void;
  updateConfig: (patch: Partial<RenderConfig>) => void;
  resetWorkflow: () => void;
}

const WorkflowContext = createContext<WorkflowContextValue | null>(null);

export const WorkflowProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [state, setState] = useState<WorkflowState>(loadState);
  const persistTimerRef = useRef<number | null>(null);
  const latestStateRef = useRef(state);
  latestStateRef.current = state;

  useEffect(() => {
    if (persistTimerRef.current !== null) {
      window.clearTimeout(persistTimerRef.current);
    }
    persistTimerRef.current = window.setTimeout(() => {
      saveStoredJson(STORAGE_KEY, state);
      persistTimerRef.current = null;
    }, 180);

    return () => {
      if (persistTimerRef.current !== null) {
        window.clearTimeout(persistTimerRef.current);
        persistTimerRef.current = null;
      }
    };
  }, [state]);

  useEffect(() => () => {
    if (persistTimerRef.current !== null) {
      window.clearTimeout(persistTimerRef.current);
    }
    saveStoredJson(STORAGE_KEY, latestStateRef.current);
  }, []);

  const setConfig = useCallback((config: RenderConfig) => {
    setState((prev) => ({ ...prev, config: normalizeRenderConfig(config) }));
  }, []);

  const setActiveStep = useCallback((activeStep: number) => {
    setState((prev) => ({ ...prev, activeStep }));
  }, []);

  const updateConfig = useCallback((patch: Partial<RenderConfig>) => {
    setState((prev) =>
      prev.config
        ? { ...prev, config: { ...prev.config, ...patch } }
        : prev
    );
  }, []);

  const resetWorkflow = useCallback(() => {
    setState(DEFAULT_STATE);
    saveStoredJson(STORAGE_KEY, DEFAULT_STATE);
  }, []);

  const value = useMemo(
    () => ({
      ...state,
      setConfig,
      setActiveStep,
      updateConfig,
      resetWorkflow,
    }),
    [state, setConfig, setActiveStep, updateConfig, resetWorkflow]
  );

  return (
    <WorkflowContext.Provider value={value}>{children}</WorkflowContext.Provider>
  );
};

export const useWorkflow = (): WorkflowContextValue => {
  const ctx = useContext(WorkflowContext);
  if (!ctx) {
    throw new Error("useWorkflow must be used within WorkflowProvider");
  }
  return ctx;
};
