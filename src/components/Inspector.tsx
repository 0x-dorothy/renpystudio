import { useTranslation } from "react-i18next";
import { Copy, Crosshair, Eye, Lock, Trash2, Wand2 } from "lucide-react";
import { useStudio } from "../store";
import { BlockForm } from "./BlockForm";
import { Button, Field, IconButton, Input, Select, Textarea } from "./ui";
import { blockDef } from "../blocks/registry";
import { useDebouncedValue } from "../hooks";
import type { Character, Variable } from "../types";

export function Inspector() {
  const { t } = useTranslation();
  const tab = useStudio((s) => s.tab);
  const project = useStudio((s) => s.project);
  const activeSceneId = useStudio((s) => s.activeSceneId);
  const selectedNodeId = useStudio((s) => s.selectedNodeId);
  const selectedCharacterId = useStudio((s) => s.selectedCharacterId);
  const selectedVariableId = useStudio((s) => s.selectedVariableId);
  const issues = useStudio((s) => s.issues);

  const scene = project?.scenes.find((s) => s.id === activeSceneId) ?? null;
  const node = scene?.nodes.find((n) => n.id === selectedNodeId) ?? null;
  const character = project?.characters.find((c) => c.id === selectedCharacterId) ?? null;
  const variable = project?.variables.find((v) => v.id === selectedVariableId) ?? null;

  let body: React.ReactNode;
  if (tab === "characters" && character) body = <CharacterEditor character={character} />;
  else if (tab === "variables" && variable) body = <VariableEditor variable={variable} />;
  else if (node && scene) body = <NodeInspector node={node} />;
  else if (scene) body = <SceneEditorPanel sceneId={scene.id} />;
  else body = <EmptyInspector />;

  return (
    <aside className="flex h-full w-[330px] flex-none flex-col border-l border-[var(--border)] bg-[var(--panel)]">
      <div className="flex h-[42px] flex-none items-center justify-between border-b border-[var(--border)] px-3">
        <div className="flex items-center gap-2">
          <span className="text-[11px] font-bold uppercase tracking-[0.08em] text-[var(--muted-2)]">
            {t("inspector.title")}
          </span>
          {issues && issues.errors > 0 ? (
            <span className="chip chip-danger">{issues.errors}</span>
          ) : null}
        </div>
        {node ? (
          <span className="chip">{t("inspector.block")}</span>
        ) : tab === "characters" && character ? (
          <span className="chip">{t("inspector.character")}</span>
        ) : tab === "variables" && variable ? (
          <span className="chip">{t("inspector.variable")}</span>
        ) : scene ? (
          <span className="chip">{t("inspector.scene")}</span>
        ) : null}
      </div>
      <div className="scroll flex-1 px-3.5 py-3.5">{body}</div>
    </aside>
  );
}

function EmptyInspector() {
  const { t } = useTranslation();
  return (
    <div className="mt-16 flex flex-col items-center gap-2 px-4 text-center">
      <div className="flex h-10 w-10 items-center justify-center rounded-full bg-[var(--panel-2)] text-[var(--muted-2)]">
        <Wand2 size={18} />
      </div>
      <p className="text-[12px] leading-relaxed text-[var(--muted)]">{t("inspector.empty")}</p>
    </div>
  );
}

