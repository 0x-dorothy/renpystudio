import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertTriangle,
  Check,
  Copy,
  Download,
  FolderOpen,
  Loader2,
  Play,
  RefreshCw,
  Server,
  ShieldCheck,
  X,
} from "lucide-react";
import { useStudio } from "../store";
import { api } from "../api";
import { pickFolder } from "../dialogs";import { Button, Checkbox, Field, IconButton, Input, Modal, Segmented, Select, Spinner, Textarea } from "./ui";
import type { ImportReport, Settings } from "../types";
import { formatDateTime } from "../util";
import { useDebouncedValue } from "../hooks";
import { listen } from "@tauri-apps/api/event";
import { HotspotModal } from "./HotspotEditor";

export function StudioModals() {
  return (
    <>
      <NewProjectModal />
      <SettingsModal />
      <BuildModal />
      <LintModal />
      <ConflictsModal />
      <DiffModal />
      <PluginDocsModal />
      <ImportModal />
      <HotspotModal />
    </>
  );
}

/* ── New project ──────────────────────────────────────────────────────── */

function NewProjectModal() {
  const { t } = useTranslation();
  const open = useStudio((s) => s.modal) === "newProject";
  const close = useStudio((s) => s.closeModal);
  const newProject = useStudio((s) => s.newProject);
  const busy = useStudio((s) => s.busy);
  const [title, setTitle] = useState("");
  const [folder, setFolder] = useState("");

  useEffect(() => {
    if (open) {
      setTitle("");
      setFolder("");
    }
  }, [open]);

  const choose = async () => {
    const picked = await pickFolder(t("newProject.folder"));
    if (picked) setFolder(picked);
  };

  return (
    <Modal
      open={open}
      onClose={close}
      title={t("newProject.title")}
      subtitle={t("newProject.hint")}
      width={560}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="primary"
            disabled={!title.trim() || !folder || busy}
            onClick={() => void newProject(folder, title.trim())}
          >
            {busy ? <Spinner /> : <Play size={14} />}
            {t("newProject.create")}
          </Button>
        </>
      }
    >
      <Field label={t("newProject.name")}>
        <Input
          autoFocus
          placeholder={t("newProject.namePlaceholder")}
          value={title}
          onChange={(event) => setTitle(event.target.value)}
        />
      </Field>
      <Field label={t("newProject.folder")}>
        <div className="flex gap-2">
          <Input value={folder} readOnly placeholder="C:\\Users\\…\\Documents" />
          <Button variant="ghost" onClick={() => void choose()}>
            <FolderOpen size={14} /> {t("newProject.choose")}
          </Button>
        </div>
      </Field>
      {title.trim() && folder ? (
        <div className="mono rounded-lg border border-[var(--border)] bg-[var(--bg-soft)] px-3 py-2 text-[11px] text-[var(--muted)]">
          {folder}\{title.trim()}\game\…
        </div>
      ) : null}
    </Modal>
  );
}

/* ── Settings ─────────────────────────────────────────────────────────── */

const ACCENTS = ["violet", "rose", "teal", "amber", "blue", "green"];

