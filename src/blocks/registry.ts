import {
  ArrowRight,
  BookOpen,
  Code2,
  EyeOff,
  GitBranch,
  Image as ImageIcon,
  LogOut,
  MessageSquare,
  MonitorPlay,
  MonitorX,
  Music,
  Pause,
  PersonStanding,
  SlidersHorizontal,
  Sparkles,
  Split,
  StickyNote,
  Tag,
  UserRound,
  Volume2,
  type LucideIcon,
} from "lucide-react";
import type { Project } from "../types";

export type BlockCategory = "text" | "visual" | "flow" | "sound" | "data" | "screens" | "code";

export type FieldType =
  | "text"
  | "textarea"
  | "number"
  | "checkbox"
  | "select"
  | "color"
  | "localized"
  | "localizedTextarea"
  | "character"
  | "scene"
  | "asset"
  | "variable"
  | "expression"
  | "menuOptions"
  | "code"
  | "setValue";

export interface FieldDef {
  key: string;
  type: FieldType;
  labelKey: string;
  hintKey?: string;
  options?: { value: string; labelKey: string }[];
  assetKind?: "image" | "audio";
  placeholder?: string;
  min?: number;
  max?: number;
  step?: number;
  showWhen?: (params: Record<string, unknown>) => boolean;
}

export interface BlockDef {
  kind: string;
  labelKey: string;
  icon: LucideIcon;
  category: BlockCategory;
  fields: FieldDef[];
}

export const TRANSITIONS: { value: string; labelKey: string }[] = [
  { value: "", labelKey: "transition.none" },
  { value: "dissolve", labelKey: "transition.dissolve" },
  { value: "fade", labelKey: "transition.fade" },
  { value: "move", labelKey: "transition.move" },
  { value: "pixellate", labelKey: "transition.pixellate" },
  { value: "wipeleft", labelKey: "transition.wipeleft" },
  { value: "wiperight", labelKey: "transition.wiperight" },
  { value: "wipeup", labelKey: "transition.wipeup" },
  { value: "wipedown", labelKey: "transition.wipedown" },
  { value: "irisin", labelKey: "transition.irisin" },
  { value: "irisout", labelKey: "transition.irisout" },
  { value: "hpunch", labelKey: "transition.hpunch" },
  { value: "flash", labelKey: "transition.flash" },
];

export const POSITIONS: { value: string; labelKey: string }[] = [
  { value: "auto", labelKey: "position.auto" },
  { value: "left", labelKey: "position.left" },
  { value: "center", labelKey: "position.center" },
  { value: "right", labelKey: "position.right" },
  { value: "far_left", labelKey: "position.far_left" },
  { value: "far_right", labelKey: "position.far_right" },
  { value: "custom", labelKey: "position.custom" },
];

export const ANIM_PRESETS: { value: string; labelKey: string }[] = [
  { value: "shake", labelKey: "anim.shake" },
  { value: "bounce", labelKey: "anim.bounce" },
  { value: "flash", labelKey: "anim.flash" },
  { value: "slide_left", labelKey: "anim.slide_left" },
  { value: "slide_right", labelKey: "anim.slide_right" },
  { value: "zoom", labelKey: "anim.zoom" },
];

export const MUSIC_ACTIONS: { value: string; labelKey: string }[] = [
  { value: "play", labelKey: "music.play" },
  { value: "stop", labelKey: "music.stop" },
  { value: "queue", labelKey: "music.queue" },
];

export const SET_OPS: { value: string; labelKey: string }[] = [
  { value: "set", labelKey: "op.set" },
  { value: "add", labelKey: "op.add" },
  { value: "sub", labelKey: "op.sub" },
  { value: "mul", labelKey: "op.mul" },
];

