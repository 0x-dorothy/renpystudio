import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import i18n, { setLanguage } from "./i18n";
import { slugify } from "./util";
import type {
  AssetInfo,
  Bootstrap,
  Character,
  DiffEntry,
  ExternalChange,
  GraphData,
  Issue,
  LintPayload,
  Meta,
  Project,
  Scene,
  Settings,
  SdkInfo,
  SnapshotInfo,
  StateEnvelope,
  StoreStatus,
  UiConfig,
  ValidationReport,
  Variable,
} from "./types";

export type Tab = "scene" | "graph" | "characters" | "variables" | "screens" | "assets" | "plugins";
export type ModalKind =
  | "none"
  | "settings"
  | "history"
  | "newProject"
  | "import"
  | "build"
  | "lint"
  | "conflicts"
  | "diff"
  | "pluginDocs";

export interface Toast {
  id: number;
  kind: "success" | "error" | "info";
  message: string;
}

interface StudioState {
  ready: boolean;
  version: string;
  settings: Settings;
  sdk: SdkInfo | null;
  project: Project | null;
  status: StoreStatus | null;
  issues: ValidationReport | null;
  graph: GraphData | null;
  assets: AssetInfo[];
  snapshots: SnapshotInfo[];
  lint: LintPayload | null;
  lastReport: import("./types").ExportReport | null;
  tab: Tab;
  activeSceneId: string | null;
  selectedNodeId: string | null;
  selectedCharacterId: string | null;
  selectedVariableId: string | null;
  paletteIndex: number | null;
  editLocale: string;
  modal: ModalKind;
  busy: boolean;
  busyLabel: string | null;
  externalChange: ExternalChange | null;
  diffs: import("./types").FileDiff[] | null;
  plugins: import("./types").PluginInfo[];
  pluginsRoot: string;
  pluginDocs: { name: string; text: string } | null;
  toasts: Toast[];
  mcp: { running: boolean; port?: number; url?: string };
  welcomeStep: "home" | "new" | "open";

  init: () => Promise<void>;
  applyEnvelope: (env: StateEnvelope, opts?: { reset?: boolean }) => void;
  refreshGraph: () => Promise<void>;
  refreshAssets: () => Promise<void>;
  refreshSnapshots: () => Promise<void>;
  refreshState: () => Promise<void>;

  setTab: (tab: Tab) => void;
  selectScene: (sceneId: string | null) => void;
  selectNode: (nodeId: string | null) => void;
  selectCharacter: (id: string | null) => void;
  selectVariable: (id: string | null) => void;
  setEditLocale: (locale: string) => void;
  openPalette: (index: number) => void;
  closePalette: () => void;
  openModal: (modal: ModalKind) => void;
  closeModal: () => void;
  setWelcomeStep: (step: "home" | "new" | "open") => void;

  toast: (kind: Toast["kind"], message: string) => void;
  dismissToast: (id: number) => void;
  dismissExternal: () => void;

  save: () => Promise<void>;
  undo: () => Promise<void>;
  redo: () => Promise<void>;
  run: () => Promise<void>;
  stop: () => Promise<void>;
  lintProject: () => Promise<void>;
  exportProject: (force?: boolean) => Promise<void>;
  buildRelease: (pkg: string) => Promise<void>;

  createScene: (title: string, index?: number) => Promise<void>;
  updateScene: (sceneId: string, patch: Partial<{ title: string; summary: string; color: string; slugValue: string; labelName: string }>) => Promise<void>;
  deleteScene: (sceneId: string) => Promise<void>;
  duplicateScene: (sceneId: string) => Promise<void>;
  reorderScene: (sceneId: string, toIndex: number) => Promise<void>;
  setStartScene: (sceneId: string) => Promise<void>;

  addNode: (kind: string, index?: number) => Promise<void>;
  updateNodeParams: (nodeId: string, params: Record<string, unknown>) => Promise<void>;
  deleteNode: (nodeId: string) => Promise<void>;
  duplicateNode: (nodeId: string) => Promise<void>;
  moveNode: (nodeId: string, toIndex: number) => Promise<void>;

  saveCharacter: (character: Character) => Promise<void>;
  deleteCharacter: (id: string) => Promise<void>;
  saveVariable: (variable: Variable) => Promise<void>;
  deleteVariable: (id: string) => Promise<void>;
  saveMeta: (meta: Meta) => Promise<void>;
  saveUi: (ui: UiConfig) => Promise<void>;

