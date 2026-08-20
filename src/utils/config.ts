import { getPrimaryGlyphComponent } from "../types/config";
import type {
  Event,
  FontConfig,
  GlyphComponent,
  RenderConfig,
  RenderConfigPath,
  SceneComponent,
  TextComponent,
} from "../types/config";
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

type LegacyTextElement = Partial<
  Omit<TextComponent, "type" | "font">
> & {
  id?: string;
  fonts?: string[];
};

type LegacyRenderConfig = Partial<RenderConfig> & {
  main_font?: Partial<FontConfig>;
  bottom_font?: Partial<FontConfig>;
  main_text?: LegacyTextElement;
  bottom_text?: LegacyTextElement;
};

const normalizeFontConfig = (
  value: Partial<FontConfig> | undefined,
  fallbackSize: number
): FontConfig => ({
  size: Number.isFinite(value?.size) ? Number(value?.size) : fallbackSize,
  font_feature_settings: value?.font_feature_settings ?? {},
  font_variation_settings: value?.font_variation_settings ?? {},
  font_optical_sizing: value?.font_optical_sizing !== false,
});

const normalizeEvent = (event: Event): Event => {
  const eventType = event.event_type as Event["event_type"] &
    Record<string, unknown>;
  const aliases: Array<[string, string]> = [
    ["set_text_color", "set_component_color"],
    ["set_text_position", "set_component_position"],
    ["move_text_position", "move_component_position"],
  ];

  for (const [legacyName, currentName] of aliases) {
    if (legacyName in eventType) {
      return {
        ...event,
        event_type: {
          [currentName]: eventType[legacyName],
        } as Event["event_type"],
      };
    }
  }
  return event;
};

const normalizeSceneComponent = (component: SceneComponent): SceneComponent => {
  if (component.type === "glyph") {
    return {
      ...component,
      enabled: component.enabled !== false,
      content: component.content || "{glyph}",
      fonts: component.fonts ?? [],
      font: normalizeFontConfig(component.font, 512),
      overlay_combining_mark: component.overlay_combining_mark !== false,
    };
  }
  if (component.type === "text") {
    return {
      ...component,
      enabled: component.enabled !== false,
      content: component.content ?? "",
      fonts: component.fonts ?? [],
      font: normalizeFontConfig(component.font, 42),
      align: component.align ?? "left",
      wrap: component.wrap ?? false,
      max_width: Number.isFinite(component.max_width) ? component.max_width : 0,
    };
  }

  // Preserve future component types verbatim if a newer backend writes them.
  return component;
};

/**
 * Normalize configuration JSON loaded outside Tauri. Desktop loading uses the
 * Rust migration path; this keeps browser file import and persisted editor state
 * compatible with the same v3 -> v4 scene transition.
 */
export function normalizeRenderConfig(
  input: RenderConfig | LegacyRenderConfig
): RenderConfig {
  const raw = input as LegacyRenderConfig;
  const hasComponents = Array.isArray(raw.components);
  let components: SceneComponent[];

  if (hasComponents) {
    components = (raw.components as SceneComponent[]).map(normalizeSceneComponent);
  } else {
    const main = raw.main_text;
    const bottom = raw.bottom_text;
    const glyph: GlyphComponent = {
      type: "glyph",
      id: main?.id || "main",
      enabled: main?.enabled !== false,
      content: "{glyph}",
      position: main?.position ?? { x: 0.5, y: 0.5 },
      color: main?.color ?? { r: 0, g: 0, b: 0, a: 128 },
      fonts: main?.fonts ?? [],
      font: normalizeFontConfig(raw.main_font, 512),
      overlay_combining_mark: true,
    };
    const bottomFonts = bottom?.fonts ?? [];
    const text: TextComponent = {
      type: "text",
      id: bottom?.id || "bottom",
      enabled: bottom?.enabled ?? bottomFonts.length > 0,
      content: bottom?.content ?? "{code}\n{description}",
      position: bottom?.position ?? { x: 0.05, y: 0.95 },
      color: bottom?.color ?? { r: 0, g: 0, b: 0, a: 128 },
      fonts: bottomFonts,
      font: normalizeFontConfig(raw.bottom_font, 42),
      align: bottom?.align ?? "left",
      wrap: bottom?.wrap ?? true,
      max_width: Number.isFinite(bottom?.max_width) ? Number(bottom?.max_width) : 0.9,
    };
    components = [glyph, text];
  }

  const current: LegacyRenderConfig = { ...raw };
  delete current.main_font;
  delete current.bottom_font;
  delete current.main_text;
  delete current.bottom_text;

  return {
    ...(current as RenderConfig),
    schema_version: Math.max(Number(raw.schema_version) || 0, 4),
    components,
    content_templates: raw.content_templates ?? {},
    events: (raw.events ?? []).map(normalizeEvent),
  };
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
  const extractedPrimary = getPrimaryGlyphComponent(extracted);
  const currentPrimary = getPrimaryGlyphComponent(current);
  const components = current.components.map((component) =>
    component === currentPrimary && extractedPrimary
      ? { ...component, fonts: extractedPrimary.fonts }
      : component
  );

  return {
    ...current,
    characters: extracted.characters,
    components,
  };
}
