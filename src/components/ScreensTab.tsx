import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Image as ImageIcon,
  MousePointerClick,
  Music,
  Play,
  Plus,
  Sparkles,
  Trash2,
  Type as TypeIcon,
} from "lucide-react";
import { useStudio } from "../store";
import { api } from "../api";
import { Button, Checkbox, Field, IconButton, Input, Segmented, Select } from "./ui";
import { useDebouncedValue } from "../hooks";
import { assetUrl, clamp, newId } from "../util";
import type {
  MainMenuConfig,
  Project,
  ProjectOptions,
  ScreenDef,
  ScreenElement,
  ThemeConfig,
  UiConfig,
} from "../types";

export function AssetSelect({
  kind,
  value,
  onChange,
  allowEmpty = true,
}: {
  kind: "image" | "audio" | "font";
  value: string | null;
  onChange: (value: string | null) => void;
  allowEmpty?: boolean;
}) {
  const { t } = useTranslation();
  const assets = useStudio((s) => s.assets);
  const list = useMemo(() => assets.filter((a) => a.kind === kind), [assets, kind]);
  const current = value ?? "";

  return (
    <div className="flex gap-2">
      {kind === "image" && current ? (
        <img
          src={assetUrl(current)}
          alt=""
          onError={(event) => {
            event.currentTarget.style.display = "none";
          }}
          className="h-[34px] w-[52px] flex-none rounded-md border border-[var(--border)] object-cover"
        />
      ) : null}
      <select className="select" value={current} onChange={(event) => onChange(event.target.value || null)}>
        {allowEmpty ? <option value="">—</option> : null}
        {list.map((asset) => (
          <option key={asset.path} value={asset.path}>
            {asset.path}
          </option>
        ))}
        {current && !list.some((asset) => asset.path === current) ? (
          <option value={current}>{t("assets.notInLibrary", { path: current })}</option>
        ) : null}
      </select>
    </div>
  );
}

const TEMPLATES = [
  { id: "aurora", accent: "#8b6cf2", bg: "#12102a", labelKey: "screens.template.aurora" },
  { id: "midnight", accent: "#55a8f5", bg: "#0a1424", labelKey: "screens.template.midnight" },
  { id: "paper", accent: "#b06a3b", bg: "#efe7d8", labelKey: "screens.template.paper" },
  { id: "classic", accent: "#8b6cf2", bg: "#000000", labelKey: "screens.template.classic" },
];

const TRANSITIONS = [
  "none",
  "dissolve",
  "fade",
  "move",
  "pixellate",
  "wipeleft",
  "wiperight",
  "wipeup",
  "wipedown",
  "irisin",
  "irisout",
  "hpunch",
  "flash",
];

const ACTIONS = ["jump", "return", "start", "load", "save", "preferences", "about", "quit", "show", "hide", "none"];

type TemplateField =
  | { key: string; type: "text" | "number" | "checkbox"; labelKey: string }
  | { key: string; type: "select"; labelKey: string; options: { value: string; labelKey: string }[] }
  | {
      key: string;
      type: "list";
      labelKey: string;
      item: { key: string; type: "text" | "image" | "audio"; labelKey: string }[];
      defaults: Record<string, unknown>;
    };

interface ScreenTemplate {
  id: string;
  labelKey: string;
  hintKey: string;
  name: string;
  fields: TemplateField[];
  defaults: Record<string, unknown>;
}

const SCREEN_TEMPLATES: ScreenTemplate[] = [
  {
    id: "save_load",
    labelKey: "tpl.save_load",
    hintKey: "tpl.save_load.hint",
    name: "save_menu",
    defaults: { mode: "save", title: "", columns: 3, rows: 2, thumbnails: true, deleteKey: true },
    fields: [
      {
        key: "mode",
        type: "select",
        labelKey: "tpl.mode",
        options: [
          { value: "save", labelKey: "gui.action.save" },
          { value: "load", labelKey: "gui.action.load" },
        ],
      },
      { key: "title", type: "text", labelKey: "field.title" },
      { key: "columns", type: "number", labelKey: "gui.cols" },
      { key: "rows", type: "number", labelKey: "gui.rows" },
      { key: "thumbnails", type: "checkbox", labelKey: "tpl.thumbnails" },
      { key: "deleteKey", type: "checkbox", labelKey: "tpl.deleteKey" },
    ],
  },
  {
    id: "preferences",
    labelKey: "tpl.preferences",
    hintKey: "tpl.preferences.hint",
    name: "studio_preferences",
    defaults: { title: "", showLanguage: true },
    fields: [
      { key: "title", type: "text", labelKey: "field.title" },
      { key: "showLanguage", type: "checkbox", labelKey: "tpl.showLanguage" },
    ],
  },
  {
    id: "gallery",
    labelKey: "tpl.gallery",
    hintKey: "tpl.gallery.hint",
    name: "studio_gallery",
    defaults: { title: "", columns: 3, items: [] },
    fields: [
      { key: "title", type: "text", labelKey: "field.title" },
      { key: "columns", type: "number", labelKey: "gui.cols" },
      {
        key: "items",
        type: "list",
        labelKey: "tpl.items",
        defaults: { image: null, title: "" },
        item: [
          { key: "image", type: "image", labelKey: "field.asset" },
          { key: "title", type: "text", labelKey: "field.title" },
        ],
      },
    ],
  },
  {
    id: "music_room",
    labelKey: "tpl.music_room",
    hintKey: "tpl.music_room.hint",
    name: "studio_music",
    defaults: { title: "", tracks: [] },
    fields: [
      { key: "title", type: "text", labelKey: "field.title" },
      {
        key: "tracks",
        type: "list",
        labelKey: "tpl.tracks",
        defaults: { file: null, title: "" },
        item: [
          { key: "file", type: "audio", labelKey: "field.file" },
          { key: "title", type: "text", labelKey: "field.title" },
        ],
      },
    ],
  },
  {
    id: "chapter_select",
    labelKey: "tpl.chapter_select",
    hintKey: "tpl.chapter_select.hint",
    name: "studio_chapters",
    defaults: { title: "", chapters: [] },
    fields: [
      { key: "title", type: "text", labelKey: "field.title" },
      {
        key: "chapters",
        type: "list",
        labelKey: "tpl.chapters",
        defaults: { title: "", target: "" },
        item: [
          { key: "title", type: "text", labelKey: "field.title" },
          { key: "target", type: "text", labelKey: "field.target" },
        ],
      },
    ],
  },
  {
    id: "notification",
    labelKey: "tpl.notification",
    hintKey: "tpl.notification.hint",
    name: "studio_notification",
    defaults: { seconds: 3, position: "top-right" },
    fields: [
      { key: "seconds", type: "number", labelKey: "field.seconds" },
      {
        key: "position",
        type: "select",
        labelKey: "tpl.position",
        options: [
          { value: "top-right", labelKey: "tpl.pos.top_right" },
          { value: "top-left", labelKey: "tpl.pos.top_left" },
          { value: "bottom-right", labelKey: "tpl.pos.bottom_right" },
          { value: "bottom-left", labelKey: "tpl.pos.bottom_left" },
        ],
      },
    ],
  },
  {
    id: "hud",
    labelKey: "tpl.hud",
    hintKey: "tpl.hud.hint",
    name: "studio_hud",
    defaults: { title: "", variable: "", label: "" },
    fields: [
      { key: "title", type: "text", labelKey: "field.title" },
      { key: "variable", type: "text", labelKey: "field.variable" },
      { key: "label", type: "text", labelKey: "tpl.label" },
    ],
  },
  {
    id: "confirm",
    labelKey: "tpl.confirm",
    hintKey: "tpl.confirm.hint",
    name: "studio_confirm",
    defaults: { yes: "Да", no: "Нет" },
    fields: [
      { key: "yes", type: "text", labelKey: "tpl.yes" },
      { key: "no", type: "text", labelKey: "tpl.no" },
    ],
  },
  {
    id: "stats",
    labelKey: "tpl.stats",
    hintKey: "tpl.stats.hint",
    name: "studio_stats",
    defaults: { title: "", stats: [] },
    fields: [
      { key: "title", type: "text", labelKey: "field.title" },
      {
        key: "stats",
        type: "list",
        labelKey: "tpl.stats.items",
        defaults: { label: "", variable: "" },
        item: [
          { key: "label", type: "text", labelKey: "tpl.label" },
          { key: "variable", type: "text", labelKey: "field.variable" },
        ],
      },
    ],
  },
  {
    id: "journal",
    labelKey: "tpl.journal",
    hintKey: "tpl.journal.hint",
    name: "studio_journal",
    defaults: { title: "", entries: [] },
    fields: [
      { key: "title", type: "text", labelKey: "field.title" },
      {
        key: "entries",
        type: "list",
        labelKey: "tpl.entries",
        defaults: { title: "", text: "" },
        item: [
          { key: "title", type: "text", labelKey: "field.title" },
          { key: "text", type: "text", labelKey: "field.text" },
        ],
      },
    ],
  },
  {
    id: "inventory",
    labelKey: "tpl.inventory",
    hintKey: "tpl.inventory.hint",
    name: "studio_inventory",
    defaults: { title: "", columns: 4, items: [] },
    fields: [
      { key: "title", type: "text", labelKey: "field.title" },
      { key: "columns", type: "number", labelKey: "gui.cols" },
      {
        key: "items",
        type: "list",
        labelKey: "tpl.items",
        defaults: { image: null, title: "" },
        item: [
          { key: "image", type: "image", labelKey: "field.asset" },
          { key: "title", type: "text", labelKey: "field.title" },
        ],
      },
    ],
  },
  {
    id: "phone",
    labelKey: "tpl.phone",
    hintKey: "tpl.phone.hint",
    name: "studio_phone",
    defaults: { title: "", messages: [] },
    fields: [
      { key: "title", type: "text", labelKey: "field.title" },
      {
        key: "messages",
        type: "list",
        labelKey: "tpl.messages",
        defaults: { who: "", text: "" },
        item: [
          { key: "who", type: "text", labelKey: "tpl.who" },
          { key: "text", type: "text", labelKey: "field.text" },
        ],
      },
    ],
  },
];