  saveSettings: (settings: Settings) => Promise<void>;
  detectSdk: (force?: boolean) => Promise<void>;
  setSdkPath: (path: string | null) => Promise<void>;
  startMcp: (port?: number) => Promise<void>;
  stopMcp: () => Promise<void>;

  takeSnapshot: (label: string) => Promise<void>;
  restoreSnapshot: (file: string) => Promise<void>;
  diffSnapshot: (file: string) => Promise<DiffEntry[]>;

  openProject: (path: string) => Promise<void>;
  importProject: (path: string, mode: "replace" | "merge") => Promise<void>;
  importAssets: (paths: string[]) => Promise<void>;
  translateProject: (locale: string) => Promise<void>;
  openExternalDiff: () => Promise<void>;
  retargetEdge: (args: {
    sceneId: string;
    nodeId: string;
    target: string | null;
    optionId?: string | null;
    branch?: "then" | "else" | null;
  }) => Promise<void>;
  refreshPlugins: () => Promise<void>;
  enablePlugin: (id: string) => Promise<void>;
  disablePlugin: (id: string) => Promise<void>;
  showPluginDocs: (id: string) => Promise<void>;
  insertPluginSnippet: (snippet: string) => Promise<void>;
  ensureCharacterFolder: (character: import("./types").Character) => Promise<void>;
  newProject: (parent: string, title: string) => Promise<void>;
  createInExisting: (path: string, title: string) => Promise<void>;
  closeProject: () => Promise<void>;
  reloadProject: () => Promise<void>;
  focusIssue: (issue: Issue) => void;
}

const defaultSettings: Settings = {
  locale: "ru",
  theme: "dark",
  accent: "violet",
  sdkPath: null,
  autosaveSeconds: 30,
  mcpEnabled: false,
  mcpPort: 8791,
  mcpRequireConfirm: true,
  graphDetail: "scenes",
  lastProject: null,
  recentProjects: [],
  ai: { baseUrl: "", apiKey: "", model: "" },
  editorFontSize: 16,
  showBlockIds: false,
};

export function applyTheme(settings: Settings) {
  const root = document.documentElement;
  root.classList.toggle("dark", settings.theme === "dark");
  root.classList.toggle("light", settings.theme === "light");
  root.dataset.accent = settings.accent || "violet";
}

let toastSeq = 1;
const errText = (error: unknown): string =>
  typeof error === "string" ? error : error instanceof Error ? error.message : String(error);

