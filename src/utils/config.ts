import type { RenderConfig, RenderConfigPath } from "../types/config";
import pkg from "../../package.json";

export interface RgbaColor {
  r: number;
  g: number;
  b: number;
  a?: number;
}

export const APP_VERSION: string = pkg.version;

export const clampByte = (value: number): number =>
  Math.max(0, Math.min(255, Number.isFinite(value) ? value : 0));

export const colorToHex = (color: Pick<RgbaColor, "r" | "g" | "b">): string =>
  `#${[color.r, color.g, color.b]
    .map((part) => clampByte(Number(part)).toString(16).padStart(2, "0"))
    .join("")}`;

export const isHexColor = (value: string): boolean =>
  /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(value.trim());

/** Parse #RRGGBB, returning null for invalid input. */
export function hexToRgb(
  hex: string
): { r: number; g: number; b: number } | null {
  const normalized = hex.trim();
  if (!/^#[0-9a-fA-F]{6}$/.test(normalized)) return null;
  return {
    r: Number.parseInt(normalized.slice(1, 3), 16),
    g: Number.parseInt(normalized.slice(3, 5), 16),
    b: Number.parseInt(normalized.slice(5, 7), 16),
  };
}

type MutableRecord = Record<string, unknown>;

const isRecord = (value: unknown): value is MutableRecord =>
  typeof value === "object" && value !== null && !Array.isArray(value);

export function setNestedValue(
  source: RenderConfig,
  path: RenderConfigPath,
  value: unknown
): RenderConfig {
  const next = { ...source } as RenderConfig & MutableRecord;
  const keys = path.split(".");
  let target: MutableRecord = next;

  for (const key of keys.slice(0, -1)) {
    const currentValue = target[key];
    const child: MutableRecord = isRecord(currentValue)
      ? { ...currentValue }
      : {};
    target[key] = child;
    target = child;
  }

  target[keys[keys.length - 1]] = value;
  return next;
}

export function moveItem<T>(items: readonly T[], from: number, to: number): T[] {
  if (
    from === to ||
    from < 0 ||
    to < 0 ||
    from >= items.length ||
    to >= items.length
  ) {
    return [...items];
  }
  const next = [...items];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

/**
 * Replace extracted character data and normalized main-font paths while
 * preserving user-edited rendering settings.
 */
export function mergeExtractedConfig(
  extracted: RenderConfig,
  current: RenderConfig
): RenderConfig {
  return {
    ...current,
    characters: extracted.characters,
    main_text: {
      ...current.main_text,
      fonts: extracted.main_text.fonts,
    },
  };
}
