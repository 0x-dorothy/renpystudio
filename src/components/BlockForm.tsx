import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowDown, ArrowUp, Plus, Trash2 } from "lucide-react";
import { useStudio } from "../store";
import {
  Button,
  Checkbox,
  Field,
  IconButton,
  Input,
  Select,
  Textarea,
} from "./ui";
import {
  blockDef,
  localeText,
  type FieldDef,
} from "../blocks/registry";
import type { StudioNode } from "../types";
import { newId } from "../util";
import { assetUrl } from "../util";

interface MenuOption {
  id: string;
  text: Record<string, string>;
  condition?: string | null;
  target?: string | null;
  setVars?: unknown[];
}

export function BlockForm({ node }: { node: StudioNode }) {
  const { t } = useTranslation();
  const def = blockDef(node.kind);
  const project = useStudio((s) => s.project);
  const editLocale = useStudio((s) => s.editLocale);
  const updateNodeParams = useStudio((s) => s.updateNodeParams);
  const [draft, setDraft] = useState<Record<string, unknown>>(node.params);
  const [pending, setPending] = useState(false);
  const timer = useRef<number | null>(null);
  const draftRef = useRef(draft);
  draftRef.current = draft;

  useEffect(() => {
    setDraft(node.params);
    setPending(false);
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = null;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [node.id]);

  useEffect(() => {
    if (!pending) setDraft(node.params);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [JSON.stringify(node.params)]);

  useEffect(() => {
    return () => {
      if (timer.current) {
        window.clearTimeout(timer.current);
        void updateNodeParams(node.id, draftRef.current);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [node.id]);

  const update = (key: string, value: unknown) => {
    const next = { ...draft, [key]: value };
    setDraft(next);
    setPending(true);
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      timer.current = null;
      setPending(false);
      void updateNodeParams(node.id, next);
    }, 420);
  };

  const updateLocalized = (key: string, value: string) => {
    const current = (draft[key] as Record<string, string> | undefined) ?? {};
    update(key, { ...current, [editLocale]: value });
  };

  const sceneOptions = useMemo(() => {
    const scenes = project?.scenes ?? [];
    return scenes.map((s) => ({ value: s.id, label: s.title }));
  }, [project?.scenes]);

  const characterOptions = useMemo(() => {
    const characters = project?.characters ?? [];
    return [
      { value: "", label: t("characters.kind.narrator") },
      ...characters.map((c) => ({
        value: c.id,
        label: c.kind === "thought" ? `${c.name} (${t("characters.kind.thought")})` : c.name,
      })),
    ];
  }, [project?.characters, t]);

  if (!def) {
    return <div className="text-[12px] text-[var(--muted)]">{t("inspector.unknownBlock", { kind: node.kind })}</div>;
  }

  const renderField = (field: FieldDef) => {
    const value = draft[field.key];
    if (field.showWhen && !field.showWhen(draft)) return null;

    switch (field.type) {
      case "localized":
      case "localizedTextarea": {
        const localized = (value as Record<string, string> | undefined) ?? {};
        const isTextarea = field.type === "localizedTextarea";
        return (
          <Field
            key={field.key}
            label={`${t(field.labelKey)} · ${editLocale.toUpperCase()}`}
            hint={isTextarea ? t("field.hint.localized") : undefined}
          >
            {isTextarea ? (
              <Textarea
                value={localized[editLocale] ?? ""}
                onChange={(event) => updateLocalized(field.key, event.target.value)}
                rows={3}
                autoFocus={field.key === "text"}
              />
            ) : (
              <Input
                value={localized[editLocale] ?? ""}
                onChange={(event) => updateLocalized(field.key, event.target.value)}
              />
            )}
            {project && project.locales.length > 1 ? (
              <div className="mt-1 flex gap-1">
                {project.locales.map((locale) => (
                  <span
                    key={locale}
                    className={`chip ${locale === editLocale ? "chip-accent" : ""}`}
                    title={localeText(value, locale, project.meta.defaultLocale)}
                  >
                    {locale.toUpperCase()}
                  </span>
                ))}
              </div>
            ) : null}
          </Field>
        );
      }
      case "text":
        return (
          <Field key={field.key} label={t(field.labelKey)}>
            <Input
              value={String(value ?? "")}
              placeholder={field.placeholder}
              onChange={(event) => update(field.key, event.target.value)}
            />
          </Field>
        );
      case "number":
        return (
          <Field key={field.key} label={t(field.labelKey)}>
            <Input
              type="number"
              min={field.min}
              max={field.max}
              step={field.step ?? 1}
              value={value === null || value === undefined ? "" : String(value)}
              onChange={(event) => {
                const raw = event.target.value;
                update(field.key, raw === "" ? null : Number(raw));
              }}
            />
          </Field>
        );
      case "checkbox":
        return (
          <Checkbox
            key={field.key}
            checked={Boolean(value)}
            label={t(field.labelKey)}
            onChange={(checked) => update(field.key, checked)}
          />
        );
      case "select":
        return (
          <Field key={field.key} label={t(field.labelKey)} hint={field.hintKey ? t(field.hintKey) : undefined}>
            <Select
              value={String(value ?? "")}
              onChange={(event) => update(field.key, event.target.value)}
              options={(field.options ?? []).map((option) => ({
                value: option.value,
                label: t(option.labelKey),
              }))}
            />
          </Field>
        );
      case "character":
        return (
          <Field key={field.key} label={t(field.labelKey)}>
            <Select
              value={String(value ?? "")}
              onChange={(event) => update(field.key, event.target.value || null)}
              options={characterOptions}
            />
          </Field>
        );
      case "scene": {
        const current = String(value ?? "");
        const options = [...sceneOptions];
        if (current && !options.some((option) => option.value === current)) {
          options.unshift({ value: current, label: t("field.labelPrefix", { label: current }) });
        }
        return (
          <Field key={field.key} label={t(field.labelKey)} hint={field.hintKey ? t(field.hintKey) : undefined}>
            <Select
              value={current}
              onChange={(event) => update(field.key, event.target.value || null)}
              options={options}
              placeholder="—"
            />
          </Field>
        );
      }
      case "asset": {
        const kind = field.assetKind ?? "image";
        const assets = (useStudio.getState().assets ?? []).filter((asset) => asset.kind === kind);
        const current = String(value ?? "");
        const listId = `asset-list-${field.key}`;
        return (
          <Field key={field.key} label={t(field.labelKey)}>
            <div className="flex items-start gap-2">
              <div className="min-w-0 flex-1">
                <Input
                  list={listId}
                  value={current}
                  placeholder={field.placeholder ?? "images/…"}
                  onChange={(event) => update(field.key, event.target.value)}
                />
                <datalist id={listId}>
                  {assets.map((asset) => (
                    <option key={asset.path} value={asset.path} />
                  ))}
                </datalist>
                <div className="mt-1 text-[10px] text-[var(--muted-2)]">
                  {assets.length > 0
                    ? t("assets.libraryHint", { count: assets.length })
                    : t("assets.empty")}
                </div>
              </div>
              {kind === "image" && current ? (
                <img
                  src={assetUrl(current)}
                  alt=""
                  className="h-[34px] w-[52px] flex-none rounded-md border border-[var(--border)] object-cover"
                />
              ) : null}
            </div>
          </Field>
        );
      }
      case "variable":
        return (
          <Field key={field.key} label={t(field.labelKey)}>
            <Select
              value={String(value ?? "")}
              onChange={(event) => update(field.key, event.target.value)}
              options={(project?.variables ?? []).map((variable) => ({
                value: variable.name,
                label: variable.name,
              }))}
              placeholder="—"
            />
          </Field>
        );
      case "setValue": {
        const variableName = String(draft.variable ?? "");
        const variable = project?.variables.find((v) => v.name === variableName);
        const type = variable?.varType ?? "number";
        return (
          <Field key={field.key} label={t(field.labelKey)}>
            {type === "bool" ? (
              <Select
                value={value === true ? "true" : "false"}
                onChange={(event) => update(field.key, event.target.value === "true")}
                options={[
                  { value: "true", label: t("common.value") + ": True" },
                  { value: "false", label: t("common.value") + ": False" },
                ]}
              />
            ) : type === "text" ? (
              <Input
                value={String(value ?? "")}
                onChange={(event) => update(field.key, event.target.value)}
              />
            ) : (
              <Input
                type="number"
                value={value === null || value === undefined ? "" : String(value)}
                onChange={(event) =>
                  update(field.key, event.target.value === "" ? null : Number(event.target.value))
                }
              />
            )}
          </Field>
        );
      }
      case "expression":
        return (
          <Field key={field.key} label={t(field.labelKey)} hint={field.hintKey ? t(field.hintKey) : undefined}>
            <Input
              className="mono text-[12px]"
              value={String(value ?? "")}
              placeholder="trust >= 5 and met_key"
              onChange={(event) => update(field.key, event.target.value)}
            />
          </Field>
        );
      case "code":
        return (
          <Field key={field.key} label={t(field.labelKey)}>
            <Textarea
              className="mono text-[12.5px]"
              rows={12}
              spellCheck={false}
              value={String(value ?? "")}
              onChange={(event) => update(field.key, event.target.value)}
              placeholder={t("field.codePlaceholder")}
            />
          </Field>
        );
      case "menuOptions":
        return (
          <MenuOptionsEditor
            key={field.key}
            options={(value as MenuOption[] | undefined) ?? []}
            onChange={(next) => update(field.key, next)}
          />
        );
      default:
        return null;
    }
  };

  return (
    <div className="animate-in">
      {def.fields.map(renderField)}
      {node.locked ? (
        <div className="chip-warn chip mb-2">{t("inspector.locked")}</div>
      ) : null}
    </div>
  );
}

function MenuOptionsEditor({
  options,
  onChange,
}: {
  options: MenuOption[];
  onChange: (options: MenuOption[]) => void;
}) {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const editLocale = useStudio((s) => s.editLocale);

  const patch = (index: number, partial: Partial<MenuOption>) => {
    const next = options.map((option, i) => (i === index ? { ...option, ...partial } : option));
    onChange(next);
  };

  const move = (index: number, direction: -1 | 1) => {
    const target = index + direction;
    if (target < 0 || target >= options.length) return;
    const next = [...options];
    const [item] = next.splice(index, 1);
    next.splice(target, 0, item);
    onChange(next);
  };

  return (
    <Field label={t("field.options")}>
      <div className="flex flex-col gap-2">
        {options.map((option, index) => (
          <div key={option.id} className="card p-2.5">
            <div className="flex items-center gap-1.5">
              <span className="mono text-[11px] text-[var(--muted-2)]">{index + 1}</span>
              <Input
                value={option.text?.[editLocale] ?? ""}
                placeholder={t("field.text")}
                onChange={(event) =>
                  patch(index, {
                    text: { ...(option.text ?? {}), [editLocale]: event.target.value },
                  })
                }
                className="h-[30px]"
              />
              <IconButton title="Up" onClick={() => move(index, -1)}>
                <ArrowUp size={14} />
              </IconButton>
              <IconButton title="Down" onClick={() => move(index, 1)}>
                <ArrowDown size={14} />
              </IconButton>
              <IconButton
                title={t("common.delete")}
                onClick={() => onChange(options.filter((_, i) => i !== index))}
              >
                <Trash2 size={14} />
              </IconButton>
            </div>
            <div className="mt-1.5 grid grid-cols-2 gap-1.5">
              <Select
                value={option.target ?? ""}
                onChange={(event) => patch(index, { target: event.target.value || null })}
                options={(project?.scenes ?? []).map((s) => ({ value: s.id, label: s.title }))}
                placeholder={t("field.target")}
              />
              <Input
                value={option.condition ?? ""}
                placeholder={`${t("field.condition")}: trust >= 3`}
                onChange={(event) => patch(index, { condition: event.target.value || null })}
                className="mono h-[30px] text-[12px]"
              />
            </div>
          </div>
        ))}
        <Button
          variant="ghost"
          size="sm"
          onClick={() =>
            onChange([
              ...options,
              { id: newId(), text: { [editLocale]: "" }, condition: null, target: null, setVars: [] },
            ])
          }
        >
          <Plus size={14} /> {t("common.add")}
        </Button>
      </div>
    </Field>
  );
}