export const CONDITION_OPS: { value: string; labelKey: string }[] = [
  { value: "eq", labelKey: "op.eq" },
  { value: "ne", labelKey: "op.ne" },
  { value: "gt", labelKey: "op.gt" },
  { value: "gte", labelKey: "op.gte" },
  { value: "lt", labelKey: "op.lt" },
  { value: "lte", labelKey: "op.lte" },
  { value: "is_true", labelKey: "op.is_true" },
  { value: "is_false", labelKey: "op.is_false" },
];

export const BLOCKS: BlockDef[] = [
  {
    kind: "say",
    labelKey: "block.say",
    icon: MessageSquare,
    category: "text",
    fields: [
      { key: "who", type: "character", labelKey: "field.who" },
      { key: "text", type: "localizedTextarea", labelKey: "field.text" },
      { key: "image", type: "text", labelKey: "field.image", placeholder: "happy" },
      { key: "voice", type: "asset", assetKind: "audio", labelKey: "field.voice" },
      { key: "cps", type: "number", labelKey: "field.cps", min: 0, max: 200, step: 5 },
    ],
  },
  {
    kind: "narrate",
    labelKey: "block.narrate",
    icon: BookOpen,
    category: "text",
    fields: [{ key: "text", type: "localizedTextarea", labelKey: "field.text" }],
  },
  {
    kind: "comment",
    labelKey: "block.comment",
    icon: StickyNote,
    category: "text",
    fields: [
      { key: "text", type: "localizedTextarea", labelKey: "field.text" },
      { key: "internalOnly", type: "checkbox", labelKey: "field.internalOnly" },
    ],
  },
  {
    kind: "scene",
    labelKey: "block.scene",
    icon: ImageIcon,
    category: "visual",
    fields: [
      { key: "bg", type: "asset", assetKind: "image", labelKey: "field.bg" },
      { key: "transition", type: "select", labelKey: "field.transition", options: TRANSITIONS, hintKey: "field.hint.transition" },
      { key: "duration", type: "number", labelKey: "field.duration", min: 0, max: 10, step: 0.1 },
    ],
  },
  {
    kind: "show",
    labelKey: "block.show",
    icon: UserRound,
    category: "visual",
    fields: [
      { key: "who", type: "character", labelKey: "field.who" },
      { key: "image", type: "text", labelKey: "field.image", placeholder: "happy" },
      { key: "asset", type: "asset", assetKind: "image", labelKey: "field.asset" },
      { key: "position", type: "select", labelKey: "field.position", options: POSITIONS },
      { key: "customX", type: "number", labelKey: "field.customX", min: 0, max: 1, step: 0.01, showWhen: (p) => p.position === "custom" },
      { key: "flip", type: "checkbox", labelKey: "field.flip" },
      { key: "transition", type: "select", labelKey: "field.transition", options: TRANSITIONS },
    ],
  },
  {
    kind: "hide",
    labelKey: "block.hide",
    icon: EyeOff,
    category: "visual",
    fields: [
      { key: "who", type: "character", labelKey: "field.who" },
      { key: "image", type: "text", labelKey: "field.image" },
      { key: "transition", type: "select", labelKey: "field.transition", options: TRANSITIONS },
    ],
  },
  {
    kind: "pose",
    labelKey: "block.pose",
    icon: PersonStanding,
    category: "visual",
    fields: [
      { key: "who", type: "character", labelKey: "field.who" },
      { key: "image", type: "text", labelKey: "field.image", placeholder: "sad" },
      { key: "asset", type: "asset", assetKind: "image", labelKey: "field.asset" },
      { key: "transition", type: "select", labelKey: "field.transition", options: TRANSITIONS },
    ],
  },
  {
    kind: "anim",
    labelKey: "block.anim",
    icon: Sparkles,
    category: "visual",
    fields: [
      { key: "who", type: "character", labelKey: "field.who" },
      { key: "preset", type: "select", labelKey: "field.preset", options: ANIM_PRESETS },
    ],
  },
  {
    kind: "menu",
    labelKey: "block.menu",
    icon: Split,
    category: "flow",
    fields: [
      { key: "prompt", type: "localized", labelKey: "field.prompt" },
      { key: "options", type: "menuOptions", labelKey: "field.options" },
    ],
  },
  {
    kind: "jump",
    labelKey: "block.jump",
    icon: ArrowRight,
    category: "flow",
    fields: [
      { key: "target", type: "scene", labelKey: "field.target", hintKey: "field.hint.target" },
      { key: "transition", type: "select", labelKey: "field.transition", options: TRANSITIONS },
    ],
  },
  {
    kind: "return",
    labelKey: "block.return",
    icon: LogOut,
    category: "flow",
    fields: [{ key: "toMainMenu", type: "checkbox", labelKey: "field.toMainMenu" }],
  },
  {
    kind: "if",
    labelKey: "block.if",
    icon: GitBranch,
    category: "flow",
    fields: [
      {
        key: "mode",
        type: "select",
        labelKey: "field.mode",
        options: [
          { value: "builder", labelKey: "mode.builder" },
          { value: "expression", labelKey: "mode.expression" },
        ],
      },
      { key: "variable", type: "variable", labelKey: "field.variable", showWhen: (p) => p.mode !== "expression" },
      { key: "op", type: "select", labelKey: "field.op", options: CONDITION_OPS, showWhen: (p) => p.mode !== "expression" },
      { key: "value", type: "setValue", labelKey: "field.value", showWhen: (p) => p.mode !== "expression" && p.op !== "is_true" && p.op !== "is_false" },
      { key: "expression", type: "expression", labelKey: "field.expression", hintKey: "field.hint.expression", showWhen: (p) => p.mode === "expression" },
      { key: "thenTarget", type: "scene", labelKey: "field.thenTarget" },
      { key: "hasElse", type: "checkbox", labelKey: "field.hasElse" },
      { key: "elseTarget", type: "scene", labelKey: "field.elseTarget", showWhen: (p) => Boolean(p.hasElse) },
    ],
  },
  {
    kind: "pause",
    labelKey: "block.pause",
    icon: Pause,
    category: "flow",
    fields: [
      { key: "seconds", type: "number", labelKey: "field.seconds", min: 0, max: 60, step: 0.1 },
      { key: "hard", type: "checkbox", labelKey: "field.hard" },
    ],
  },
  {
    kind: "label",
    labelKey: "block.label",
    icon: Tag,
    category: "flow",
    fields: [{ key: "name", type: "text", labelKey: "field.name" }],
  },
  {
    kind: "music",
    labelKey: "block.music",
    icon: Music,
    category: "sound",
    fields: [
      { key: "action", type: "select", labelKey: "field.action", options: MUSIC_ACTIONS },
      { key: "file", type: "asset", assetKind: "audio", labelKey: "field.file", showWhen: (p) => p.action !== "stop" },
      { key: "fadeout", type: "number", labelKey: "field.fadeout", min: 0, max: 30, step: 0.5 },
      { key: "loop", type: "checkbox", labelKey: "field.loop", showWhen: (p) => p.action === "play" },
      { key: "channel", type: "text", labelKey: "field.channel", placeholder: "music" },
    ],
  },
  {
    kind: "sound",
    labelKey: "block.sound",
    icon: Volume2,
    category: "sound",
    fields: [
      { key: "file", type: "asset", assetKind: "audio", labelKey: "field.file" },
      { key: "loop", type: "checkbox", labelKey: "field.loop" },
    ],
  },
  {
    kind: "set",
    labelKey: "block.set",
    icon: SlidersHorizontal,
    category: "data",
    fields: [
      { key: "variable", type: "variable", labelKey: "field.variable" },
      { key: "op", type: "select", labelKey: "field.op", options: SET_OPS },
      { key: "value", type: "setValue", labelKey: "field.value" },
    ],
  },
  {
    kind: "show_screen",
    labelKey: "block.show_screen",
    icon: MonitorPlay,
    category: "screens",
    fields: [
      { key: "screen", type: "text", labelKey: "field.screen" },
      { key: "withNone", type: "checkbox", labelKey: "field.withNone" },
    ],
  },
  {
    kind: "hide_screen",
    labelKey: "block.hide_screen",
    icon: MonitorX,
    category: "screens",
    fields: [{ key: "screen", type: "text", labelKey: "field.screen" }],
  },
  {
    kind: "code",
    labelKey: "block.code",
    icon: Code2,
    category: "code",
    fields: [
      {
        key: "language",
        type: "select",
        labelKey: "field.language",
        options: [
          { value: "renpy", labelKey: "lang.renpy" },
          { value: "python", labelKey: "lang.python" },
        ],
      },
      { key: "code", type: "code", labelKey: "field.code" },
    ],
  },
];

