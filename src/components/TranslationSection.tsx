import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Download, Languages, Search } from "lucide-react";
import { useStudio } from "../store";
import { api } from "../api";
import { Button, Checkbox, Field, Input, Select } from "./ui";
import { blockDef } from "../blocks/registry";
import type { LocalizedText, Project, StudioNode } from "../types";

interface Entry {
  key: string;
  sceneId: string;
  nodeId: string;
  sceneTitle: string;
  blockLabel: string;
  fieldLabel: string;
  text: LocalizedText;
  apply: (locale: string, value: string) => void;
}

function localized(value: unknown): LocalizedText {
  return value && typeof value === "object" ? (value as LocalizedText) : {};
}

function textOf(text: LocalizedText, locale: string): string {
  return text[locale] ?? "";
}

export function TranslationSection({ project }: { project: Project }) {
  const { t } = useTranslation();
  const editLocale = useStudio((s) => s.editLocale);
  const setEditLocale = useStudio((s) => s.setEditLocale);
  const toast = useStudio((s) => s.toast);
  const [query, setQuery] = useState("");
  const [onlyEmpty, setOnlyEmpty] = useState(false);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [exportLocale, setExportLocale] = useState(project.meta.defaultLocale || project.locales[0] || "ru");
  const [exporting, setExporting] = useState(false);

  const saveText = (sceneId: string, nodeId: string, mutate: (params: Record<string, unknown>) => Record<string, unknown>) => {
    const current = useStudio.getState().project;
    const node = current?.scenes.find((scene) => scene.id === sceneId)?.nodes.find((entry) => entry.id === nodeId);
    if (!node) return;
    const next = mutate((node.params ?? {}) as Record<string, unknown>);
    void useStudio.getState().updateNodeParamsInScene(sceneId, nodeId, next);
  };

  const entries = useMemo<Entry[]>(() => {
    const out: Entry[] = [];
    const push = (
      sceneId: string,
      sceneTitle: string,
      node: StudioNode,
      suffix: string,
      fieldLabel: string,
      text: LocalizedText,
      apply: (locale: string, value: string) => void,
    ) => {
      out.push({
        key: `${sceneId}:${node.id}:${suffix}`,
        sceneId,
        nodeId: node.id,
        sceneTitle,
        blockLabel: t(blockDef(node.kind)?.labelKey ?? `block.${node.kind}`),
        fieldLabel,
        text,
        apply,
      });
    };

    for (const scene of project.scenes) {
      for (const node of scene.nodes) {
        const params = (node.params ?? {}) as Record<string, unknown>;
        if (node.kind === "say" || node.kind === "narrate" || node.kind === "comment") {
          const text = localized(params.text);
          push(scene.id, scene.title, node, "text", t("field.text"), text, (locale, value) =>
            saveText(scene.id, node.id, (current) => ({
              ...current,
              text: { ...localized(current.text), [locale]: value },
            })),
          );
        } else if (node.kind === "menu") {
          push(scene.id, scene.title, node, "prompt", t("field.prompt"), localized(params.prompt), (locale, value) =>
            saveText(scene.id, node.id, (current) => ({
              ...current,
              prompt: { ...localized(current.prompt), [locale]: value },
            })),
          );
          const options = Array.isArray(params.options) ? (params.options as Record<string, unknown>[]) : [];
          for (const option of options) {
            push(
              scene.id,
              scene.title,
              node,
              `option:${String(option.id)}`,
              t("field.options"),
              localized(option.text),
              (locale, value) =>
                saveText(scene.id, node.id, (current) => {
                  const list = Array.isArray(current.options) ? (current.options as Record<string, unknown>[]) : [];
                  return {
                    ...current,
                    options: list.map((entry) =>
                      entry.id === option.id
                        ? { ...entry, text: { ...localized(entry.text), [locale]: value } }
                        : entry,
                    ),
                  };
                }),
            );
          }
        } else if (node.kind === "pointclick") {
          push(scene.id, scene.title, node, "prompt", t("field.prompt"), localized(params.prompt), (locale, value) =>
            saveText(scene.id, node.id, (current) => ({
              ...current,
              prompt: { ...localized(current.prompt), [locale]: value },
            })),
          );
          const hotspots = Array.isArray(params.hotspots) ? (params.hotspots as Record<string, unknown>[]) : [];
          for (const hotspot of hotspots) {
            push(
              scene.id,
              scene.title,
              node,
              `zone:${String(hotspot.id)}`,
              t("pointclick.zoneLabel"),
              localized(hotspot.label),
              (locale, value) =>
                saveText(scene.id, node.id, (current) => {
                  const list = Array.isArray(current.hotspots) ? (current.hotspots as Record<string, unknown>[]) : [];
                  return {
                    ...current,
                    hotspots: list.map((entry) =>
                      entry.id === hotspot.id
                        ? { ...entry, label: { ...localized(entry.label), [locale]: value } }
                        : entry,
                    ),
                  };
                }),
            );
          }
        }
      }
    }
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project, t]);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return entries.filter((entry) => {
      if (onlyEmpty && textOf(entry.text, editLocale).trim().length > 0) return false;
      if (!needle) return true;
      const haystack = `${entry.sceneTitle} ${entry.blockLabel} ${entry.text[project.meta.defaultLocale] ?? ""} ${entry.text[editLocale] ?? ""}`.toLowerCase();
      return haystack.includes(needle);
    });
  }, [entries, query, onlyEmpty, editLocale, project.meta.defaultLocale]);

  const coverage = project.locales.map((locale) => {
    const filled = entries.filter((entry) => textOf(entry.text, locale).trim().length > 0).length;
    return { locale, filled, total: entries.length, percent: entries.length ? Math.round((filled / entries.length) * 100) : 0 };
  });

  const exportTranslations = async () => {
    setExporting(true);
    try {
      const report = await api.translate(exportLocale);
      if (report.ok) toast("success", t("tr.exported", { files: report.files, locale: exportLocale.toUpperCase() }));
      else toast("error", report.log.slice(0, 300) || t("error.generic"));
    } catch (error) {
      toast("error", String(error));
    } finally {
      setExporting(false);
    }
  };

  const columnLocales = project.locales;

  return (
    <div className="flex h-full flex-col">
      <div className="flex flex-none flex-wrap items-center gap-2 border-b border-[var(--border)] px-5 py-3">
        <Languages size={15} className="text-[var(--accent)]" />
        <span className="text-[12px] font-semibold">{t("tr.title")}</span>
        <span className="chip">{t("tr.total", { count: entries.length })}</span>
        {coverage.map((entry) => (
          <span key={entry.locale} className={`chip ${entry.percent === 100 ? "chip-ok" : ""}`} title={`${entry.filled}/${entry.total}`}>
            {entry.locale.toUpperCase()} · {entry.percent}%
          </span>
        ))}
        <div className="flex-1" />
        <div className="flex items-center gap-1.5">
          <Select
            className="h-[30px] text-[11px]"
            value={exportLocale}
            onChange={(event) => setExportLocale(event.target.value)}
            options={project.locales.map((locale) => ({ value: locale, label: locale.toUpperCase() }))}
          />
          <Button size="sm" variant="primary" disabled={exporting} onClick={() => void exportTranslations()}>
            <Download size={13} /> {t("tr.export")}
          </Button>
        </div>
      </div>

      <div className="flex flex-none flex-wrap items-center gap-3 border-b border-[var(--border)] px-5 py-2">
        <div className="flex items-center gap-1.5">
          <Search size={13} className="text-[var(--muted-2)]" />
          <Input
            className="h-[28px] w-[240px] text-[12px]"
            value={query}
            placeholder={t("common.search")}
            onChange={(event) => setQuery(event.target.value)}
          />
        </div>
        <Checkbox checked={onlyEmpty} label={t("tr.onlyEmpty", { locale: editLocale.toUpperCase() })} onChange={setOnlyEmpty} />
        <div className="flex-1" />
        <span className="text-[11px] text-[var(--muted-2)]">{t("tr.hint")}</span>
        <Select
          className="h-[28px] text-[11px]"
          value={editLocale}
          onChange={(event) => setEditLocale(event.target.value)}
          options={project.locales.map((locale) => ({ value: locale, label: locale.toUpperCase() }))}
        />
      </div>

      <div className="scroll min-h-0 flex-1">
        <table className="w-full table-fixed border-collapse text-[12px]">
          <thead className="sticky top-0 z-10 bg-[var(--panel)]">
            <tr className="border-b border-[var(--border)] text-left text-[10px] uppercase tracking-[0.06em] text-[var(--muted-2)]">
              <th className="w-[220px] px-3 py-2 font-semibold">{t("tr.context")}</th>
              {columnLocales.map((locale) => (
                <th key={locale} className={`px-3 py-2 font-semibold ${locale === editLocale ? "text-[var(--accent)]" : ""}`}>
                  {locale.toUpperCase()}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {filtered.map((entry) => (
              <tr key={entry.key} className="border-b border-[var(--border)]/60 align-top">
                <td className="px-3 py-2">
                  <div className="truncate text-[11px] text-[var(--muted)]" title={entry.sceneTitle}>
                    {entry.sceneTitle}
                  </div>
                  <div className="truncate text-[10px] text-[var(--muted-2)]">
                    {entry.blockLabel} · {entry.fieldLabel}
                  </div>
                </td>
                {columnLocales.map((locale) => {
                  const key = `${entry.key}:${locale}`;
                  const value = drafts[key] ?? textOf(entry.text, locale);
                  const empty = value.trim().length === 0;
                  return (
                    <td key={locale} className="px-2 py-1.5">
                      <textarea
                        className={`textarea min-h-[44px] w-full resize-y text-[12px] ${
                          empty ? "border-[var(--warn)]/50" : ""
                        } ${locale === editLocale ? "ring-1 ring-[var(--accent)]/30" : ""}`}
                        value={value}
                        onChange={(event) => setDrafts({ ...drafts, [key]: event.target.value })}
                        onBlur={() => {
                          const next = drafts[key];
                          if (next === undefined) return;
                          entry.apply(locale, next);
                          const copy = { ...drafts };
                          delete copy[key];
                          setDrafts(copy);
                        }}
                      />
                    </td>
                  );
                })}
              </tr>
            ))}
            {filtered.length === 0 ? (
              <tr>
                <td colSpan={columnLocales.length + 1} className="px-4 py-10 text-center text-[12px] text-[var(--muted-2)]">
                  {t("tr.empty")}
                </td>
              </tr>
            ) : null}
          </tbody>
        </table>
      </div>
    </div>
  );
}