function NodeInspector({ node }: { node: NonNullable<ReturnType<typeof useStudio.getState>["project"]>["scenes"][number]["nodes"][number] }) {
  const { t } = useTranslation();
  const selectedNodeId = useStudio((s) => s.selectedNodeId);
  const deleteNode = useStudio((s) => s.deleteNode);
  const duplicateNode = useStudio((s) => s.duplicateNode);
  const setTab = useStudio((s) => s.setTab);
  const def = blockDef(node.kind);
  const Icon = def?.icon;

  return (
    <div>
      <div className="mb-3 flex items-start justify-between gap-2">
        <div className="flex items-center gap-2.5">
          <div
            className="flex h-9 w-9 items-center justify-center rounded-lg"
            style={{ background: "var(--accent-soft)", color: "var(--accent)" }}
          >
            {Icon ? <Icon size={18} /> : null}
          </div>
          <div>
            <div className="text-[13px] font-semibold">{def ? t(def.labelKey) : node.kind}</div>
            <div className="mono text-[10px] text-[var(--muted-2)]">{node.id.slice(0, 8)}</div>
          </div>
        </div>
        <div className="flex items-center gap-1">
          <IconButton title={t("action.duplicate")} onClick={() => selectedNodeId && void duplicateNode(selectedNodeId)}>
            <Copy size={14} />
          </IconButton>
          <IconButton
            title={t("action.showOnGraph")}
            onClick={() => setTab("graph")}
          >
            <Crosshair size={14} />
          </IconButton>
          <IconButton
            title={t("common.delete")}
            onClick={() => selectedNodeId && void deleteNode(selectedNodeId)}
          >
            <Trash2 size={14} className="text-[var(--danger)]" />
          </IconButton>
        </div>
      </div>

      {node.source === "mcp" ? (
        <div className="mb-3 flex items-center gap-1.5 rounded-lg border border-[var(--accent)]/40 bg-[var(--accent-soft)] px-2 py-1.5 text-[11px] text-[var(--accent)]">
          <Eye size={12} /> {t("inspector.sourceMcp")}
        </div>
      ) : null}
      {node.locked ? (
        <div className="mb-3 flex items-center gap-1.5 rounded-lg border border-[var(--warn)]/40 bg-[var(--warn)]/10 px-2 py-1.5 text-[11px] text-[var(--warn)]">
          <Lock size={12} /> {t("inspector.locked")}
        </div>
      ) : null}

      <BlockForm node={node} />
    </div>
  );
}

function SceneEditorPanel({ sceneId }: { sceneId: string }) {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const updateScene = useStudio((s) => s.updateScene);
  const deleteScene = useStudio((s) => s.deleteScene);
  const duplicateScene = useStudio((s) => s.duplicateScene);
  const setStartScene = useStudio((s) => s.setStartScene);
  const scene = project?.scenes.find((s) => s.id === sceneId);
  const [draft, setDraft] = useDebouncedValue(
    sceneId,
    scene ?? null,
    (value) => {
      if (value) {
        void updateScene(value.id, {
          title: value.title,
          summary: value.summary,
          color: value.color ?? "",
          slugValue: value.slug,
          labelName: value.labelName ?? "",
        });
      }
    },
  );
  if (!draft) return null;
  const isStart = project?.meta.startScene === draft.id;

  return (
    <div>
      <div className="mb-3 flex items-center justify-between">
        <div className="text-[13px] font-semibold">{t("inspector.scene")}</div>
        <div className="flex items-center gap-1">
          <IconButton title={t("action.duplicate")} onClick={() => void duplicateScene(draft.id)}>
            <Copy size={14} />
          </IconButton>
          <IconButton
            title={t("common.delete")}
            onClick={() => {
              if (window.confirm(t("scene.deleteConfirm", { title: draft.title }))) {
                void deleteScene(draft.id);
              }
            }}
          >
            <Trash2 size={14} className="text-[var(--danger)]" />
          </IconButton>
        </div>
      </div>
      <Field label={t("field.title")}>
        <Input value={draft.title} onChange={(event) => setDraft({ ...draft, title: event.target.value })} />
      </Field>
      <Field label={t("field.slug")}>
        <Input
          className="mono text-[12px]"
          value={draft.slug}
          onChange={(event) => setDraft({ ...draft, slug: event.target.value })}
        />
      </Field>
      <Field label="Метка Ren'Py" hint={`label ${draft.labelName || `scene_${draft.slug}`}`}>
        <Input
          className="mono text-[12px]"
          value={draft.labelName ?? ""}
          placeholder={`scene_${draft.slug}`}
          onChange={(event) => setDraft({ ...draft, labelName: event.target.value })}
        />
      </Field>
      <Field label={t("field.summary")}>
        <Textarea
          rows={3}
          placeholder={t("scene.summaryPlaceholder")}
          value={draft.summary}
          onChange={(event) => setDraft({ ...draft, summary: event.target.value })}
        />
      </Field>
      <Field label={t("common.color")}>
        <div className="flex gap-2">
          <input
            type="color"
            className="h-[32px] w-[46px] cursor-pointer rounded-md border border-[var(--border)] bg-transparent"
            value={draft.color ?? "#8b6cf2"}
            onChange={(event) => setDraft({ ...draft, color: event.target.value })}
          />
          <Input
            value={draft.color ?? ""}
            placeholder="#8b6cf2"
            onChange={(event) => setDraft({ ...draft, color: event.target.value })}
          />
        </div>
      </Field>
      <Button
        variant={isStart ? "primary" : "default"}
        size="sm"
        onClick={() => void setStartScene(draft.id)}
        disabled={isStart}
        className="w-full"
      >
        {isStart ? t("scene.startBadge") : t("scene.setStart")}
      </Button>
      <div className="mt-4 rounded-lg border border-[var(--border)] bg-[var(--bg-soft)] px-3 py-2">
        <div className="text-[11px] text-[var(--muted-2)]">{t("scene.count", { count: draft.nodes.length })}</div>
        <div className="mono mt-0.5 text-[11px] text-[var(--muted)]">
          {draft.nodes.length ? `label ${draft.labelName || `scene_${draft.slug}`}` : "—"}
        </div>
      </div>
    </div>
  );
}