export const blockDef = (kind: string): BlockDef | undefined => BLOCKS.find((b) => b.kind === kind);

export const CATEGORY_ORDER: BlockCategory[] = ["text", "visual", "flow", "sound", "data", "screens", "code"];

export function localeText(
  value: unknown,
  locale: string,
  fallback: string,
): string {
  if (!value || typeof value !== "object") return "";
  const map = value as Record<string, string>;
  return map[locale] ?? map[fallback] ?? Object.values(map)[0] ?? "";
}

/** One-line description shown on a block card. */
export function blockSummary(
  kind: string,
  params: Record<string, unknown>,
  project: Project | null,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string {
  const locale = project?.meta.defaultLocale ?? "ru";
  const fallback = project?.meta.defaultLocale ?? "ru";
  const text = (key: string) => localeText(params[key], locale, fallback);
  const charName = (id: unknown): string => {
    if (!id || typeof id !== "string") return t("characters.kind.narrator");
    return project?.characters.find((c) => c.id === id)?.name ?? "…";
  };
  const sceneName = (id: unknown): string => {
    if (!id || typeof id !== "string") return "—";
    return project?.scenes.find((s) => s.id === id)?.title ?? id;
  };
  switch (kind) {
    case "say": {
      const who = charName(params.who);
      const body = text("text");
      return body ? `${who}: «${body}»` : who;
    }
    case "narrate":
      return text("text");
    case "comment":
      return text("text");
    case "scene": {
      const bg = String(params.bg ?? "");
      const name = bg.split("/").pop()?.replace(/\.[^.]+$/, "").replace(/[_-]+/g, " ") ?? "";
      return name || t("summary.empty");
    }
    case "show":
    case "pose": {
      const who = charName(params.who);
      const image = params.image ? ` · ${String(params.image)}` : "";
      return `${who}${image}`;
    }
    case "hide":
      return charName(params.who);
    case "anim":
      return `${charName(params.who)} · ${t(`anim.${String(params.preset ?? "shake")}`)}`;
    case "menu": {
      const options = Array.isArray(params.options) ? params.options.length : 0;
      const prompt = text("prompt");
      return prompt || t("summary.menu", { count: options });
    }
    case "jump":
      return t("summary.jump", { target: sceneName(params.target) });
    case "return":
      return params.toMainMenu ? t("field.toMainMenu") : t("summary.return");
    case "if": {
      if (params.mode === "expression") return String(params.expression ?? "…");
      const variable = String(params.variable ?? "…");
      const op = t(`op.${String(params.op ?? "gt")}`);
      return `${variable} ${op}`;
    }
    case "pause":
      return Number(params.seconds) > 0 ? `${params.seconds}s` : t("summary.click");
    case "label":
      return String(params.name ?? "");
    case "music":
    case "sound": {
      const file = String(params.file ?? "");
      return file.split("/").pop() ?? file;
    }
    case "set": {
      const value = params.value;
      return `${String(params.variable ?? "…")} ${t(`op.${String(params.op ?? "set")}`)} ${String(value ?? "")}`;
    }
    case "show_screen":
    case "hide_screen":
      return String(params.screen ?? "");
    case "code": {
      const lines = String(params.code ?? "").split("\n").length;
      return t("summary.code", { lines });
    }
    default:
      return "";
  }
}