function templateScreen(template: ScreenTemplate, index: number): ScreenDef {
  return {
    id: newId(),
    name: `${template.name}_${index}`,
    template: template.id,
    title: { ru: template.name },
    elements: [],
    params: { ...template.defaults },
    background: null,
    backgroundColor: "#0b0a12",
    modal: true,
    tagMenu: true,
  };
}

const defaultTheme: ThemeConfig = {
  enabled: false,
  width: 1280,
  height: 720,
  accentColor: "#00b8c3",
  idleColor: "#888888",
  hoverColor: "#00b8c3",
  textColor: "#ffffff",
  interfaceTextColor: "#ffffff",
  selectedColor: "#ffffff",
  mutedColor: "#aaaaaa",
  textFont: null,
  nameFont: null,
  interfaceFont: null,
  mainMenuBackground: null,
  gameMenuBackground: null,
  textboxHeight: null,
  dialogueWidth: null,
  fileSlotCols: 3,
  fileSlotRows: 2,
};

const defaultOptions: ProjectOptions = {
  enabled: false,
  configName: "",
  version: "",
  windowIcon: null,
  enterTransition: null,
  exitTransition: null,
  intTransition: null,
  afterLoadTransition: null,
  endGameTransition: null,
  rollbackEnabled: true,
  allowSkipping: true,
  hasSound: true,
  hasMusic: true,
  hasVoice: true,
  saveDirectory: null,
};

function blankScreen(name: string, title: string): ScreenDef {
  return {
    id: newId(),
    name,
    template: "blank",
    title: { ru: title },
    elements: [
      {
        id: newId(),
        kind: "text",
        text: { ru: title },
        image: null,
        action: null,
        target: null,
        x: 0.08,
        y: 0.1,
        w: 0.6,
        h: 0,
        fontSize: 40,
        color: "#ffffff",
        align: "left",
        style: "",
      },
      {
        id: newId(),
        kind: "button",
        text: { ru: "Назад" },
        image: null,
        action: "return",
        target: null,
        x: 0.08,
        y: 0.82,
        w: 0.16,
        h: 0.07,
        fontSize: 26,
        color: "#ffffff",
        align: "left",
        style: "",
      },
    ],
    background: null,
    backgroundColor: "#0b0a12",
    modal: true,
    tagMenu: false,
  };
}

function newElement(kind: ScreenElement["kind"], locale: string): ScreenElement {
  return {
    id: newId(),
    kind,
    text: { [locale]: kind === "button" ? "Кнопка" : "Текст" },
    image: null,
    action: kind === "button" ? "return" : null,
    target: null,
    x: 0.3,
    y: 0.45,
    w: kind === "button" ? 0.22 : 0.4,
    h: kind === "button" ? 0.08 : 0,
    fontSize: kind === "text" ? 34 : 26,
    color: "#ffffff",
    align: "left",
    style: "",
  };
}

export function ScreensTab() {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const saveUi = useStudio((s) => s.saveUi);
  const [section, setSection] = useState<"menu" | "theme" | "screens" | "options">("menu");
  const [draft, setDraft] = useDebouncedValue<UiConfig | null>("ui", project?.ui ?? null, (next) => {
    if (next) void saveUi(next);
  });

  if (!project || !draft) {
    return <div className="flex flex-1 items-center justify-center text-[12px] text-[var(--muted)]">—</div>;
  }

  const ui: UiConfig = {
    ...draft,
    theme: draft.theme ?? defaultTheme,
    options: draft.options ?? defaultOptions,
    screens: draft.screens ?? [],
  };
  const onChange = (next: UiConfig | ((prev: UiConfig) => UiConfig)) => {
    setDraft((prev) => (typeof next === "function" ? next(prev ?? ui) : next));
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-[52px] flex-none items-center justify-between border-b border-[var(--border)] px-5">
        <div className="flex items-center gap-3">
          <h2 className="text-[14px] font-semibold">{t("screens.title")}</h2>
          <Segmented
            value={section}
            onChange={setSection}
            options={[
              { value: "menu", label: t("gui.section.menu") },
              { value: "theme", label: t("gui.section.theme") },
              { value: "screens", label: t("gui.section.screens") },
              { value: "options", label: t("gui.section.options") },
            ]}
          />
        </div>
        <span className="text-[11px] text-[var(--muted-2)]">{t("screens.saved")}</span>
      </div>

      <div className="scroll flex-1">
        {section === "menu" ? (
          <MainMenuSection ui={ui} onChange={onChange} project={project} />
        ) : section === "theme" ? (
          <ThemeSection ui={ui} onChange={onChange} project={project} />
        ) : section === "options" ? (
          <OptionsSection ui={ui} onChange={onChange} />
        ) : (
          <ScreensSection ui={ui} onChange={onChange} project={project} />
        )}
      </div>
    </div>
  );
}