function CharacterEditor({ character }: { character: Character }) {
  const { t } = useTranslation();
  const save = useStudio((s) => s.saveCharacter);
  const remove = useStudio((s) => s.deleteCharacter);
  const assets = useStudio((s) => s.assets);
  const [draft, setDraft] = useDebouncedValue(character.id, character, (value) => {
    void save(value);
  });

  const images = assets.filter((a) => a.kind === "image");

  return (
    <div>
      <div className="mb-3 flex items-center justify-between">
        <div className="text-[13px] font-semibold">{t("inspector.character")}</div>
        <IconButton
          title={t("common.delete")}
          onClick={() => {
            if (window.confirm(t("characters.deleteConfirm", { name: draft.name }))) void remove(draft.id);
          }}
        >
          <Trash2 size={14} className="text-[var(--danger)]" />
        </IconButton>
      </div>
      <Field label={t("characters.name")}>
        <Input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} />
      </Field>
      <Field label={t("characters.kind")}>
        <Select
          value={draft.kind}
          onChange={(event) => setDraft({ ...draft, kind: event.target.value as Character["kind"] })}
          options={[
            { value: "character", label: t("characters.kind.character") },
            { value: "thought", label: t("characters.kind.thought") },
            { value: "narrator", label: t("characters.kind.narrator") },
          ]}
        />
      </Field>
      <Field label={t("characters.color")}>
        <div className="flex gap-2">
          <input
            type="color"
            className="h-[32px] w-[46px] cursor-pointer rounded-md border border-[var(--border)] bg-transparent"
            value={draft.color ?? "#e96aa5"}
            onChange={(event) => setDraft({ ...draft, color: event.target.value })}
          />
          <Input
            value={draft.color ?? ""}
            placeholder="#e96aa5"
            onChange={(event) => setDraft({ ...draft, color: event.target.value })}
          />
        </div>
      </Field>
      <Field label={t("characters.spritePrefix")} hint="mira → show mira happy">
        <Input
          className="mono text-[12px]"
          value={draft.spritePrefix ?? ""}
          onChange={(event) => setDraft({ ...draft, spritePrefix: event.target.value })}
        />
      </Field>
      <Field label={t("characters.avatar")}>
        <Select
          value={draft.avatar ?? ""}
          onChange={(event) => setDraft({ ...draft, avatar: event.target.value || null })}
          options={images.map((image) => ({ value: image.path, label: image.path }))}
          placeholder="—"
        />
      </Field>
      <Field label={t("characters.voiceTag")}>
        <Input
          value={draft.voiceTag ?? ""}
          onChange={(event) => setDraft({ ...draft, voiceTag: event.target.value })}
        />
      </Field>
      <div className="grid grid-cols-2 gap-2">
        <Field label={t("characters.prefix")}>
          <Input
            value={draft.prefix ?? ""}
            onChange={(event) => setDraft({ ...draft, prefix: event.target.value })}
          />
        </Field>
        <Field label={t("characters.suffix")}>
          <Input
            value={draft.suffix ?? ""}
            onChange={(event) => setDraft({ ...draft, suffix: event.target.value })}
          />
        </Field>
      </div>
      <Field label={t("characters.notes")}>
        <Textarea
          rows={2}
          value={draft.notes}
          onChange={(event) => setDraft({ ...draft, notes: event.target.value })}
        />
      </Field>
    </div>
  );
}