function SettingsModal() {
  const { t } = useTranslation();
  const open = useStudio((s) => s.modal) === "settings";
  const close = useStudio((s) => s.closeModal);
  const settings = useStudio((s) => s.settings);
  const sdk = useStudio((s) => s.sdk);
  const mcp = useStudio((s) => s.mcp);
  const version = useStudio((s) => s.version);
  const project = useStudio((s) => s.project);
  const saveSettings = useStudio((s) => s.saveSettings);
  const forgetAiKey = useStudio((s) => s.forgetAiKey);
  const detectSdk = useStudio((s) => s.detectSdk);
  const setSdkPath = useStudio((s) => s.setSdkPath);
  const startMcp = useStudio((s) => s.startMcp);
  const stopMcp = useStudio((s) => s.stopMcp);
  const [draft, setDraft] = useState<Settings>(settings);

  useEffect(() => {
    if (open) setDraft(settings);
  }, [open, settings]);

  const commit = (patch: Partial<Settings>, immediate = true) => {
    const next = { ...draft, ...patch };
    setDraft(next);
    if (immediate) void saveSettings(next);
  };

  const chooseSdk = async () => {
    const folder = await pickFolder(t("settings.sdk"));
    if (folder) await setSdkPath(folder);
  };

  return (
    <Modal
      open={open}
      onClose={close}
      title={t("settings.title")}
      width={780}
      footer={
        <Button variant="primary" onClick={close}>
          {t("common.close")}
        </Button>
      }
    >
      {project ? <ProjectSection /> : null}

      <Section title={t("settings.sdk")} icon={<ShieldCheck size={14} />}>
        <div className="flex items-center gap-2">
          <Input value={sdk?.root ?? draft.sdkPath ?? ""} readOnly placeholder={t("settings.sdkMissing")} />
          <Button variant="ghost" size="sm" onClick={() => void detectSdk(true)}>
            <RefreshCw size={13} /> {t("settings.sdkDetect")}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => void chooseSdk()}>
            <FolderOpen size={13} /> {t("settings.sdkBrowse")}
          </Button>
        </div>
        {sdk ? (
          <div className="mt-1.5 flex items-center gap-2 text-[11px] text-[var(--ok)]">
            <Check size={11} /> {t("settings.sdkFound", { version: sdk.version })}
          </div>
        ) : (
          <div className="mt-1.5 flex items-center gap-2 text-[11px] text-[var(--warn)]">
            <AlertTriangle size={11} /> {t("settings.sdkMissing")}
          </div>
        )}
      </Section>

      <Section title={t("settings.interface")}>
        <div className="grid grid-cols-2 gap-x-5">
          <Field label={t("settings.language")}>
            <Segmented
              value={draft.locale}
              onChange={(value) => commit({ locale: value })}
              options={[
                { value: "ru", label: t("locale.ru") },
                { value: "en", label: t("locale.en") },
              ]}
            />
          </Field>
          <Field label={t("settings.theme")}>
            <Segmented
              value={draft.theme}
              onChange={(value) => commit({ theme: value })}
              options={[
                { value: "dark", label: t("settings.theme.dark") },
                { value: "light", label: t("settings.theme.light") },
              ]}
            />
          </Field>
          <Field label={t("settings.accent")}>
            <div className="flex gap-1.5">
              {ACCENTS.map((accent) => (
                <button
                  key={accent}
                  type="button"
                  onClick={() => commit({ accent })}
                  className={`h-6 w-6 rounded-full border-2 transition-transform ${
                    draft.accent === accent ? "scale-110 border-[var(--text)]" : "border-transparent"
                  }`}
                  style={{
                    background:
                      accent === "violet" ? "#8b6cf2"
                      : accent === "rose" ? "#e96aa5"
                      : accent === "teal" ? "#2fc7c0"
                      : accent === "amber" ? "#e5a03c"
                      : accent === "blue" ? "#55a8f5"
                      : "#4ec97a",
                  }}
                  title={accent}
                />
              ))}
            </div>
          </Field>
          <Field label={t("settings.autosave")}>
            <Input
              type="number"
              min={5}
              max={600}
              value={draft.autosaveSeconds}
              onChange={(event) => commit({ autosaveSeconds: Number(event.target.value) || 30 }, false)}
              onBlur={() => void saveSettings(draft)}
            />
          </Field>
          <Field label={t("settings.graphDetail")}>
            <Select
              value={draft.graphDetail}
              onChange={(event) => commit({ graphDetail: event.target.value as Settings["graphDetail"] })}
              options={[
                { value: "scenes", label: t("graph.detail.scenes") },
                { value: "choices", label: t("graph.detail.choices") },
                { value: "all", label: t("graph.detail.all") },
              ]}
            />
          </Field>
        </div>
      </Section>

      <Section title={t("settings.mcp")} icon={<Server size={14} />}>
        <div className="flex items-end gap-2">
          <Field label={t("settings.mcpPort")} className="w-[120px]">
            <Input
              type="number"
              value={draft.mcpPort}
              onChange={(event) => setDraft({ ...draft, mcpPort: Number(event.target.value) || 8791 })}
            />
          </Field>
          {mcp.running ? (
            <Button variant="danger" size="sm" onClick={() => void stopMcp()}>
              {t("action.stop")}
            </Button>
          ) : (
            <Button
              variant="primary"
              size="sm"
              onClick={() => void startMcp(draft.mcpPort)}
            >
              <Play size={13} /> {t("settings.mcpEnable")}
            </Button>
          )}
          <span className={`mb-1 text-[11px] ${mcp.running ? "text-[var(--ok)]" : "text-[var(--muted-2)]"}`}>
            {mcp.running ? t("settings.mcpRunning") : t("settings.mcpStopped")}
          </span>
        </div>
        <Field label={t("settings.mcpUrl")} hint={t("settings.mcpHint")}>
          <div className="flex gap-2">
            <Input
              readOnly
              className="mono text-[11.5px]"
              value={mcp.running ? `http://127.0.0.1:${mcp.port}/mcp` : "http://127.0.0.1:—/mcp"}
            />
            <IconButton
              title={t("common.copy")}
              onClick={() => {
                if (mcp.running) void navigator.clipboard.writeText(`http://127.0.0.1:${mcp.port}/mcp`);
              }}
            >
              <Copy size={14} />
            </IconButton>
          </div>
        </Field>
        <Checkbox
          checked={draft.mcpRequireConfirm}
          label={t("settings.mcpMark")}
          onChange={(checked) => commit({ mcpRequireConfirm: checked })}
        />
      </Section>

      <Section title={t("settings.ai")}>
        <div className="grid grid-cols-3 gap-x-4">
          <Field label={t("settings.aiUrl")}>
            <Input
              className="mono text-[11.5px]"
              value={draft.ai.baseUrl}
              onChange={(event) => setDraft({ ...draft, ai: { ...draft.ai, baseUrl: event.target.value } })}
              onBlur={() => void saveSettings(draft)}
            />
          </Field>
          <Field label={t("settings.aiKey")}>
            <div className="flex items-center gap-1.5">
              <Input
                type="password"
                autoComplete="off"
                value={draft.ai.apiKey ?? ""}
                placeholder={draft.ai.hasKey ? t("settings.aiKeySaved") : "sk-…"}
                onChange={(event) => setDraft({ ...draft, ai: { ...draft.ai, apiKey: event.target.value } })}
                onBlur={() => void saveSettings(draft)}
              />
              {draft.ai.hasKey ? (
                <Button
                  size="sm"
                  variant="ghost"
                  title={t("settings.aiKeyRemove")}
                  onClick={() => void forgetAiKey()}
                >
                  {t("settings.aiKeyRemove")}
                </Button>
              ) : null}
            </div>
          </Field>
          <Field label={t("settings.aiModel")}>
            <Input
              value={draft.ai.model}
              onChange={(event) => setDraft({ ...draft, ai: { ...draft.ai, model: event.target.value } })}
              onBlur={() => void saveSettings(draft)}
            />
          </Field>
        </div>
        <p className="text-[11px] leading-relaxed text-[var(--muted-2)]">{t("settings.aiHint")}</p>
      </Section>

      <div className="mt-2 flex items-center justify-between border-t border-[var(--border)] pt-3 text-[11px] text-[var(--muted-2)]">
        <span>{t("settings.version", { version })}</span>
        <span className="mono">Tauri 2 · React 18 · Rust</span>
      </div>
    </Modal>
  );
}