function MainMenuSection({
  ui,
  onChange,
  project,
}: {
  ui: UiConfig;
  onChange: (ui: UiConfig) => void;
  project: Project;
}) {
  const { t } = useTranslation();
  const editLocale = useStudio((s) => s.editLocale);
  const cfg = ui.mainMenu;
  const patch = (partial: Partial<MainMenuConfig>) =>
    onChange({ ...ui, mainMenu: { ...cfg, ...partial } });
  const patchLocalized = (key: "title" | "tagline", value: string) =>
    patch({ [key]: { ...(cfg[key] ?? {}), [editLocale]: value } } as Partial<MainMenuConfig>);
  const localizedValue = (key: "title" | "tagline") => (cfg[key]?.[editLocale] ?? "");
  const accent = cfg.accent || "#8b6cf2";

  return (
    <div className="grid grid-cols-[minmax(0,1fr)_420px] gap-6 px-6 py-5">
      <div>
        <div className="mb-5">
          <div className="label">{t("screens.template")}</div>
          <div className="grid grid-cols-4 gap-2">
            {TEMPLATES.map((template) => (
              <button
                key={template.id}
                type="button"
                onClick={() => patch({ template: template.id, accent: template.accent })}
                className={`overflow-hidden rounded-lg border text-left transition-all ${
                  cfg.template === template.id
                    ? "border-[var(--accent)] shadow-[0_0_0_2px_var(--accent-soft)]"
                    : "border-[var(--border)] hover:border-[var(--muted-2)]"
                }`}
              >
                <div
                  className="flex h-16 flex-col items-center justify-center gap-1"
                  style={{ background: template.bg }}
                >
                  <div className="h-1 w-10 rounded" style={{ background: template.accent }} />
                  <div className="h-1 w-7 rounded bg-white/25" />
                  <div className="h-1 w-8 rounded bg-white/15" />
                </div>
                <div className="px-2 py-1 text-[11px] text-[var(--muted)]">{t(template.labelKey)}</div>
              </button>
            ))}
          </div>
        </div>

        {cfg.template === "classic" ? (
          <div className="mb-4 flex items-start gap-2 rounded-xl border border-[var(--border)] bg-[var(--panel-2)] p-3 text-[12px] text-[var(--muted)]">
            <Sparkles size={14} className="mt-0.5 text-[var(--accent)]" />
            {t("screens.classicHint")}
          </div>
        ) : null}

        <div className="grid grid-cols-2 gap-x-4">
          <Field label={`${t("screens.titleText")} · ${editLocale.toUpperCase()}`}>
            <Input value={localizedValue("title")} onChange={(event) => patchLocalized("title", event.target.value)} />
          </Field>
          <Field label={`${t("screens.tagline")} · ${editLocale.toUpperCase()}`}>
            <Input value={localizedValue("tagline")} onChange={(event) => patchLocalized("tagline", event.target.value)} />
          </Field>
          <Field label={t("screens.background")}>
            <AssetSelect kind="image" value={cfg.background} onChange={(value) => patch({ background: value })} />
          </Field>
          <Field label={t("screens.logo")}>
            <AssetSelect kind="image" value={cfg.logo} onChange={(value) => patch({ logo: value })} />
          </Field>
          <Field label={t("screens.music")}>
            <AssetSelect kind="audio" value={cfg.music} onChange={(value) => patch({ music: value })} />
          </Field>
          <Field label={t("screens.align")}>
            <Segmented
              value={cfg.buttonAlign}
              onChange={(value) => patch({ buttonAlign: value })}
              options={[
                { value: "left", label: t("screens.align.left") },
                { value: "center", label: t("screens.align.center") },
                { value: "right", label: t("screens.align.right") },
              ]}
            />
          </Field>
          <Field label={t("screens.accent")}>
            <div className="flex gap-2">
              <input
                type="color"
                value={accent}
                onChange={(event) => patch({ accent: event.target.value })}
                className="h-[32px] w-[46px] cursor-pointer rounded-md border border-[var(--border)] bg-transparent"
              />
              <Input className="mono text-[12px]" value={cfg.accent} onChange={(event) => patch({ accent: event.target.value })} />
            </div>
          </Field>
          <Field label={t("screens.textColor")}>
            <div className="flex gap-2">
              <input
                type="color"
                value={cfg.textColor}
                onChange={(event) => patch({ textColor: event.target.value })}
                className="h-[32px] w-[46px] cursor-pointer rounded-md border border-[var(--border)] bg-transparent"
              />
              <Input className="mono text-[12px]" value={cfg.textColor} onChange={(event) => patch({ textColor: event.target.value })} />
            </div>
          </Field>
        </div>

        <div className="mb-2 mt-2">
          <div className="label">{t("screens.buttons")}</div>
          <div className="grid grid-cols-2 gap-x-4">
            <Checkbox checked={cfg.showStart} label={t("screens.btn.start")} onChange={(v) => patch({ showStart: v })} />
            <Checkbox checked={cfg.showLoad} label={t("screens.btn.load")} onChange={(v) => patch({ showLoad: v })} />
            <Checkbox checked={cfg.showPreferences} label={t("screens.btn.preferences")} onChange={(v) => patch({ showPreferences: v })} />
            <Checkbox checked={cfg.showAbout} label={t("screens.btn.about")} onChange={(v) => patch({ showAbout: v })} />
            <Checkbox checked={cfg.showQuit} label={t("screens.btn.quit")} onChange={(v) => patch({ showQuit: v })} />
          </div>
        </div>

        <Field label={t("field.editLocale")}>
          <div className="flex items-center gap-2 text-[11px] text-[var(--muted-2)]">
            {project.locales.map((locale) => (
              <button
                key={locale}
                type="button"
                onClick={() => useStudio.getState().setEditLocale(locale)}
                className={`rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase ${
                  editLocale === locale
                    ? "bg-[var(--accent-soft)] text-[var(--accent)]"
                    : "hover:text-[var(--text)]"
                }`}
              >
                {locale}
              </button>
            ))}
            <span>{t("field.hint.localized")}</span>
          </div>
        </Field>
      </div>

      <div className="sticky top-0 h-fit">
        <div className="label">{t("common.preview")}</div>
        <div
          className="relative overflow-hidden rounded-xl border border-[var(--border)]"
          style={{ aspectRatio: "16 / 10", background: "#0b0a12" }}
        >
          {cfg.background ? (
            <img
              src={assetUrl(cfg.background)}
              alt=""
              onError={(event) => {
                event.currentTarget.style.display = "none";
              }}
              className="absolute inset-0 h-full w-full object-cover"
            />
          ) : (
            <div
              className="absolute inset-0"
              style={{
                background: `radial-gradient(120% 120% at 50% 0%, ${accent}55 0%, #0b0a12 62%)`,
              }}
            />
          )}
          <div
            className="absolute inset-0"
            style={{ background: "linear-gradient(180deg, rgba(0,0,0,0.15), rgba(0,0,0,0.55))" }}
          />
          <div
            className="absolute inset-0 flex flex-col px-6 py-5"
            style={{
              alignItems:
                cfg.buttonAlign === "left" ? "flex-start" : cfg.buttonAlign === "right" ? "flex-end" : "center",
              justifyContent: "center",
              gap: 8,
              color: cfg.textColor,
              textAlign: cfg.buttonAlign,
            }}
          >
            {cfg.logo ? <img src={assetUrl(cfg.logo)} alt="" className="mb-1 max-h-14 object-contain" /> : null}
            <div className="text-[26px] font-bold leading-tight" style={{ textShadow: "0 2px 18px rgba(0,0,0,0.5)" }}>
              {localizedValue("title") || project.meta.title}
            </div>
            {localizedValue("tagline") ? <div className="text-[12px] opacity-80">{localizedValue("tagline")}</div> : null}
            <div className="mt-3 flex flex-col items-stretch gap-1.5" style={{ minWidth: 150 }}>
              {[
                cfg.showStart && t("screens.btn.start"),
                cfg.showLoad && t("screens.btn.load"),
                cfg.showPreferences && t("screens.btn.preferences"),
                cfg.showAbout && t("screens.btn.about"),
                cfg.showQuit && t("screens.btn.quit"),
              ]
                .filter(Boolean)
                .map((label, index) => (
                  <div
                    key={String(label) + index}
                    className="rounded-md px-3 py-1.5 text-center text-[12px] font-medium transition-colors"
                    style={{
                      background: index === 0 ? `${accent}` : "rgba(255,255,255,0.08)",
                      color: index === 0 ? "#fff" : cfg.textColor,
                      border: `1px solid ${index === 0 ? "transparent" : "rgba(255,255,255,0.14)"}`,
                    }}
                  >
                    {label}
                  </div>
                ))}
            </div>
          </div>
        </div>
        <div className="mt-3 flex items-center gap-2 text-[11px] text-[var(--muted-2)]">
          <ImageIcon size={12} /> {cfg.background ?? "gradient"}
          {cfg.music ? (
            <>
              <Music size={12} className="ml-2" /> {cfg.music.split("/").pop()}
            </>
          ) : null}
        </div>

        <div className="mt-5 rounded-xl border border-[var(--border)] bg-[var(--panel-2)] p-3">
          <div className="mb-1 text-[10px] font-semibold uppercase tracking-wider text-[var(--muted-2)]">
            {t("common.preview")} · .rpy
          </div>
          <pre className="mono max-h-[180px] overflow-auto text-[10.5px] leading-relaxed text-[var(--muted)]">
{`screen main_menu():
    tag menu
    add Solid("${accent}22")
    vbox:
        xalign ${cfg.buttonAlign === "left" ? "0.12" : cfg.buttonAlign === "right" ? "0.88" : "0.5"}
        yalign 0.62
        textbutton _("Начать") action Start()
        textbutton _("Загрузить") action ShowMenu("load")`}
          </pre>
        </div>
      </div>
    </div>
  );
}

