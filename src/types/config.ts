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

export interface TextElement {
  id: string;
  fonts: string[];
  content: string;
  position: Position;
  color: Color;
  enabled: boolean;
  align: TextAlign;
  wrap: boolean;
  max_width: number;
}

export type EventType =
  | { set_background_color: { color: Color } }
  | { set_text_color: { element_id: string; color: Color } }
  | {
      set_text_position: {
        element_id: string;
        position: Position;
      };
    }
  | {
      move_text_position: {
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
  output_path: string;
  resolution: [number, number];
  fps: number;
  background_colors: Color[];
  dynamic_background: boolean;
  background_color: Color;
  fixed_background: boolean;
  main_font: FontConfig;
  bottom_font: FontConfig;
  text_x_offset: number;
  text_y_offset: number;
  overlay_enabled: boolean;
  main_text: TextElement;
  bottom_text: TextElement;
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