function ProjectSection() {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const saveMeta = useStudio((s) => s.saveMeta);
  const translateProject = useStudio((s) => s.translateProject);
  const [meta, setMeta] = useDebouncedValue(
    project?.meta.createdAt ?? "meta",
    project?.meta ?? null,
    (value) => {
      if (value) void saveMeta(value);
    },
  );
  if (!project || !meta) return null;
  return (
    <Section title={t("menu.project")}>
      <div className="grid grid-cols-2 gap-x-4">
        <Field label={t("common.name")}>
          <Input value={meta.title} onChange={(event) => setMeta({ ...meta, title: event.target.value })} />
        </Field>
        <Field label={t("common.author")}>
          <Input value={meta.author} onChange={(event) => setMeta({ ...meta, author: event.target.value })} />
        </Field>
      </div>
      <Field label={t("common.description")}>
        <Textarea
          rows={2}
          value={meta.description}
          onChange={(event) => setMeta({ ...meta, description: event.target.value })}
        />
      </Field>
      <div className="grid grid-cols-2 gap-x-4">
        <Field label={t("field.slug")}>
          <Input
            className="mono text-[12px]"
            value={meta.gameSlug}
            onChange={(event) => setMeta({ ...meta, gameSlug: event.target.value })}
          />
        </Field>
        <Field label={t("scene.startBadge")}>
          <Select
            value={meta.startScene ?? ""}
            onChange={(event) => setMeta({ ...meta, startScene: event.target.value || null })}
            options={project.scenes.map((scene) => ({ value: scene.id, label: scene.title }))}
            placeholder="—"
          />
        </Field>
      </div>
      <Field label={t("settings.localization")} hint={t("settings.tlHint")}>
        <div className="flex flex-wrap gap-2">
          {project.locales.filter((locale) => locale !== project.meta.defaultLocale).length === 0 ? (
            <span className="text-[11px] text-[var(--muted-2)]">—</span>
          ) : (
            project.locales
              .filter((locale) => locale !== project.meta.defaultLocale)
              .map((locale) => (
                <Button key={locale} size="sm" onClick={() => void translateProject(locale)}>
                  {t("settings.generateTl", { locale })}
                </Button>
              ))
          )}
        </div>
      </Field>
    </Section>
  );
}