function ThemeSection({
  ui,
  onChange,
  project,
}: {
  ui: UiConfig;
  onChange: (ui: UiConfig) => void;
  project: Project;
}) {
  const { t } = useTranslation();
  const assets = useStudio((s) => s.assets);
  const theme = ui.theme;
  const patch = (partial: Partial<ThemeConfig>) => onChange({ ...ui, theme: { ...theme, ...partial } });
  const colors: { key: keyof ThemeConfig; label: string }[] = [
    { key: "accentColor", label: t("theme.accent") },
    { key: "idleColor", label: t("theme.idle") },
    { key: "hoverColor", label: t("theme.hover") },
    { key: "textColor", label: t("theme.text") },
    { key: "interfaceTextColor", label: t("theme.interface") },
    { key: "selectedColor", label: t("theme.selected") },
    { key: "mutedColor", label: t("theme.muted") },
  ];

  return (
    <div className="max-w-[980px] px-6 py-5">
      <div className="mb-4 flex items-start gap-3 rounded-xl border border-[var(--border)] bg-[var(--panel-2)] p-3">
        <Sparkles size={16} className="mt-0.5 text-[var(--accent)]" />
        <div className="text-[12px] leading-relaxed text-[var(--muted)]">
          {t("theme.hint")}
          <div className="mt-1 text-[11px] text-[var(--muted-2)]">{t("theme.hint2")}</div>
        </div>
        <div className="flex-1" />
        <Checkbox checked={theme.enabled} label={t("theme.enabled")} onChange={(v) => patch({ enabled: v })} />
      </div>

      <div className="grid grid-cols-3 gap-x-4">
        <Field label={t("theme.width")}>
          <Select
            value={String(theme.width)}
            onChange={(event) => {
              const width = Number(event.target.value);
              const preset = [
                { w: 1280, h: 720 },
                { w: 1920, h: 1080 },
                { w: 1600, h: 1200 },
                { w: 2560, h: 1440 },
              ].find((entry) => entry.w === width);
              patch({ width, height: preset ? preset.h : theme.height });
            }}
            options={[
              { value: "1280", label: "1280 × 720" },
              { value: "1920", label: "1920 × 1080" },
              { value: "1600", label: "1600 × 1200 (4:3)" },
              { value: "2560", label: "2560 × 1440" },
            ]}
          />
        </Field>
        <Field label={t("theme.height")}>
          <Input
            type="number"
            value={String(theme.height)}
            onChange={(event) => patch({ height: Number(event.target.value) || 720 })}
          />
        </Field>
        <Field label={t("theme.scaleHint")}>
          <div className="mono rounded-md border border-[var(--border)] bg-[var(--bg-soft)] px-3 py-2 text-[12px] text-[var(--muted)]">
            gui.scale = {(theme.width / 1280).toFixed(2)}×
          </div>
        </Field>
      </div>

      <div className="label mt-2">{t("theme.palette")}</div>
      <div className="mb-4 grid grid-cols-3 gap-x-4">
        {colors.map((entry) => {
          const value = String(theme[entry.key] ?? "#ffffff");
          return (
            <Field key={String(entry.key)} label={entry.label}>
              <div className="flex gap-2">
                <input
                  type="color"
                  value={value}
                  onChange={(event) => patch({ [entry.key]: event.target.value } as Partial<ThemeConfig>)}
                  className="h-[32px] w-[46px] cursor-pointer rounded-md border border-[var(--border)] bg-transparent"
                />
                <Input
                  className="mono text-[12px]"
                  value={value}
                  onChange={(event) => patch({ [entry.key]: event.target.value } as Partial<ThemeConfig>)}
                />
              </div>
            </Field>
          );
        })}
      </div>

      <div className="label">{t("theme.fonts")}</div>
      <div className="grid grid-cols-3 gap-x-4">
        <Field label={t("theme.fontText")} hint={t("theme.fontHint")}>
          <AssetSelect kind="font" value={theme.textFont ?? null} onChange={(value) => patch({ textFont: value })} />
        </Field>
        <Field label={t("theme.fontName")}>
          <AssetSelect kind="font" value={theme.nameFont ?? null} onChange={(value) => patch({ nameFont: value })} />
        </Field>
        <Field label={t("theme.fontInterface")}>
          <AssetSelect kind="font" value={theme.interfaceFont ?? null} onChange={(value) => patch({ interfaceFont: value })} />
        </Field>
      </div>
      {assets.filter((asset) => asset.kind === "font").length === 0 ? (
        <p className="-mt-2 mb-3 text-[11px] leading-relaxed text-[var(--muted-2)]">{t("theme.noFonts")}</p>
      ) : null}

      <div className="label">{t("theme.backgrounds")}</div>
      <div className="grid grid-cols-2 gap-x-4">
        <Field label={t("theme.mainMenuBackground")}>
          <AssetSelect kind="image" value={theme.mainMenuBackground ?? null} onChange={(value) => patch({ mainMenuBackground: value })} />
        </Field>
        <Field label={t("theme.gameMenuBackground")}>
          <AssetSelect kind="image" value={theme.gameMenuBackground ?? null} onChange={(value) => patch({ gameMenuBackground: value })} />
        </Field>
      </div>

      <div className="label">{t("theme.sizes")}</div>
      <div className="grid grid-cols-4 gap-x-4">
        <Field label={t("theme.textboxHeight")} hint={t("theme.baseHint")}>
          <Input
            type="number"
            value={theme.textboxHeight == null ? "" : String(theme.textboxHeight)}
            placeholder="240"
            onChange={(event) =>
              patch({ textboxHeight: event.target.value === "" ? null : Number(event.target.value) })
            }
          />
        </Field>
        <Field label={t("theme.dialogueWidth")} hint={t("theme.baseHint")}>
          <Input
            type="number"
            value={theme.dialogueWidth == null ? "" : String(theme.dialogueWidth)}
            placeholder="1100"
            onChange={(event) =>
              patch({ dialogueWidth: event.target.value === "" ? null : Number(event.target.value) })
            }
          />
        </Field>
        <Field label={t("theme.slotCols")}>
          <Input
            type="number"
            min={1}
            max={6}
            value={String(theme.fileSlotCols)}
            onChange={(event) => patch({ fileSlotCols: clamp(Number(event.target.value) || 3, 1, 6) })}
          />
        </Field>
        <Field label={t("theme.slotRows")}>
          <Input
            type="number"
            min={1}
            max={6}
            value={String(theme.fileSlotRows)}
            onChange={(event) => patch({ fileSlotRows: clamp(Number(event.target.value) || 2, 1, 6) })}
          />
        </Field>
      </div>
    </div>
  );
}