function VariableEditor({ variable }: { variable: Variable }) {
  const { t } = useTranslation();
  const save = useStudio((s) => s.saveVariable);
  const remove = useStudio((s) => s.deleteVariable);
  const [draft, setDraft] = useDebouncedValue(variable.id, variable, (value) => {
    void save(value);
  });

  return (
    <div>
      <div className="mb-3 flex items-center justify-between">
        <div className="text-[13px] font-semibold">{t("inspector.variable")}</div>
        <IconButton
          title={t("common.delete")}
          onClick={() => {
            if (window.confirm(t("variables.deleteConfirm", { name: draft.name }))) void remove(draft.id);
          }}
        >
          <Trash2 size={14} className="text-[var(--danger)]" />
        </IconButton>
      </div>
      <Field label={t("variables.name")}>
        <Input
          className="mono text-[12px]"
          value={draft.name}
          onChange={(event) => setDraft({ ...draft, name: event.target.value })}
        />
      </Field>
      <Field label={t("variables.type")}>
        <Select
          value={draft.varType}
          onChange={(event) =>
            setDraft({
              ...draft,
              varType: event.target.value as Variable["varType"],
              default:
                event.target.value === "number" ? 0 : event.target.value === "bool" ? false : "",
            })
          }
          options={[
            { value: "number", label: t("variables.type.number") },
            { value: "bool", label: t("variables.type.bool") },
            { value: "text", label: t("variables.type.text") },
          ]}
        />
      </Field>
      <Field label={t("variables.default")}>
        {draft.varType === "bool" ? (
          <Select
            value={draft.default === true ? "true" : "false"}
            onChange={(event) => setDraft({ ...draft, default: event.target.value === "true" })}
            options={[
              { value: "false", label: "False" },
              { value: "true", label: "True" },
            ]}
          />
        ) : draft.varType === "number" ? (
          <Input
            type="number"
            value={String(draft.default ?? 0)}
            onChange={(event) => setDraft({ ...draft, default: Number(event.target.value) })}
          />
        ) : (
          <Input
            value={String(draft.default ?? "")}
            onChange={(event) => setDraft({ ...draft, default: event.target.value })}
          />
        )}
      </Field>
      <Field label={t("field.description")}>
        <Textarea
          rows={3}
          value={draft.description}
          onChange={(event) => setDraft({ ...draft, description: event.target.value })}
        />
      </Field>
      <div className="rounded-lg border border-[var(--border)] bg-[var(--bg-soft)] px-3 py-2 text-[11px] text-[var(--muted)]">
        <span className="mono">
          default {draft.name || "…"} = {JSON.stringify(draft.default ?? null)}
        </span>
      </div>
    </div>
  );
}