export const useStudio = create<StudioState>((set, get) => {
  const applyEnvelope = (env: StateEnvelope, opts?: { reset?: boolean }) => {
    const project = env.project;
    const known = (id: string | null) => !!id && project.scenes.some((s) => s.id === id);
    const reset = opts?.reset === true;
    const activeSceneId =
      !reset && known(get().activeSceneId)
        ? (get().activeSceneId as string)
        : project.meta.startScene && project.scenes.some((s) => s.id === project.meta.startScene)
          ? project.meta.startScene
          : (project.scenes[0]?.id ?? null);
    const activeScene = project.scenes.find((s) => s.id === activeSceneId) ?? null;
    const nodeExists = !!get().selectedNodeId && !!activeScene?.nodes.some((n) => n.id === get().selectedNodeId);
    set({
      project,
      status: env.status,
      issues: env.issues,
      activeSceneId,
      selectedNodeId: reset ? null : nodeExists ? get().selectedNodeId : null,
    });
  };

  const fail = (error: unknown) => {
    get().toast("error", errText(error));
    return null;
  };

  const guarded = async <T,>(fn: () => Promise<T>, okMessage?: string): Promise<T | null> => {
    set({ busy: true });
    try {
      const result = await fn();
      if (okMessage) get().toast("success", okMessage);
      return result;
    } catch (error) {
      return fail(error);
    } finally {
      set({ busy: false });
    }
  };

  const applyProject = async (fn: () => Promise<StateEnvelope>) => {
    const env = await guarded(fn);
    if (env) {
      applyEnvelope(env);
      if (get().tab === "graph") void get().refreshGraph();
    }
  };

  const init = async () => {
    try {
      const boot: Bootstrap = await api.bootstrap();
      setLanguage(boot.settings.locale);
      applyTheme(boot.settings);
      set({
        ready: true,
        version: boot.version,
        settings: boot.settings,
        sdk: boot.sdk,
        mcp: { running: boot.mcpRunning, port: boot.mcpPort ?? undefined },
      });
      if (boot.project) {
        applyEnvelope(boot.project, { reset: true });
        set({ editLocale: boot.project.project.meta.defaultLocale || boot.settings.locale });
        void get().refreshAssets();
        void get().refreshGraph();
      }
      void listen<ExternalChange>("studio://external-change", (event) => {
        const change = event.payload;
        set({ externalChange: change });
        if (change.assets.length > 0) void get().refreshAssets();
      });
      void listen<{ reason: string }>("studio://project-changed", async (event) => {
        if (event.payload?.reason === "mcp") {
          await get().refreshState();
          get().toast("info", i18n.t("toast.mcpChanged"));
        }
      });
      void listen<{ at: string }>("studio://saved", (event) => {
        const status = get().status;
        if (status) set({ status: { ...status, lastSaved: event.payload.at, dirty: false } });
      });
      void listen("studio://assets-changed", () => {
        void get().refreshAssets();
      });
    } catch (error) {
      set({ ready: true });
      fail(error);
    }
  };

  const refreshState = async () => {
    try {
      const env = await api.projectState();
      if (env) applyEnvelope(env);
    } catch {
      /* ignore */
    }
  };

  return {
    ready: false,
    version: "0.1.0",
    settings: defaultSettings,
    sdk: null,
    project: null,
    status: null,
    issues: null,
    graph: null,
    assets: [],
    snapshots: [],
    lint: null,
    lastReport: null,
    tab: "scene",
    activeSceneId: null,
    selectedNodeId: null,
    selectedCharacterId: null,
    selectedVariableId: null,
    paletteIndex: null,
    editLocale: "ru",
    modal: "none",
    busy: false,
    busyLabel: null,
    externalChange: null,
    diffs: null,
    plugins: [],
    pluginsRoot: "",
    pluginDocs: null,
    toasts: [],
    mcp: { running: false },
    welcomeStep: "home",

    init,
    applyEnvelope,
    refreshState,

    refreshGraph: async () => {
      try {
        const graph = await api.graph();
        set({ graph });
      } catch {
        /* no project */
      }
    },
    refreshAssets: async () => {
      try {
        const assets = await api.assets();
        set({ assets });
      } catch {
        /* no project */
      }
    },
    refreshSnapshots: async () => {
      try {
        const snapshots = await api.snapshots();
        set({ snapshots });
      } catch (error) {
        fail(error);
      }
    },

    setTab: (tab) => set({ tab }),
    selectScene: (sceneId) =>
      set({ activeSceneId: sceneId, selectedNodeId: null, tab: get().tab === "graph" ? "graph" : "scene" }),
    selectNode: (nodeId) => set({ selectedNodeId: nodeId }),
    selectCharacter: (id) => set({ selectedCharacterId: id }),
    selectVariable: (id) => set({ selectedVariableId: id }),
    setEditLocale: (locale) => set({ editLocale: locale }),
    openPalette: (index) => set({ paletteIndex: index }),
    closePalette: () => set({ paletteIndex: null }),
    openModal: (modal) => {
      if (modal === "history") void get().refreshSnapshots();
      set({ modal });
    },
    closeModal: () => set({ modal: "none" }),
    setWelcomeStep: (step) => set({ welcomeStep: step }),

    toast: (kind, message) => {
      const id = toastSeq++;
      set({ toasts: [...get().toasts, { id, kind, message }] });
      window.setTimeout(() => get().dismissToast(id), kind === "error" ? 6500 : 3500);
    },
    dismissToast: (id) => set({ toasts: get().toasts.filter((t) => t.id !== id) }),
    dismissExternal: () => set({ externalChange: null }),

    save: async () => {
      const status = await guarded(() => api.save());
      if (status) set({ status });
    },
    undo: async () => {
      await applyProject(() => api.undo());
    },
    redo: async () => {
      await applyProject(() => api.redo());
    },

    run: async () => {
      set({ busy: true, busyLabel: i18n.t("busy.run") });
      try {
        const result = await api.run();
        if (!result.ok) {
          get().toast("error", result.message);
        } else {
          get().toast("success", i18n.t("toast.runStarted"));
        }
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },
    stop: async () => {
      const stopped = await guarded(() => api.stop());
      if (stopped) get().toast("info", i18n.t("action.stopped"));
    },
    lintProject: async () => {
      set({ busy: true, busyLabel: i18n.t("busy.lint") });
      try {
        const lint = await api.lint();
        set({ lint, modal: "lint" });
        if (lint.report.problems.length === 0) get().toast("success", i18n.t("toast.lintOk"));
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },
    exportProject: async (force = false) => {
      set({ busy: true, busyLabel: i18n.t("busy.export") });
      try {
        const report = await api.exportProject(force);
        set({ lastReport: report });
        if (report.conflicts.length > 0) {
          set({ modal: "conflicts" });
          get().toast("error", i18n.t("toast.conflicts", { count: report.conflicts.length }));
        } else {
          get().toast("success", i18n.t("toast.exported", { count: report.written.length }));
        }
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },
    buildRelease: async (pkg) => {
      set({ busy: true, busyLabel: i18n.t("busy.build") });
      try {
        const report = await api.build(pkg);
        if (report.ok) get().toast("success", i18n.t("toast.buildDone", { artifact: report.artifact ?? "dist" }));
        else get().toast("error", report.log.slice(-400) || i18n.t("toast.buildFailed"));
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },

    createScene: async (title, index) => {
      const before = new Set((get().project?.scenes ?? []).map((s) => s.id));
      await applyProject(() => api.createScene(title, index));
      const created = get().project?.scenes.find((s) => !before.has(s.id));
      if (created) set({ activeSceneId: created.id, selectedNodeId: null, tab: "scene" });
    },
    updateScene: async (sceneId, patch) => {
      await applyProject(() => api.updateScene(sceneId, patch));
    },
    deleteScene: async (sceneId) => {
      await applyProject(() => api.deleteScene(sceneId));
    },
    duplicateScene: async (sceneId) => {
      await applyProject(() => api.duplicateScene(sceneId));
    },
    reorderScene: async (sceneId, toIndex) => {
      await applyProject(() => api.reorderScene(sceneId, toIndex));
    },
    setStartScene: async (sceneId) => {
      await applyProject(() => api.setStartScene(sceneId));
      get().toast("success", i18n.t("toast.startUpdated"));
    },

    addNode: async (kind, index) => {
      const sceneId = get().activeSceneId;
      if (!sceneId) return;
      const before = new Set(
        get().project?.scenes.find((s) => s.id === sceneId)?.nodes.map((n) => n.id) ?? [],
      );
      const env = await guarded(() => api.addNode(sceneId, kind, index));
      if (env) {
        applyEnvelope(env);
        const scene = env.project.scenes.find((s) => s.id === sceneId);
        const inserted = scene?.nodes.find((n) => !before.has(n.id)) ?? null;
        set({ selectedNodeId: inserted?.id ?? null });
        if (get().tab === "graph") void get().refreshGraph();
      }
    },
    updateNodeParams: async (nodeId, params) => {
      const sceneId = get().activeSceneId;
      if (!sceneId) return;
      const env = await guarded(() => api.updateNode(sceneId, nodeId, params));
      if (env) {
        applyEnvelope(env);
        if (get().tab === "graph") void get().refreshGraph();
      }
    },
    deleteNode: async (nodeId) => {
      const sceneId = get().activeSceneId;
      if (!sceneId) return;
      await applyProject(() => api.deleteNode(sceneId, nodeId));
      if (get().selectedNodeId === nodeId) set({ selectedNodeId: null });
    },
    duplicateNode: async (nodeId) => {
      const sceneId = get().activeSceneId;
      if (!sceneId) return;
      const before = new Set(
        get().project?.scenes.find((s) => s.id === sceneId)?.nodes.map((n) => n.id) ?? [],
      );
      await applyProject(() => api.duplicateNode(sceneId, nodeId));
      const scene = get().project?.scenes.find((s) => s.id === sceneId);
      const copy = scene?.nodes.find((n) => !before.has(n.id));
      if (copy) set({ selectedNodeId: copy.id });
    },
    moveNode: async (nodeId, toIndex) => {
      const sceneId = get().activeSceneId;
      if (!sceneId) return;
      await applyProject(() => api.moveNode(sceneId, nodeId, toIndex));
    },

    saveCharacter: async (character) => {
      await applyProject(() => api.upsertCharacter(character));
    },
    deleteCharacter: async (id) => {
      await applyProject(() => api.deleteCharacter(id));
      set({ selectedCharacterId: null });
    },
    saveVariable: async (variable) => {
      await applyProject(() => api.upsertVariable(variable));
    },
    deleteVariable: async (id) => {
      await applyProject(() => api.deleteVariable(id));
      set({ selectedVariableId: null });
    },
    saveMeta: async (meta) => {
      await applyProject(() => api.updateMeta(meta));
    },
    saveUi: async (ui) => {
      await applyProject(() => api.updateUi(ui));
    },

    saveSettings: async (settings) => {
      const saved = await guarded(() => api.saveSettings(settings));
      if (saved) {
        setLanguage(saved.locale);
        applyTheme(saved);
        set({ settings: saved });
        get().toast("success", i18n.t("settings.saved"));
      }
    },
    detectSdk: async (force = false) => {
      const info = await guarded(() => api.detectSdk(force));
      if (info) {
        set({ sdk: info });
        get().toast("success", i18n.t("settings.sdkUpdated", { version: info.version }));
      } else {
        get().toast("error", i18n.t("error.sdkMissing"));
      }
    },
    setSdkPath: async (path) => {
      const info = await guarded(() => api.setSdkPath(path));
      if (info) {
        set({ sdk: info });
        get().toast("success", i18n.t("settings.sdkUpdated", { version: info.version }));
      } else if (path) {
        get().toast("error", i18n.t("error.sdkNoExe"));
      }
    },
    startMcp: async (port) => {
      const result = await guarded(() => api.mcpStart(port));
      if (result) {
        set({ mcp: { running: true, port: result.port, url: `http://127.0.0.1:${result.port}/mcp` } });
        get().toast("success", i18n.t("toast.mcpStarted", { url: `http://127.0.0.1:${result.port}/mcp` }));
      }
    },
    stopMcp: async () => {
      const result = await guarded(() => api.mcpStop());
      if (result) {
        set({ mcp: { running: false } });
        get().toast("info", i18n.t("toast.mcpStopped"));
      }
    },

    takeSnapshot: async (label) => {
      const info = await guarded(() => api.snapshot(label || i18n.t("history.snapshot")));
      if (info) {
        get().toast("success", i18n.t("toast.snapshot"));
        void get().refreshSnapshots();
      }
    },
    restoreSnapshot: async (file) => {
      const env = await guarded(() => api.restore(file));
      if (env) {
        applyEnvelope(env);
        void get().refreshSnapshots();
        void get().refreshGraph();
        get().toast("success", i18n.t("toast.restored"));
      }
    },
    diffSnapshot: async (file) => {
      try {
        return await api.snapshotDiff(file);
      } catch (error) {
        fail(error);
        return [];
      }
    },

    openProject: async (path) => {
      const env = await guarded(() => api.openProject(path));
      if (env) {
        applyEnvelope(env, { reset: true });
        set({ tab: "scene", welcomeStep: "home", editLocale: env.project.meta.defaultLocale || "ru" });
        void get().refreshAssets();
        void get().refreshGraph();
      }
    },
    importProject: async (path, mode) => {
      set({ busy: true, busyLabel: i18n.t("busy.import") });
      try {
        const payload = await api.importApply(path, mode);
        await get().refreshState();
        void get().refreshAssets();
        void get().refreshGraph();
        set({ modal: "none", tab: "scene" });
        get().toast(
          "success",
          i18n.t("toast.imported", { scenes: payload.report.scenes, blocks: payload.report.recognized }),
        );
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },
    importAssets: async (paths) => {
      if (paths.length === 0) return;
      set({ busy: true });
      try {
        const report = await api.importAssets(paths);
        set({ assets: report.assets });
        void get().refreshGraph();
        if (report.imported.length > 0) {
          get().toast("success", i18n.t("toast.assetsImported", { count: report.imported.length }));
        }
        if (report.skipped.length > 0) {
          get().toast("error", i18n.t("toast.assetsSkipped", { count: report.skipped.length }));
        }
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false });
      }
    },
    translateProject: async (locale) => {
      set({ busy: true, busyLabel: i18n.t("busy.translate") });
      try {
        const report = await api.translate(locale);
        if (report.ok) {
          get().toast("success", i18n.t("toast.translated", { locale, count: report.files }));
        } else {
          get().toast("error", report.log.trim().slice(-400) || i18n.t("toast.translateFailed"));
        }
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },
    openExternalDiff: async () => {
      const change = get().externalChange;
      if (!change || change.rpy.length === 0) return;
      const diffs = await guarded(() => api.externalDiff(change.rpy));
      if (diffs) set({ diffs, modal: "diff" });
    },
    retargetEdge: async (args) => {
      const env = await guarded(() => api.retargetEdge(args));
      if (env) {
        applyEnvelope(env);
        void get().refreshGraph();
      }
    },
    refreshPlugins: async () => {
      try {
        const payload = await api.pluginsList();
        set({ plugins: payload.plugins, pluginsRoot: payload.root });
      } catch {
        /* plugins folder may be absent */
      }
    },
    enablePlugin: async (id) => {
      const env = await guarded(() => api.pluginEnable(id));
      if (env) {
        applyEnvelope(env);
        await get().refreshPlugins();
        get().toast("success", i18n.t("plugins.enabled"));
      }
    },
    disablePlugin: async (id) => {
      const env = await guarded(() => api.pluginDisable(id));
      if (env) {
        applyEnvelope(env);
        await get().refreshPlugins();
        get().toast("info", i18n.t("plugins.disabled"));
      }
    },
    showPluginDocs: async (id) => {
      const text = await guarded(() => api.pluginDocs(id));
      if (text) {
        const name = get().plugins.find((plugin) => plugin.id === id)?.name ?? id;
        set({ pluginDocs: { name, text }, modal: "pluginDocs" });
      } else {
        get().toast("info", i18n.t("plugins.noDocs"));
      }
    },
    insertPluginSnippet: async (snippet) => {
      const sceneId = get().activeSceneId;
      if (!sceneId) {
        get().toast("error", i18n.t("error.projectNotOpen"));
        return;
      }
      const env = await guarded(() =>
        api.addNode(sceneId, "code", undefined, { code: snippet, language: "renpy" }),
      );
      if (env) {
        applyEnvelope(env);
        get().toast("success", i18n.t("plugins.snippetAdded"));
      }
    },
    ensureCharacterFolder: async (character) => {
      const folder = character.spriteFolder?.trim() || `characters/${slugify(character.name || "hero")}`;
      try {
        const absolute = await api.ensureSpriteFolder(folder);
        await get().saveCharacter({ ...character, spriteFolder: folder });
        await api.revealInExplorer(absolute);
      } catch (error) {
        fail(error);
      }
    },
    newProject: async (parent, title) => {
      set({ busy: true, busyLabel: i18n.t("busy.create") });
      try {
        const env = await api.newProject(parent, title);
        applyEnvelope(env, { reset: true });
        set({ tab: "scene", modal: "none", welcomeStep: "home" });
        void get().refreshAssets();
        void get().refreshGraph();
        get().toast("success", i18n.t("toast.projectCreated"));
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },
    createInExisting: async (path, title) => {
      const env = await guarded(() => api.createInExisting(path, title));
      if (env) {
        applyEnvelope(env);
        set({ tab: "scene", modal: "none" });
        void get().refreshAssets();
        void get().refreshGraph();
      }
    },
    closeProject: async () => {
      await guarded(() => api.closeProject());
      set({ project: null, status: null, issues: null, graph: null, assets: [], activeSceneId: null, selectedNodeId: null });
    },
    reloadProject: async () => {
      const env = await guarded(() => api.reload());
      if (env) {
        applyEnvelope(env);
        set({ externalChange: null });
        void get().refreshAssets();
        void get().refreshGraph();
        get().toast("success", i18n.t("toast.reloaded"));
      }
    },
    focusIssue: (issue) => {
      if (issue.sceneId) {
        set({ activeSceneId: issue.sceneId, tab: "scene", selectedNodeId: issue.nodeId ?? null });
      }
    },
  };
});

export const selectActiveScene = (state: StudioState): Scene | null => {
  if (!state.project || !state.activeSceneId) return null;
  return state.project.scenes.find((s) => s.id === state.activeSceneId) ?? null;
};