function OptionsSection({ ui, onChange }: { ui: UiConfig; onChange: (ui: UiConfig) => void }) {
  const { t } = useTranslation();
  const options = ui.options;
  const patch = (partial: Partial<ProjectOptions>) => onChange({ ...ui, options: { ...options, ...partial } });
  const transitionField = (key: keyof ProjectOptions, label: string) => (
    <Field key={String(key)} label={label}>
      <Select
        value={String(options[key] ?? "")}
        onChange={(event) => patch({ [key]: event.target.value || null } as Partial<ProjectOptions>)}
        options={TRANSITIONS.map((name) => ({ value: name, label: t(`transition.${name}`) }))}
        placeholder={t("options.default")}
      />
    </Field>
  );

  return (
    <div className="max-w-[980px] px-6 py-5">
      <div className="mb-4 flex items-start gap-3 rounded-xl border border-[var(--border)] bg-[var(--panel-2)] p-3">
        <Sparkles size={16} className="mt-0.5 text-[var(--accent)]" />
        <div className="text-[12px] leading-relaxed text-[var(--muted)]">{t("options.hint")}</div>
        <div className="flex-1" />
        <Checkbox checked={options.enabled} label={t("options.enabled")} onChange={(v) => patch({ enabled: v })} />
      </div>

      <div className="grid grid-cols-2 gap-x-4">
        <Field label={t("options.name")} hint={t("options.nameHint")}>
          <Input value={options.configName} onChange={(event) => patch({ configName: event.target.value })} />
        </Field>
        <Field label={t("options.version")}>
          <Input value={options.version} onChange={(event) => patch({ version: event.target.value })} />
        </Field>
        <Field label={t("options.icon")}>
          <AssetSelect kind="image" value={options.windowIcon ?? null} onChange={(value) => patch({ windowIcon: value })} />
        </Field>
        <Field label={t("options.saveDir")} hint={t("options.saveDirHint")}>
          <Input value={options.saveDirectory ?? ""} onChange={(event) => patch({ saveDirectory: event.target.value || null })} />
        </Field>
      </div>

      <div className="label">{t("options.transitions")}</div>
      <div className="grid grid-cols-3 gap-x-4">
        {transitionField("enterTransition", t("options.enterTransition"))}
        {transitionField("exitTransition", t("options.exitTransition"))}
        {transitionField("intTransition", t("options.intTransition"))}
        {transitionField("afterLoadTransition", t("options.afterLoad"))}
        {transitionField("endGameTransition", t("options.endGame"))}
      </div>

      <div className="label">{t("options.engine")}</div>
      <div className="grid grid-cols-2 gap-x-4">
        <Checkbox checked={options.hasSound} label={t("options.hasSound")} onChange={(v) => patch({ hasSound: v })} />
        <Checkbox checked={options.hasMusic} label={t("options.hasMusic")} onChange={(v) => patch({ hasMusic: v })} />
        <Checkbox checked={options.hasVoice} label={t("options.hasVoice")} onChange={(v) => patch({ hasVoice: v })} />
        <Checkbox checked={options.rollbackEnabled} label={t("options.rollback")} onChange={(v) => patch({ rollbackEnabled: v })} />
        <Checkbox checked={options.allowSkipping} label={t("options.skipping")} onChange={(v) => patch({ allowSkipping: v })} />
      </div>
    </div>
  );
}

type DragState =
  | { mode: "move"; id: string; start: { x: number; y: number }; origin: ScreenElement }
  | { mode: "resize"; id: string; start: { x: number; y: number }; origin: ScreenElement };

