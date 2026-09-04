import type {
  FontConfig,
  GlyphComponent,
  GroupComponent,
  ImageComponent,
  ProgressBarComponent,
  SceneComponent,
  TextComponent,
} from "../types/config";

export type SceneComponentType = SceneComponent["type"];

const defaultFont = (size: number): FontConfig => ({
  size,
  font_feature_settings: {},
  font_variation_settings: {},
  font_optical_sizing: true,
});

export const collectComponentIds = (
  components: SceneComponent[],
  out = new Set<string>()
): Set<string> => {
  for (const component of components) {
    out.add(component.id);
    if (component.type === "group") {
      collectComponentIds(component.children, out);
    }
  }
  return out;
};

export const collectGroupIds = (
  components: SceneComponent[],
  out = new Set<string>()
): Set<string> => {
  for (const component of components) {
    if (component.type === "group") {
      out.add(component.id);
      collectGroupIds(component.children, out);
    }
  }
  return out;
};

const nextId = (components: SceneComponent[], prefix: string): string => {
  const used = collectComponentIds(components);
  let index = 1;
  while (used.has(`${prefix}_${index}`)) index += 1;
  return `${prefix}_${index}`;
};

const createGlyphComponent = (components: SceneComponent[]): GlyphComponent => ({
  type: "glyph",
  id: nextId(components, "glyph"),
  enabled: false,
  content: "{glyph}",
  position: { x: 0.5, y: 0.5 },
  color: { r: 0, g: 0, b: 0, a: 128 },
  fonts: [],
  font: defaultFont(512),
  overlay_combining_mark: true,
});

const createTextComponent = (components: SceneComponent[]): TextComponent => ({
  type: "text",
  id: nextId(components, "text"),
  enabled: false,
  content: "{char}",
  position: { x: 0.05, y: 0.1 },
  color: { r: 0, g: 0, b: 0, a: 192 },
  fonts: [],
  font: defaultFont(42),
  align: "left",
  wrap: true,
  max_width: 0.9,
});

const createImageComponent = (components: SceneComponent[]): ImageComponent => ({
  type: "image",
  id: nextId(components, "image"),
  enabled: false,
  source: "",
  position: { x: 0.5, y: 0.5 },
  size: { width: 0.25, height: 0.25 },
  opacity: 1,
  fit: "contain",
});

const createProgressBarComponent = (
  components: SceneComponent[]
): ProgressBarComponent => ({
  type: "progress_bar",
  id: nextId(components, "progress"),
  enabled: false,
  position: { x: 0.5, y: 0.9 },
  size: { width: 0.6, height: 0.04 },
  progress: 0.5,
  background_color: { r: 255, g: 255, b: 255, a: 96 },
  fill_color: { r: 255, g: 255, b: 255, a: 224 },
  direction: "left_to_right",
  border: null,
});

const createGroupComponent = (components: SceneComponent[]): GroupComponent => ({
  type: "group",
  id: nextId(components, "group"),
  enabled: false,
  transform: {
    translation: { x: 0, y: 0 },
    scale: { x: 1, y: 1 },
    rotation: 0,
    anchor: { x: 0.5, y: 0.5 },
  },
  opacity: 1,
  children: [],
});

export const createSceneComponent = (
  type: SceneComponentType,
  idScope: SceneComponent[]
): SceneComponent => {
  switch (type) {
    case "glyph":
      return createGlyphComponent(idScope);
    case "text":
      return createTextComponent(idScope);
    case "image":
      return createImageComponent(idScope);
    case "progress_bar":
      return createProgressBarComponent(idScope);
    case "group":
      return createGroupComponent(idScope);
  }
};

export const findComponentById = (
  components: SceneComponent[],
  id: string | null
): SceneComponent | undefined => {
  if (id === null) return undefined;
  for (const component of components) {
    if (component.id === id) return component;
    if (component.type === "group") {
      const nested = findComponentById(component.children, id);
      if (nested) return nested;
    }
  }
  return undefined;
};

export const findComponentPath = (
  components: SceneComponent[],
  id: string,
  parents: SceneComponent[] = []
): SceneComponent[] | undefined => {
  for (const component of components) {
    const path = [...parents, component];
    if (component.id === id) return path;
    if (component.type === "group") {
      const nested = findComponentPath(component.children, id, path);
      if (nested) return nested;
    }
  }
  return undefined;
};

export const findParentId = (
  components: SceneComponent[],
  id: string,
  parentId: string | null = null
): string | null | undefined => {
  for (const component of components) {
    if (component.id === id) return parentId;
    if (component.type === "group") {
      const nested = findParentId(component.children, id, component.id);
      if (nested !== undefined) return nested;
    }
  }
  return undefined;
};

export const updateComponentById = (
  components: SceneComponent[],
  id: string,
  replacement: SceneComponent
): SceneComponent[] =>
  components.map((component) => {
    if (component.id === id) return replacement;
    if (component.type === "group") {
      return {
        ...component,
        children: updateComponentById(component.children, id, replacement),
      };
    }
    return component;
  });

export const removeComponentById = (
  components: SceneComponent[],
  id: string
): SceneComponent[] =>
  components
    .filter((component) => component.id !== id)
    .map((component) =>
      component.type === "group"
        ? {
            ...component,
            children: removeComponentById(component.children, id),
          }
        : component
    );

export const appendComponent = (
  components: SceneComponent[],
  targetGroupId: string | null,
  component: SceneComponent
): SceneComponent[] => {
  if (!targetGroupId) return [...components, component];

  return components.map((current) => {
    if (current.type !== "group") return current;
    if (current.id === targetGroupId) {
      return { ...current, children: [...current.children, component] };
    }
    return {
      ...current,
      children: appendComponent(current.children, targetGroupId, component),
    };
  });
};

export interface SiblingPosition {
  index: number;
  count: number;
}

export const findSiblingPosition = (
  components: SceneComponent[],
  id: string
): SiblingPosition | undefined => {
  const index = components.findIndex((component) => component.id === id);
  if (index >= 0) return { index, count: components.length };

  for (const component of components) {
    if (component.type === "group") {
      const nested = findSiblingPosition(component.children, id);
      if (nested) return nested;
    }
  }
  return undefined;
};

export const moveComponentById = (
  components: SceneComponent[],
  id: string,
  direction: -1 | 1
): SceneComponent[] => {
  const index = components.findIndex((component) => component.id === id);
  if (index >= 0) {
    const target = index + direction;
    if (target < 0 || target >= components.length) return components;
    const next = [...components];
    [next[index], next[target]] = [next[target], next[index]];
    return next;
  }

  return components.map((component) =>
    component.type === "group"
      ? {
          ...component,
          children: moveComponentById(component.children, id, direction),
        }
      : component
  );
};
