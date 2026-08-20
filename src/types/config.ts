/** Mirrors the snake_case fields serialized by Rust RenderConfig. */

export interface Color {
  r: number;
  g: number;
  b: number;
  a: number;
}

export interface Position {
  x: number;
  y: number;
}

export type TextAlign = "left" | "center" | "right";

export type AnimationCurve =
  | "linear"
  | "ease_in"
  | "ease_out"
  | "ease_in_out"
  | "bounce";

export interface FontConfig {
  size: number;
  font_feature_settings: Record<string, number>;
  font_variation_settings: Record<string, number>;
  font_optical_sizing: boolean;
}

export interface GlyphComponent {
  type: "glyph";
  id: string;
  enabled: boolean;
  content: string;
  position: Position;
  color: Color;
  fonts: string[];
  font: FontConfig;
  overlay_combining_mark: boolean;
}

export interface TextComponent {
  type: "text";
  id: string;
  enabled: boolean;
  content: string;
  position: Position;
  color: Color;
  fonts: string[];
  font: FontConfig;
  align: TextAlign;
  wrap: boolean;
  max_width: number;
}

export type SceneComponent = GlyphComponent | TextComponent;

export type ContentTemplate =
  | { type: "text"; value: string }
  | { type: "external"; executable: string; args: string[] };

export type EventType =
  | { set_background_color: { color: Color } }
  | { set_component_color: { element_id: string; color: Color } }
  | {
      set_component_position: {
        element_id: string;
        position: Position;
      };
    }
  | {
      move_component_position: {
        element_id: string;
        start_position: Position;
        end_position: Position;
        duration: number;
        curve: AnimationCurve;
      };
    }
  | {
      color_transition: {
        start_color: Color;
        end_color: Color;
        duration: number;
        curve: AnimationCurve;
      };
    }
  | { pause: { duration: number } };

export interface Event {
  frame: number;
  event_type: EventType;
}

export interface CharEntry {
  code_point: string;
  description: string;
  background_color: Color | null;
  text_color: Color | null;
  position: Position | null;
  duration_frames: number | null;
}

export interface FfmpegConfig {
  path: string;
  crf: number;
  preset: string;
  encoder: string;
  pixel_format: string;
  parallel_workers: number;
  max_inflight_frames: number;
  encoding_processes: number;
}

export interface RenderConfig {
  schema_version: number;
  output_path: string;
  resolution: [number, number];
  fps: number;
  background_colors: Color[];
  dynamic_background: boolean;
  background_color: Color;
  fixed_background: boolean;
  components: SceneComponent[];
  content_templates: Record<string, ContentTemplate>;
  text_x_offset: number;
  text_y_offset: number;
  overlay_enabled: boolean;
  characters: CharEntry[];
  events: Event[];
  ffmpeg: FfmpegConfig;
  music_path: string | null;
  title: string;
}

export type Config = RenderConfig;

type AtomicConfigValue = string | number | boolean | null | undefined;
type DeepConfigPath<T> = T extends AtomicConfigValue | readonly unknown[]
  ? never
  : {
      [Key in keyof T & string]: T[Key] extends
        | AtomicConfigValue
        | readonly unknown[]
        ? Key
        : Key | `${Key}.${DeepConfigPath<T[Key]>}`;
    }[keyof T & string];

/** Dot-separated paths accepted by the configuration editor. */
export type RenderConfigPath = DeepConfigPath<RenderConfig>;

export const getPrimaryGlyphComponent = (
  config: RenderConfig | null | undefined
): GlyphComponent | undefined =>
  config?.components?.find(
    (component): component is GlyphComponent => component.type === "glyph"
  );