function Section({
  title,
  icon,
  children,
}: {
  title: string;
  icon?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="mb-5">
      <div className="mb-2 flex items-center gap-1.5 text-[11px] font-bold uppercase tracking-[0.08em] text-[var(--muted-2)]">
        {icon}
        {title}
      </div>
      {children}
    </div>
  );
}

/* ── Build ────────────────────────────────────────────────────────────── */

function BuildModal() {
  const { t } = useTranslation();
  const open = useStudio((s) => s.modal) === "build";
  const close = useStudio((s) => s.closeModal);
  const sdk = useStudio((s) => s.sdk);
  const [pkg, setPkg] = useState("pc");
  const [running, setRunning] = useState(false);
  const [log, setLog] = useState<string>("");
  const [ok, setOk] = useState<boolean | null>(null);
  const [artifact, setArtifact] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    let alive = true;
    const unLog = listen<{ line: string }>("studio://build-log", (event) => {
      if (!alive) return;
      setLog((prev) => (prev ? `${prev}\n${event.payload.line}` : event.payload.line));
    });
    const unDone = listen<{ ok: boolean; artifact?: string | null }>("studio://build-done", (event) => {
      if (!alive) return;
      setRunning(false);
      setOk(event.payload.ok);
      setArtifact(event.payload.artifact ?? null);
    });
    void api.buildStatus().then((state) => {
      if (!alive) return;
      setRunning(state.running);
      setOk(state.ok);
      setArtifact(state.artifact);
      if (state.log.length) setLog(state.log.join("\n"));
    });
    return () => {
      alive = false;
      void unLog.then((fn) => fn());
      void unDone.then((fn) => fn());
    };
  }, [open]);

  const build = async () => {
    setRunning(true);
    setLog("");
    setOk(null);
    setArtifact(null);
    try {
      await api.buildStart(pkg);
    } catch (error) {
      setRunning(false);
      setOk(false);
      setLog(String(error));
    }
  };

  return (
    <Modal
      open={open}
      onClose={close}
      title={t("build.title")}
      width={680}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t("common.close")}
          </Button>
          <Button variant="primary" disabled={running || !sdk} onClick={() => void build()}>
            {running ? <Spinner /> : <Download size={14} />} {t("build.start")}
          </Button>
        </>
      }
    >
      <Field label={t("build.platform")}>
        <Select
          value={pkg}
          onChange={(event) => setPkg(event.target.value)}
          options={[
            { value: "pc", label: t("build.windows") },
            { value: "linux", label: t("build.linux") },
            { value: "mac", label: t("build.mac") },
          ]}
        />
      </Field>
      <p className="mb-3 text-[11px] leading-relaxed text-[var(--muted-2)]">{t("build.requires")}</p>
      {!sdk ? (
        <div className="mb-3 flex items-center gap-2 text-[12px] text-[var(--warn)]">
          <AlertTriangle size={13} /> {t("error.sdkMissing")}
        </div>
      ) : null}
      {ok !== null ? (
        <div className={`mb-2 flex items-center gap-2 text-[12px] ${ok ? "text-[var(--ok)]" : "text-[var(--danger)]"}`}>
          {ok ? <Check size={13} /> : <X size={13} />}{" "}
          {ok ? t("build.done", { artifact: artifact ?? "dist" }) : t("build.failed")}
        </div>
      ) : null}
      <Field label={t("build.log")}>
        <Textarea className="mono text-[11px]" rows={14} readOnly value={log} />
      </Field>
    </Modal>
  );
}

