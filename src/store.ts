import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { setLanguage } from "./i18n";
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

export type Tab = "scene" | "graph" | "characters" | "variables" | "screens" | "assets";
export type ModalKind =
  | "none"
  | "settings"
  | "history"
  | "newProject"
  | "import"
  | "build"
  | "lint"
  | "conflicts";

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
  toasts: Toast[];
  mcp: { running: boolean; port?: number; url?: string };
  welcomeStep: "home" | "new" | "open";

  init: () => Promise<void>;
  applyEnvelope: (env: StateEnvelope, opts?: { keepSelection?: boolean }) => void;
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
  const applyEnvelope = (env: StateEnvelope, opts?: { keepSelection?: boolean }) => {
    const project = env.project;
    const known = (id: string | null) => !!id && project.scenes.some((s) => s.id === id);
    const activeSceneId = opts?.keepSelection && known(get().activeSceneId)
      ? get().activeSceneId
      : project.meta.startScene && project.scenes.some((s) => s.id === project.meta.startScene)
        ? project.meta.startScene
        : project.scenes[0]?.id ?? null;
    set({
      project,
      status: env.status,
      issues: env.issues,
      activeSceneId,
      selectedNodeId:
        opts?.keepSelection && known(activeSceneId) ? get().selectedNodeId : null,
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
        applyEnvelope(boot.project);
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
          get().toast("info", "Проект изменён через MCP");
        }
      });
      void listen<{ at: string }>("studio://saved", (event) => {
        const status = get().status;
        if (status) set({ status: { ...status, lastSaved: event.payload.at, dirty: false } });
      });
    } catch (error) {
      set({ ready: true });
      fail(error);
    }
  };

  const refreshState = async () => {
    try {
      const env = await api.projectState();
      if (env) applyEnvelope(env, { keepSelection: true });
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
      set({ busy: true, busyLabel: "Запуск игры…" });
      try {
        const result = await api.run();
        if (!result.ok) {
          get().toast("error", result.message);
        } else {
          get().toast("success", "Игра запущена");
        }
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },
    stop: async () => {
      const stopped = await guarded(() => api.stop());
      if (stopped) get().toast("info", "Игра остановлена");
    },
    lintProject: async () => {
      set({ busy: true, busyLabel: "Проверка SDK…" });
      try {
        const lint = await api.lint();
        set({ lint, modal: "lint" });
        if (lint.report.problems.length === 0) get().toast("success", "Проверка пройдена");
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },
    exportProject: async (force = false) => {
      set({ busy: true, busyLabel: "Экспорт .rpy…" });
      try {
        const report = await api.exportProject(force);
        set({ lastReport: report });
        if (report.conflicts.length > 0) {
          set({ modal: "conflicts" });
          get().toast("error", `Конфликтов: ${report.conflicts.length}`);
        } else {
          get().toast("success", `Экспортировано: ${report.written.length} файлов`);
        }
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },
    buildRelease: async (pkg) => {
      set({ busy: true, busyLabel: "Сборка релиза…" });
      try {
        const report = await api.build(pkg);
        if (report.ok) get().toast("success", `Сборка готова: ${report.artifact ?? "dist"}`);
        else get().toast("error", report.log.slice(-400) || "Сборка не удалась");
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },

    createScene: async (title, index) => {
      await applyProject(() => api.createScene(title, index));
      const project = get().project;
      const created = project?.scenes[project.scenes.length - 1];
      if (created) set({ activeSceneId: created.id, tab: "scene" });
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
      get().toast("success", "Стартовая сцена обновлена");
    },

    addNode: async (kind, index) => {
      const sceneId = get().activeSceneId;
      if (!sceneId) return;
      const env = await guarded(() => api.addNode(sceneId, kind, index));
      if (env) {
        applyEnvelope(env);
        const scene = env.project.scenes.find((s) => s.id === sceneId);
        const inserted = scene?.nodes[(index ?? (scene?.nodes.length ?? 1)) - 1] ?? null;
        set({ selectedNodeId: inserted?.id ?? null });
        if (get().tab === "graph") void get().refreshGraph();
      }
    },
    updateNodeParams: async (nodeId, params) => {
      const sceneId = get().activeSceneId;
      if (!sceneId) return;
      const env = await guarded(() => api.updateNode(sceneId, nodeId, params));
      if (env) {
        applyEnvelope(env, { keepSelection: true });
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
      await applyProject(() => api.duplicateNode(sceneId, nodeId));
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
        get().toast("success", "Настройки сохранены");
      }
    },
    detectSdk: async (force = false) => {
      const info = await guarded(() => api.detectSdk(force));
      if (info) {
        set({ sdk: info });
        get().toast("success", `SDK: ${info.version}`);
      } else {
        get().toast("error", "Ren'Py SDK не найден. Укажите папку вручную.");
      }
    },
    setSdkPath: async (path) => {
      const info = await guarded(() => api.setSdkPath(path));
      if (info) {
        set({ sdk: info });
        get().toast("success", `SDK: ${info.version}`);
      } else if (path) {
        get().toast("error", "В выбранной папке нет renpy.exe");
      }
    },
    startMcp: async (port) => {
      const result = await guarded(() => api.mcpStart(port));
      if (result) {
        set({ mcp: { running: true, port: result.port, url: `http://127.0.0.1:${result.port}/mcp` } });
        get().toast("success", `MCP-сервер: http://127.0.0.1:${result.port}/mcp`);
      }
    },
    stopMcp: async () => {
      const result = await guarded(() => api.mcpStop());
      if (result) {
        set({ mcp: { running: false } });
        get().toast("info", "MCP-сервер остановлен");
      }
    },

    takeSnapshot: async (label) => {
      const info = await guarded(() => api.snapshot(label || "Снимок"));
      if (info) {
        get().toast("success", "Снимок создан");
        void get().refreshSnapshots();
      }
    },
    restoreSnapshot: async (file) => {
      const env = await guarded(() => api.restore(file));
      if (env) {
        applyEnvelope(env);
        void get().refreshSnapshots();
        void get().refreshGraph();
        get().toast("success", "Проект восстановлен из снимка");
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
        applyEnvelope(env);
        set({ tab: "scene", welcomeStep: "home", editLocale: env.project.meta.defaultLocale || "ru" });
        void get().refreshAssets();
        void get().refreshGraph();
      }
    },
    importProject: async (path, mode) => {
      set({ busy: true, busyLabel: "Импорт .rpy…" });
      try {
        const payload = await api.importApply(path, mode);
        await get().refreshState();
        void get().refreshAssets();
        void get().refreshGraph();
        set({ modal: "none", tab: "scene" });
        get().toast(
          "success",
          `Импортировано: ${payload.report.scenes} сцен, ${payload.report.recognized} блоков`,
        );
      } catch (error) {
        fail(error);
      } finally {
        set({ busy: false, busyLabel: null });
      }
    },
    newProject: async (parent, title) => {
      set({ busy: true, busyLabel: "Создаю проект…" });
      try {
        const env = await api.newProject(parent, title);
        applyEnvelope(env);
        set({ tab: "scene", modal: "none", welcomeStep: "home" });
        void get().refreshAssets();
        void get().refreshGraph();
        get().toast("success", "Проект создан");
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
        applyEnvelope(env, { keepSelection: true });
        set({ externalChange: null });
        void get().refreshAssets();
        void get().refreshGraph();
        get().toast("success", "Проект перечитан с диска");
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