function ScreensSection({
  ui,
  onChange,
  project,
}: {
  ui: UiConfig;
  onChange: (next: UiConfig | ((prev: UiConfig) => UiConfig)) => void;
  project: Project;
}) {
  const { t } = useTranslation();
  const editLocale = useStudio((s) => s.editLocale);
  const toast = useStudio((s) => s.toast);
  const [previewing, setPreviewing] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(ui.screens[0]?.id ?? null);
  const [selectedElementId, setSelectedElementId] = useState<string | null>(null);
  const [templatePick, setTemplatePick] = useState("");
  const [drag, setDrag] = useState<DragState | null>(null);
  const canvasRef = useRef<HTMLDivElement | null>(null);
  const [canvasWidth, setCanvasWidth] = useState(760);

  const screen = ui.screens.find((entry) => entry.id === selectedId) ?? ui.screens[0] ?? null;
  const element = screen?.elements.find((entry) => entry.id === selectedElementId) ?? null;

  useEffect(() => {
    const node = canvasRef.current;
    if (!node) return;
    const observer = new ResizeObserver(() => setCanvasWidth(node.clientWidth || 760));
    observer.observe(node);
    setCanvasWidth(node.clientWidth || 760);
    return () => observer.disconnect();
  }, [screen?.id]);

  const updateScreen = (patch: Partial<ScreenDef>) => {
    if (!screen) return;
    onChange({
      ...ui,
      screens: ui.screens.map((entry) => (entry.id === screen.id ? { ...entry, ...patch } : entry)),
    });
  };

  const updateElement = (patch: Partial<ScreenElement>) => {
    if (!screen || !element) return;
    updateScreen({
      elements: screen.elements.map((entry) => (entry.id === element.id ? { ...entry, ...patch } : entry)),
    });
  };

  const toLocal = (clientX: number, clientY: number) => {
    const box = canvasRef.current?.getBoundingClientRect();
    if (!box || box.width === 0) return { x: 0, y: 0 };
    return {
      x: clamp((clientX - box.left) / box.width, 0, 1),
      y: clamp((clientY - box.top) / box.height, 0, 1),
    };
  };

  useEffect(() => {
    if (!drag) return;
    const move = (event: PointerEvent) => {
      const point = toLocal(event.clientX, event.clientY);
      const dx = point.x - drag.start.x;
      const dy = point.y - drag.start.y;
      const origin = drag.origin;
      const patch =
        drag.mode === "move"
          ? {
              x: clamp(origin.x + dx, 0, 1),
              y: clamp(origin.y + dy, 0, 1),
            }
          : {
              w: clamp((origin.w || 0.05) + dx, 0.02, 1 - origin.x),
              h: clamp((origin.h || 0.03) + dy, 0.02, 1 - origin.y),
            };
      onChange((prev) => ({
        ...prev,
        screens: prev.screens.map((entry) =>
          entry.id === screen?.id
            ? {
                ...entry,
                elements: entry.elements.map((item) =>
                  item.id === drag.id ? { ...item, ...patch } : item,
                ),
              }
            : entry,
        ),
      }));
    };
    const up = () => setDrag(null);
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [drag, screen?.id]);

  const startDrag = (event: React.PointerEvent, item: ScreenElement, mode: "move" | "resize") => {
    event.stopPropagation();
    event.preventDefault();
    const point = toLocal(event.clientX, event.clientY);
    setSelectedElementId(item.id);
    setDrag({ mode, id: item.id, start: point, origin: { ...item } });
  };

  const addScreen = () => {
    const next = blankScreen(`screen_${ui.screens.length + 1}`, t("gui.newScreenTitle"));
    onChange({ ...ui, screens: [...ui.screens, next] });
    setSelectedId(next.id);
    setSelectedElementId(null);
  };

  const addTemplateScreen = (id: string) => {
    const spec = SCREEN_TEMPLATES.find((entry) => entry.id === id);
    if (!spec) return;
    const next = templateScreen(spec, ui.screens.length + 1);
    next.title = { [editLocale]: t(spec.labelKey) };
    onChange({ ...ui, screens: [...ui.screens, next] });
    setSelectedId(next.id);
    setSelectedElementId(null);
  };

  const previewScreen = async () => {
    if (!screen) return;
    setPreviewing(true);
    try {
      const result = await api.previewScreen(screen.name);
      if (result.ok) toast("success", t("gui.previewStarted"));
      else toast("error", result.message);
    } catch (error) {
      toast("error", String(error));
    } finally {
      setPreviewing(false);
    }
  };

  const addPreset = (preset: "hub" | "about" | "hud") => {
    const next = blankScreen(`screen_${ui.screens.length + 1}`, t("gui.newScreenTitle"));
    next.name = `${preset}_${ui.screens.length + 1}`;
    next.elements = [];
    if (preset === "hub") {
      next.title = { [editLocale]: "Меню" };
      next.elements = [
        { ...newElement("text", editLocale), text: { [editLocale]: "Заголовок" }, x: 0.08, y: 0.08, w: 0.6, fontSize: 48 },
        { ...newElement("button", editLocale), text: { [editLocale]: "Продолжить" }, x: 0.08, y: 0.42, action: "start" },
        { ...newElement("button", editLocale), text: { [editLocale]: "Настройки" }, x: 0.08, y: 0.52, action: "preferences" },
        { ...newElement("button", editLocale), text: { [editLocale]: "Назад" }, x: 0.08, y: 0.62, action: "return" },
      ];
    } else if (preset === "about") {
      next.title = { [editLocale]: "Об игре" };
      next.elements = [
        { ...newElement("text", editLocale), text: { [editLocale]: "Об игре" }, x: 0.08, y: 0.08, w: 0.6, fontSize: 44 },
        {
          ...newElement("text", editLocale),
          text: { [editLocale]: "Автор, версия, благодарности." },
          x: 0.08,
          y: 0.24,
          w: 0.7,
          fontSize: 24,
        },
        { ...newElement("button", editLocale), text: { [editLocale]: "Назад" }, x: 0.08, y: 0.8, action: "return" },
      ];
    } else {
      next.title = { [editLocale]: "HUD" };
      next.backgroundColor = "#00000000";
      next.modal = false;
      next.elements = [
        { ...newElement("text", editLocale), text: { [editLocale]: "Глава 1" }, x: 0.02, y: 0.03, w: 0.3, fontSize: 22 },
      ];
    }
    onChange({ ...ui, screens: [...ui.screens, next] });
    setSelectedId(next.id);
    setSelectedElementId(null);
  };

  const scale = canvasWidth / 1280;

  return (
    <div className="flex gap-5 px-5 py-4">
      <div className="w-[220px] flex-none">
        <div className="mb-2 flex items-center justify-between">
          <span className="text-[11px] font-semibold uppercase tracking-[0.07em] text-[var(--muted-2)]">
            {t("gui.screensCount", { count: ui.screens.length })}
          </span>
          <IconButton title={t("gui.addScreen")} onClick={addScreen}>
            <Plus size={14} />
          </IconButton>
        </div>
        <div className="space-y-1.5">
          {ui.screens.length === 0 ? (
            <p className="text-[11px] leading-relaxed text-[var(--muted-2)]">{t("gui.noScreens")}</p>
          ) : null}
          {ui.screens.map((entry) => (
            <button
              key={entry.id}
              type="button"
              onClick={() => {
                setSelectedId(entry.id);
                setSelectedElementId(null);
              }}
              className={`w-full rounded-lg border px-2.5 py-2 text-left transition-colors ${
                screen?.id === entry.id
                  ? "border-[var(--accent)] bg-[var(--accent-soft)]"
                  : "border-[var(--border)] hover:border-[var(--muted-2)]"
              }`}
            >
              <div className="text-[12px] font-medium">{entry.title?.[editLocale] || entry.name}</div>
              <div className="mono text-[10px] text-[var(--muted-2)]">
                {entry.name} · {entry.elements.length}
              </div>
            </button>
          ))}
        </div>
        <div className="mb-2 text-[10px] font-semibold uppercase tracking-[0.07em] text-[var(--muted-2)]">
          {t("tpl.pick")}
        </div>
        <Select
          className="text-[11px]"
          value={templatePick}
          onChange={(event) => {
            const id = event.target.value;
            setTemplatePick("");
            if (id) addTemplateScreen(id);
          }}
          options={SCREEN_TEMPLATES.map((entry) => ({ value: entry.id, label: t(entry.labelKey) }))}
          placeholder={t("tpl.pick")}
        />
        <Button variant="ghost" size="sm" className="mt-2 w-full" onClick={addScreen}>
          <Plus size={13} /> {t("gui.addScreen")}
        </Button>
        <div className="mt-2 grid grid-cols-3 gap-1">
          <Button variant="ghost" size="sm" onClick={() => addPreset("hub")} title={t("gui.preset.hubHint")}>
            {t("gui.preset.hub")}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => addPreset("about")} title={t("gui.preset.aboutHint")}>
            {t("gui.preset.about")}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => addPreset("hud")} title={t("gui.preset.hudHint")}>
            {t("gui.preset.hud")}
          </Button>
        </div>
      </div>

      <div className="min-w-0 flex-1">
        {screen && screen.template !== "blank" ? (
          <div className="flex h-[420px] flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-[var(--border)] bg-[var(--panel-2)] px-8 text-center">
            <Sparkles size={22} className="text-[var(--accent)]" />
            <div className="text-[14px] font-semibold">
              {t(SCREEN_TEMPLATES.find((entry) => entry.id === screen.template)?.labelKey ?? "tpl.pick")}
            </div>
            <div className="mono text-[11px] text-[var(--muted-2)]">screen {screen.name}():</div>
            <p className="max-w-[460px] text-[11px] leading-relaxed text-[var(--muted)]">
              {t(SCREEN_TEMPLATES.find((entry) => entry.id === screen.template)?.hintKey ?? "tpl.screenHint")}
            </p>
            <p className="max-w-[460px] text-[11px] leading-relaxed text-[var(--muted-2)]">{t("tpl.screenHint")}</p>
            <Button variant="primary" size="sm" disabled={previewing} onClick={() => void previewScreen()}>
              <Play size={13} /> {t("gui.preview")}
            </Button>
          </div>
        ) : screen ? (
          <>
            <div className="mb-2 flex items-center gap-1.5">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  const el = newElement("text", editLocale);
                  updateScreen({ elements: [...screen.elements, el] });
                  setSelectedElementId(el.id);
                }}
              >
                <TypeIcon size={13} /> {t("gui.addText")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  const el = newElement("button", editLocale);
                  updateScreen({ elements: [...screen.elements, el] });
                  setSelectedElementId(el.id);
                }}
              >
                <MousePointerClick size={13} /> {t("gui.addButton")}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  const el = newElement("image", editLocale);
                  el.w = 0;
                  el.h = 0;
                  updateScreen({ elements: [...screen.elements, el] });
                  setSelectedElementId(el.id);
                }}
              >
                <ImageIcon size={13} /> {t("gui.addImage")}
              </Button>
              <div className="flex-1" />
              <span className="text-[10px] text-[var(--muted-2)]">{t("gui.dragHint")}</span>
            </div>

            <div
              ref={canvasRef}
              className="relative w-full select-none overflow-hidden rounded-xl border border-[var(--border)]"
              style={{ aspectRatio: "16 / 9", background: screen.backgroundColor || "#0b0a12" }}
              onPointerDown={() => setSelectedElementId(null)}
            >
              {screen.background ? (
                <img
                  src={assetUrl(screen.background)}
                  alt=""
                  draggable={false}
                  className="pointer-events-none absolute inset-0 h-full w-full object-contain"
                />
              ) : null}
              {screen.elements.map((item) => {
                const active = item.id === selectedElementId;
                const style: React.CSSProperties = {
                  left: `${item.x * 100}%`,
                  top: `${item.y * 100}%`,
                  width: item.w > 0 ? `${item.w * 100}%` : undefined,
                  height: item.h > 0 ? `${item.h * 100}%` : undefined,
                };
                const outline = active ? "1px solid #b9a7ff" : "1px dashed rgba(139,108,242,0.55)";
                if (item.kind === "image") {
                  return (
                    <div
                      key={item.id}
                      className="absolute"
                      style={{ ...style, outline, cursor: "move" }}
                      onPointerDown={(event) => startDrag(event, item, "move")}
                    >
                      {item.image ? (
                        <img src={assetUrl(item.image)} alt="" draggable={false} className="h-full w-full object-contain" />
                      ) : (
                        <div className="flex h-full w-full items-center justify-center text-[10px] text-[var(--muted-2)]">
                          {t("gui.noImage")}
                        </div>
                      )}
                    </div>
                  );
                }
                if (item.kind === "button") {
                  return (
                    <div
                      key={item.id}
                      className="absolute flex items-center justify-center rounded-md border text-center"
                      style={{
                        ...style,
                        outline,
                        cursor: "move",
                        background: "rgba(139,108,242,0.28)",
                        borderColor: "#8b6cf2",
                        color: item.color ?? "#fff",
                        fontSize: Math.max(9, item.fontSize * scale),
                      }}
                      onPointerDown={(event) => startDrag(event, item, "move")}
                    >
                      {item.text?.[editLocale] || "…"}
                      {active ? (
                        <span
                          className="absolute -bottom-1 -right-1 h-3 w-3 cursor-se-resize rounded-sm border border-white bg-[var(--accent)]"
                          onPointerDown={(event) => startDrag(event, item, "resize")}
                        />
                      ) : null}
                    </div>
                  );
                }
                return (
                  <div
                    key={item.id}
                    className="absolute whitespace-pre-wrap"
                    style={{
                      ...style,
                      outline,
                      cursor: "move",
                      color: item.color ?? "#fff",
                      fontSize: Math.max(9, item.fontSize * scale),
                      textAlign: item.align,
                    }}
                    onPointerDown={(event) => startDrag(event, item, "move")}
                  >
                    {item.text?.[editLocale] || "…"}
                    {active ? (
                      <span
                        className="absolute -bottom-1 -right-1 h-3 w-3 cursor-se-resize rounded-sm border border-white bg-[var(--accent)]"
                        onPointerDown={(event) => startDrag(event, item, "resize")}
                      />
                    ) : null}
                  </div>
                );
              })}
            </div>

            <p className="mt-2 text-[11px] leading-relaxed text-[var(--muted-2)]">{t("gui.canvasHint")}</p>
          </>
        ) : (
          <div className="flex h-full items-center justify-center text-[12px] text-[var(--muted-2)]">
            {t("gui.noScreens")}
          </div>
        )}
      </div>

      <div className="w-[300px] flex-none">
        {screen ? (
          <>
            <div className="mb-2 flex items-center justify-between">
              <span className="text-[12px] font-semibold">{t("gui.screenSettings")}</span>
              <div className="flex items-center gap-1">
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={previewing}
                  title={t("gui.preview")}
                  onClick={() => void previewScreen()}
                >
                  <Play size={13} /> {t("gui.preview")}
                </Button>
                <IconButton
                  title={t("common.delete")}
                  onClick={() => {
                    onChange({ ...ui, screens: ui.screens.filter((entry) => entry.id !== screen.id) });
                    setSelectedId(null);
                    setSelectedElementId(null);
                  }}
                >
                  <Trash2 size={14} className="text-[var(--danger)]" />
                </IconButton>
              </div>
            </div>
            <Field label={t("gui.screenName")} hint={t("gui.screenNameHint")}>
              <Input
                className="mono text-[12px]"
                value={screen.name}
                onChange={(event) => updateScreen({ name: event.target.value })}
              />
            </Field>
            <Field label={`${t("field.title")} · ${editLocale.toUpperCase()}`}>
              <Input
                value={screen.title?.[editLocale] ?? ""}
                onChange={(event) =>
                  updateScreen({ title: { ...(screen.title ?? {}), [editLocale]: event.target.value } })
                }
              />
            </Field>
            {screen.template === "blank" ? (
              <>
                <Field label={t("gui.backgroundImage")}>
                  <AssetSelect kind="image" value={screen.background} onChange={(value) => updateScreen({ background: value })} />
                </Field>
                <Field label={t("gui.backgroundColor")}>
                  <div className="flex gap-2">
                    <input
                      type="color"
                      value={screen.backgroundColor || "#0b0a12"}
                      onChange={(event) => updateScreen({ backgroundColor: event.target.value })}
                      className="h-[32px] w-[46px] cursor-pointer rounded-md border border-[var(--border)] bg-transparent"
                    />
                    <Input
                      className="mono text-[12px]"
                      value={screen.backgroundColor ?? ""}
                      onChange={(event) => updateScreen({ backgroundColor: event.target.value })}
                    />
                  </div>
                </Field>
                <Checkbox checked={screen.modal} label={t("gui.modal")} onChange={(value) => updateScreen({ modal: value })} />
                <Checkbox checked={screen.tagMenu} label={t("gui.tagMenu")} onChange={(value) => updateScreen({ tagMenu: value })} />
              </>
            ) : (
              <TemplateFields
                screen={screen}
                onChange={(params) => updateScreen({ params })}
              />
            )}

            <div className="mt-3 border-t border-[var(--border)] pt-3">
              {screen.template !== "blank" ? null : element ? (
                <>
                  <div className="mb-2 flex items-center justify-between">
                    <span className="text-[12px] font-semibold">
                      {t(`gui.kind.${element.kind}`)} · {Math.round(element.x * 100)}%, {Math.round(element.y * 100)}%
                    </span>
                    <IconButton
                      title={t("common.delete")}
                      onClick={() => {
                        updateScreen({ elements: screen.elements.filter((entry) => entry.id !== element.id) });
                        setSelectedElementId(null);
                      }}
                    >
                      <Trash2 size={14} className="text-[var(--danger)]" />
                    </IconButton>
                  </div>
                  <Field label={`${t("field.text")} · ${editLocale.toUpperCase()}`}>
                    <Input
                      value={element.text?.[editLocale] ?? ""}
                      onChange={(event) =>
                        updateElement({ text: { ...(element.text ?? {}), [editLocale]: event.target.value } })
                      }
                    />
                  </Field>
                  {element.kind === "image" ? (
                    <Field label={t("field.asset")}>
                      <AssetSelect kind="image" value={element.image} onChange={(value) => updateElement({ image: value })} />
                    </Field>
                  ) : null}
                  {element.kind === "button" ? (
                    <>
                      <Field label={t("field.action")}>
                        <Select
                          value={element.action ?? "none"}
                          onChange={(event) => updateElement({ action: event.target.value })}
                          options={ACTIONS.map((action) => ({
                            value: action,
                            label: t(`gui.action.${action}`),
                          }))}
                        />
                      </Field>
                      {element.action === "jump" ? (
                        <Field label={t("field.target")}>
                          <Select
                            value={element.target ?? ""}
                            onChange={(event) => updateElement({ target: event.target.value || null })}
                            options={project.scenes.map((scene) => ({ value: scene.id, label: scene.title }))}
                            placeholder="—"
                          />
                        </Field>
                      ) : null}
                      {element.action === "show" || element.action === "hide" ? (
                        <Field label={t("field.screen")}>
                          <Input
                            className="mono text-[12px]"
                            value={element.target ?? ""}
                            onChange={(event) => updateElement({ target: event.target.value || null })}
                          />
                        </Field>
                      ) : null}
                    </>
                  ) : null}
                  {element.kind === "button" ? (
                    <>
                      <div className="grid grid-cols-2 gap-x-3">
                        <Field label={t("gui.idleImage")}>
                          <AssetSelect
                            kind="image"
                            value={element.idleImage ?? null}
                            onChange={(value) => updateElement({ idleImage: value })}
                          />
                        </Field>
                        <Field label={t("gui.hoverImage")}>
                          <AssetSelect
                            kind="image"
                            value={element.hoverImage ?? null}
                            onChange={(value) => updateElement({ hoverImage: value })}
                          />
                        </Field>
                      </div>
                      <Field label={t("gui.buttonBackground")}>
                        <AssetSelect
                          kind="image"
                          value={element.backgroundImage ?? null}
                          onChange={(value) => updateElement({ backgroundImage: value })}
                        />
                      </Field>
                      <Field label={t("gui.backgroundBorder")} hint={t("gui.backgroundBorderHint")}>
                        <Input
                          type="number"
                          min={0}
                          value={String(element.backgroundBorder ?? 0)}
                          onChange={(event) =>
                            updateElement({ backgroundBorder: Number(event.target.value) || 0 })
                          }
                        />
                      </Field>
                    </>
                  ) : null}
                  <div className="grid grid-cols-2 gap-x-3">
                    <Field label={t("gui.fontSize")}>
                      <Input
                        type="number"
                        value={String(element.fontSize)}
                        onChange={(event) => updateElement({ fontSize: Number(event.target.value) || 30 })}
                      />
                    </Field>
                    <Field label={t("gui.color")}>
                      <div className="flex gap-1.5">
                        <input
                          type="color"
                          value={element.color ?? "#ffffff"}
                          onChange={(event) => updateElement({ color: event.target.value })}
                          className="h-[32px] w-[40px] cursor-pointer rounded-md border border-[var(--border)] bg-transparent"
                        />
                        <Input
                          className="mono text-[11px]"
                          value={element.color ?? ""}
                          onChange={(event) => updateElement({ color: event.target.value || null })}
                        />
                      </div>
                    </Field>
                  </div>
                  <Field label={t("gui.align")}>
                    <Segmented
                      value={element.align}
                      onChange={(value) => updateElement({ align: value })}
                      options={[
                        { value: "left", label: t("screens.align.left") },
                        { value: "center", label: t("screens.align.center") },
                        { value: "right", label: t("screens.align.right") },
                      ]}
                    />
                  </Field>
                  <div className="mono text-[10px] text-[var(--muted-2)]">
                    x {element.x.toFixed(2)} · y {element.y.toFixed(2)} · w {element.w.toFixed(2)} · h{" "}
                    {element.h.toFixed(2)}
                  </div>
                </>
              ) : (
                <p className="text-[11px] leading-relaxed text-[var(--muted-2)]">{t("gui.selectElement")}</p>
              )}
            </div>
          </>
        ) : (
          <p className="text-[11px] leading-relaxed text-[var(--muted-2)]">{t("gui.noScreens")}</p>
        )}
      </div>
    </div>
  );
}