/* ── Lint / issues ────────────────────────────────────────────────────── */

function LintModal() {
  const { t } = useTranslation();
  const open = useStudio((s) => s.modal) === "lint";
  const close = useStudio((s) => s.closeModal);
  const lint = useStudio((s) => s.lint);
  const issues = useStudio((s) => s.issues);
  const selectScene = useStudio((s) => s.selectScene);
  const selectNode = useStudio((s) => s.selectNode);
  const setTab = useStudio((s) => s.setTab);

  const focus = (sceneId: string | null, nodeId: string | null) => {
    if (!sceneId) return;
    setTab("scene");
    selectScene(sceneId);
    if (nodeId) window.setTimeout(() => selectNode(nodeId), 60);
    close();
  };

  return (
    <Modal
      open={open}
      onClose={close}
      title={t("lint.title")}
      width={760}
      footer={
        <Button variant="ghost" onClick={close}>
          {t("common.close")}
        </Button>
      }
    >
      <div className="mb-4">
        <div className="mb-1.5 text-[11px] font-bold uppercase tracking-[0.08em] text-[var(--muted-2)]">
          {t("issue.title")}
        </div>
        {!issues || issues.issues.length === 0 ? (
          <div className="flex items-center gap-2 rounded-lg border border-[var(--border)] bg-[var(--panel-2)] px-3 py-2 text-[12px] text-[var(--ok)]">
            <Check size={13} /> {t("issue.empty")}
          </div>
        ) : (
          <div className="flex flex-col gap-1">
            {issues.issues.map((issue, index) => (
              <button
                key={`${issue.code}-${index}`}
                type="button"
                onClick={() => focus(issue.sceneId, issue.nodeId)}
                className="flex items-start gap-2 rounded-lg border border-[var(--border)] bg-[var(--panel-2)] px-3 py-2 text-left hover:bg-[var(--panel-3)]"
              >
                <AlertTriangle
                  size={13}
                  className={
                    issue.severity === "error"
                      ? "mt-0.5 text-[var(--danger)]"
                      : issue.severity === "warning"
                        ? "mt-0.5 text-[var(--warn)]"
                        : "mt-0.5 text-[var(--muted-2)]"
                  }
                />
                <span className="flex-1 text-[12px] leading-snug">{issue.message}</span>
                {issue.sceneId ? <span className="text-[10px] text-[var(--muted-2)]">{t("lint.goTo")}</span> : null}
              </button>
            ))}
          </div>
        )}
      </div>

      <div>
        <div className="mb-1.5 flex items-center justify-between">
          <span className="text-[11px] font-bold uppercase tracking-[0.08em] text-[var(--muted-2)]">
            Ren'Py SDK lint
          </span>
          {lint ? (
            <span className={`chip ${lint.report.problems.length ? "chip-warn" : "chip-ok"}`}>
              {lint.report.message}
            </span>
          ) : null}
        </div>
        {!lint ? (
          <div className="rounded-lg border border-dashed border-[var(--border)] px-3 py-3 text-[12px] text-[var(--muted-2)]">
            {t("lint.notRun")}
          </div>
        ) : lint.report.problems.length === 0 ? (
          <div className="flex items-center gap-2 rounded-lg border border-[var(--border)] bg-[var(--panel-2)] px-3 py-2 text-[12px] text-[var(--ok)]">
            <Check size={13} /> {t("lint.empty")}
          </div>
        ) : (
          <div className="flex flex-col gap-1">
            {lint.report.problems.map((problem, index) => (
              <div
                key={`${problem.file}-${problem.line}-${index}`}
                className="rounded-lg border border-[var(--border)] bg-[var(--panel-2)] px-3 py-2"
              >
                <div className="flex items-center gap-2">
                  <span className={`chip ${problem.severity === "error" ? "chip-danger" : "chip-warn"}`}>
                    {problem.severity}
                  </span>
                  <span className="mono text-[10.5px] text-[var(--muted-2)]">
                    {problem.file}:{problem.line}
                  </span>
                </div>
                <div className="mt-1 text-[12px] leading-snug">{problem.message}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </Modal>
  );
}

/* ── Conflicts ────────────────────────────────────────────────────────── */

function ConflictsModal() {
  const { t } = useTranslation();
  const open = useStudio((s) => s.modal) === "conflicts";
  const close = useStudio((s) => s.closeModal);
  const report = useStudio((s) => s.lastReport);
  const exportProject = useStudio((s) => s.exportProject);

  return (
    <Modal
      open={open}
      onClose={close}
      title={t("conflict.title")}
      subtitle={t("conflict.text")}
      width={620}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t("conflict.skip")}
          </Button>
          <Button
            variant="primary"
            onClick={() => {
              void exportProject(true);
              close();
            }}
          >
            {t("conflict.overwrite")}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-1.5">
        {(report?.conflicts ?? []).map((conflict) => (
          <div key={conflict.path} className="rounded-lg border border-[var(--border)] bg-[var(--panel-2)] px-3 py-2">
            <div className="mono text-[11.5px]">{conflict.path}</div>
            <div className="mt-0.5 text-[11px] text-[var(--muted)]">{conflict.reason}</div>
          </div>
        ))}
      </div>
    </Modal>
  );
}

/* ── External diff ─────────────────────────────────────────────────── */

function DiffModal() {
  const { t } = useTranslation();
  const open = useStudio((s) => s.modal) === "diff";
  const close = useStudio((s) => s.closeModal);
  const diffs = useStudio((s) => s.diffs);
  const reload = useStudio((s) => s.reloadProject);

  return (
    <Modal
      open={open}
      onClose={close}
      title={t("diff.title")}
      width={860}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t("common.close")}
          </Button>
          <Button
            variant="primary"
            onClick={() => {
              void reload();
              close();
            }}
          >
            {t("diff.reload")}
          </Button>
        </>
      }
    >
      {!diffs || diffs.length === 0 ? (
        <div className="rounded-xl border border-dashed border-[var(--border)] px-4 py-8 text-center text-[12px] text-[var(--muted-2)]">
          {t("diff.empty")}
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          {diffs.map((diff) => (
            <div key={diff.path} className="overflow-hidden rounded-lg border border-[var(--border)]">
              <div className="flex items-center justify-between gap-2 border-b border-[var(--border)] bg-[var(--panel-2)] px-3 py-1.5">
                <span className="mono truncate text-[11.5px]">{diff.path}</span>
                <span className="chip">{t(`diff.${diff.status}`)}</span>
              </div>
              <pre className="mono max-h-[220px] overflow-auto px-3 py-2 text-[10.5px] leading-relaxed">
                {diff.lines.map((line, index) => (
                  <div
                    key={index}
                    className={
                      line.startsWith("+")
                        ? "text-[var(--ok)]"
                        : line.startsWith("-")
                          ? "text-[var(--danger)]"
                          : "text-[var(--muted)]"
                    }
                  >
                    {line || " "}
                  </div>
                ))}
              </pre>
            </div>
          ))}
        </div>
      )}
    </Modal>
  );
}

/* ── Plugin docs ───────────────────────────────────────────────────── */

function PluginDocsModal() {
  const { t } = useTranslation();
  const open = useStudio((s) => s.modal) === "pluginDocs";
  const close = useStudio((s) => s.closeModal);
  const docs = useStudio((s) => s.pluginDocs);
  return (
    <Modal
      open={open}
      onClose={close}
      title={docs?.name ?? t("plugins.docs")}
      width={820}
      footer={
        <Button variant="ghost" onClick={close}>
          {t("common.close")}
        </Button>
      }
    >
      <pre className="mono max-h-[520px] whitespace-pre-wrap text-[11.5px] leading-relaxed text-[var(--muted)]">
        {docs?.text ?? "—"}
      </pre>
    </Modal>
  );
}

/* ── Import ───────────────────────────────────────────────────────────── */

function ImportModal() {
  const { t } = useTranslation();
  const open = useStudio((s) => s.modal) === "import";
  const close = useStudio((s) => s.closeModal);
  const importProject = useStudio((s) => s.importProject);
  const project = useStudio((s) => s.project);
  const busy = useStudio((s) => s.busy);
  const [folder, setFolder] = useState("");
  const [report, setReport] = useState<ImportReport | null>(null);
  const [scanning, setScanning] = useState(false);

  const choose = async () => {
    const picked = await pickFolder(t("import.folder"));
    if (picked) {
      setFolder(picked);
      setReport(null);
      setScanning(true);
      try {
        const result = await api.importScan(picked);
        setReport(result);
      } catch (error) {
        useStudio.getState().toast("error", String(error));
      } finally {
        setScanning(false);
      }
    }
  };

  const rows = useMemo(
    () =>
      report
        ? [
            { label: t("import.files"), value: report.files },
            { label: t("import.scenes"), value: report.scenes },
            { label: t("import.recognized"), value: report.recognized },
            { label: t("import.code"), value: report.codeBlocks },
            { label: t("import.characters"), value: report.characters },
            { label: t("import.variables"), value: report.variables },
          ]
        : [],
    [report, t],
  );

  return (
    <Modal
      open={open}
      onClose={close}
      title={t("import.title")}
      subtitle={t("import.backupNote")}
      width={680}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            {t("common.cancel")}
          </Button>
          {project ? (
            <Button
              variant="danger"
              disabled={!report || report.scenes === 0 || busy}
              onClick={() => {
                if (window.confirm(t("import.replaceConfirm"))) {
                  void importProject(folder, "replace");
                }
              }}
            >
              {t("import.replace")}
            </Button>
          ) : null}
          <Button
            variant="primary"
            disabled={!report || report.scenes === 0 || busy}
            onClick={() => void importProject(folder, project ? "merge" : "replace")}
          >
            {busy ? <Spinner /> : null}
            {project ? t("import.applyMerge") : t("import.applyReplace")}
          </Button>
        </>
      }
    >
      <Field label={t("import.folder")}>
        <div className="flex gap-2">
          <Input value={folder} readOnly placeholder="C:\\Games\\MyNovel" />
          <Button variant="ghost" onClick={() => void choose()}>
            <FolderOpen size={14} /> {t("import.choose")}
          </Button>
        </div>
      </Field>

      {scanning ? (
        <div className="flex items-center gap-2 py-6 text-[12px] text-[var(--muted)]">
          <Loader2 size={14} className="animate-spin" /> {t("app.loading")}
        </div>
      ) : null}

      {report ? (
        <div>
          <div className="mb-3 grid grid-cols-3 gap-2">
            {rows.map((row) => (
              <div key={row.label} className="rounded-lg border border-[var(--border)] bg-[var(--panel-2)] px-3 py-2">
                <div className="text-[10px] uppercase tracking-wider text-[var(--muted-2)]">{row.label}</div>
                <div className="text-[18px] font-semibold">{row.value}</div>
              </div>
            ))}
          </div>
          {report.warnings.length > 0 ? (
            <div className="mb-3">
              <div className="mb-1 text-[11px] font-semibold text-[var(--warn)]">{t("import.warnings")}</div>
              <div className="flex max-h-[140px] flex-col gap-1 overflow-auto">
                {report.warnings.map((warning, index) => (
                  <div key={index} className="rounded border border-[var(--border)] px-2 py-1 text-[11px] text-[var(--muted)]">
                    {warning}
                  </div>
                ))}
              </div>
            </div>
          ) : null}
          {report.skippedFiles.length > 0 ? (
            <p className="text-[11px] text-[var(--muted-2)]">
              {t("import.skipped")}: {report.skippedFiles.join(", ")}
            </p>
          ) : null}
        </div>
      ) : null}
    </Modal>
  );
}
