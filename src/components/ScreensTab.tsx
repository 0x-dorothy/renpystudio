import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Image as ImageIcon, Music, Sparkles } from "lucide-react";
import { useStudio } from "../store";
import { Button, Checkbox, Field, Input, Segmented } from "./ui";
import { useDebouncedValue } from "../hooks";
import { assetUrl } from "../util";
import type { MainMenuConfig, UiConfig } from "../types";

export function AssetSelect({
  kind,
  value,
  onChange,
  allowEmpty = true,
}: {
  kind: "image" | "audio";
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

export function ScreensTab() {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const saveUi = useStudio((s) => s.saveUi);
  const editLocale = useStudio((s) => s.editLocale);
  const ui = project?.ui;

  const [draft, setDraft] = useDebouncedValue<UiConfig | null>("ui", ui ?? null, (next) => {
    if (next) void saveUi(next);
  });

  if (!project || !ui || !draft) {
    return <div className="flex flex-1 items-center justify-center text-[12px] text-[var(--muted)]">—</div>;
  }

  const cfg = draft.mainMenu;
  const patch = (partial: Partial<MainMenuConfig>) =>
    setDraft({ ...draft, mainMenu: { ...cfg, ...partial } });

  const patchLocalized = (key: "title" | "tagline", value: string) =>
    patch({ [key]: { ...(cfg[key] ?? {}), [editLocale]: value } } as Partial<MainMenuConfig>);

  const localizedValue = (key: "title" | "tagline") => (cfg[key]?.[editLocale] ?? "");
  const accent = cfg.accent || "#8b6cf2";

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-[52px] flex-none items-center justify-between border-b border-[var(--border)] px-5">
        <div className="flex items-center gap-2">
          <h2 className="text-[14px] font-semibold">{t("screens.title")}</h2>
          <span className="chip chip-accent">{t("screens.mainMenu")}</span>
        </div>
        <span className="text-[11px] text-[var(--muted-2)]">{t("screens.saved")}</span>
      </div>

      <div className="scroll flex-1">
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
                {localizedValue("tagline") ? (
                  <div className="text-[12px] opacity-80">{localizedValue("tagline")}</div>
                ) : null}
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
      </div>
    </div>
  );
}