function TemplateFields({
  screen,
  onChange,
}: {
  screen: ScreenDef;
  onChange: (params: Record<string, unknown>) => void;
}) {
  const { t } = useTranslation();
  const spec = SCREEN_TEMPLATES.find((entry) => entry.id === screen.template);
  if (!spec) return null;
  const params = (screen.params ?? {}) as Record<string, unknown>;
  const patch = (key: string, value: unknown) => onChange({ ...params, [key]: value });

  return (
    <div className="mt-2">
      <div className="label">{t("tpl.params")}</div>
      {spec.fields.map((field) => {
        if (field.type === "list") {
          const rows = Array.isArray(params[field.key])
            ? (params[field.key] as Record<string, unknown>[])
            : [];
          return (
            <Field key={field.key} label={t(field.labelKey)}>
              <div className="space-y-2">
                {rows.map((row, index) => (
                  <div key={index} className="card p-2">
                    <div className="mb-1 flex items-center justify-between">
                      <span className="mono text-[10px] text-[var(--muted-2)]">{index + 1}</span>
                      <IconButton
                        title={t("common.delete")}
                        onClick={() => patch(field.key, rows.filter((_, i) => i !== index))}
                      >
                        <Trash2 size={12} />
                      </IconButton>
                    </div>
                    <div className="space-y-1.5">
                      {field.item.map((item) =>
                        item.type === "image" || item.type === "audio" ? (
                          <AssetSelect
                            key={item.key}
                            kind={item.type}
                            value={(row[item.key] as string | null | undefined) ?? null}
                            onChange={(value) =>
                              patch(field.key, rows.map((entry, i) => (i === index ? { ...entry, [item.key]: value } : entry)))
                            }
                          />
                        ) : (
                          <Input
                            key={item.key}
                            className="h-[28px]"
                            value={String(row[item.key] ?? "")}
                            placeholder={t(item.labelKey)}
                            onChange={(event) =>
                              patch(field.key, rows.map((entry, i) => (i === index ? { ...entry, [item.key]: event.target.value } : entry)))
                            }
                          />
                        ),
                      )}
                    </div>
                  </div>
                ))}
                <Button variant="ghost" size="sm" onClick={() => patch(field.key, [...rows, { ...field.defaults }])}>
                  <Plus size={13} /> {t("common.add")}
                </Button>
              </div>
            </Field>
          );
        }
        if (field.type === "checkbox") {
          return (
            <Checkbox
              key={field.key}
              checked={Boolean(params[field.key])}
              label={t(field.labelKey)}
              onChange={(value) => patch(field.key, value)}
            />
          );
        }
        if (field.type === "select") {
          return (
            <Field key={field.key} label={t(field.labelKey)}>
              <Select
                value={String(params[field.key] ?? "")}
                onChange={(event) => patch(field.key, event.target.value)}
                options={field.options.map((option) => ({ value: option.value, label: t(option.labelKey) }))}
              />
            </Field>
          );
        }
        return (
          <Field key={field.key} label={t(field.labelKey)}>
            <Input
              type={field.type === "number" ? "number" : "text"}
              value={String(params[field.key] ?? "")}
              onChange={(event) =>
                patch(field.key, field.type === "number" ? Number(event.target.value) || 0 : event.target.value)
              }
            />
          </Field>
        );
      })}
    </div>
  );
}
