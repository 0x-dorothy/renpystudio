import { invoke } from "@tauri-apps/api/core";
import type {
  AssetInfo,
  Bootstrap,
  BuildReport,
  Character,
  DiffEntry,
  ExportReport,
  GraphData,
  ImportPayload,
  ImportReport,
  LintPayload,
  Meta,
  Project,
  RunResult,
  Scene,
  Settings,
  SdkInfo,
  SnapshotInfo,
  StateEnvelope,
  UiConfig,
  Variable,
  ValidationReport,
} from "./types";

export const api = {
  bootstrap: () => invoke<Bootstrap>("app_bootstrap"),
  saveSettings: (settings: Settings) => invoke<Settings>("settings_save", { settings }),
  detectSdk: (force = false) => invoke<SdkInfo | null>("sdk_detect", { force }),
  setSdkPath: (path: string | null) => invoke<SdkInfo | null>("sdk_set_path", { path }),

  newProject: (parent: string, title: string) =>
    invoke<StateEnvelope>("project_new", { parent, title }),
  openProject: (path: string) => invoke<StateEnvelope>("project_open", { path }),
  createInExisting: (path: string, title: string) =>
    invoke<StateEnvelope>("project_create_in_existing", { path, title }),
  projectState: () => invoke<StateEnvelope | null>("project_state"),
  closeProject: () => invoke<void>("project_close"),
  save: () => invoke<import("./types").StoreStatus>("project_save"),
  reload: () => invoke<StateEnvelope>("project_reload"),
  exportProject: (force = false) => invoke<ExportReport>("project_export", { force }),
  validate: () => invoke<ValidationReport>("project_validate"),
  graph: () => invoke<GraphData>("project_graph"),
  assets: () => invoke<AssetInfo[]>("assets_list"),

  undo: () => invoke<StateEnvelope>("undo"),
  redo: () => invoke<StateEnvelope>("redo"),

  createScene: (title: string, index?: number) =>
    invoke<StateEnvelope>("scene_create", { title, index }),
  updateScene: (sceneId: string, patch: Partial<{ title: string; summary: string; color: string; slugValue: string; labelName: string }>) =>
    invoke<StateEnvelope>("scene_update", { sceneId, ...patch }),
  deleteScene: (sceneId: string) => invoke<StateEnvelope>("scene_delete", { sceneId }),
  duplicateScene: (sceneId: string) => invoke<StateEnvelope>("scene_duplicate", { sceneId }),
  reorderScene: (sceneId: string, toIndex: number) =>
    invoke<StateEnvelope>("scene_reorder", { sceneId, toIndex }),
  setStartScene: (sceneId: string) => invoke<StateEnvelope>("scene_set_start", { sceneId }),

  addNode: (sceneId: string, kind: string, index?: number, params?: Record<string, unknown>) =>
    invoke<StateEnvelope>("node_add", { sceneId, kind, index, params }),
  updateNode: (sceneId: string, nodeId: string, params: Record<string, unknown>) =>
    invoke<StateEnvelope>("node_update", { sceneId, nodeId, params }),
  deleteNode: (sceneId: string, nodeId: string) =>
    invoke<StateEnvelope>("node_delete", { sceneId, nodeId }),
  moveNode: (sceneId: string, nodeId: string, toIndex: number) =>
    invoke<StateEnvelope>("node_move", { sceneId, nodeId, toIndex }),
  duplicateNode: (sceneId: string, nodeId: string) =>
    invoke<StateEnvelope>("node_duplicate", { sceneId, nodeId }),

  upsertCharacter: (character: Character) =>
    invoke<StateEnvelope>("character_upsert", { character }),
  deleteCharacter: (id: string) => invoke<StateEnvelope>("character_delete", { id }),
  upsertVariable: (variable: Variable) =>
    invoke<StateEnvelope>("variable_upsert", { variable }),
  deleteVariable: (id: string) => invoke<StateEnvelope>("variable_delete", { id }),
  updateMeta: (meta: Meta) => invoke<StateEnvelope>("meta_update", { meta }),
  updateUi: (ui: UiConfig) => invoke<StateEnvelope>("ui_update", { ui }),

  lint: () => invoke<LintPayload>("project_lint"),
  run: () => invoke<RunResult>("project_run"),
  stop: () => invoke<boolean>("project_stop"),
  build: (pkg: string, destination?: string) =>
    invoke<BuildReport>("project_build", { package: pkg, destination }),

  snapshots: () => invoke<SnapshotInfo[]>("snapshot_list"),
  snapshot: (label: string) => invoke<SnapshotInfo>("snapshot_create", { label }),
  restore: (file: string) => invoke<StateEnvelope>("snapshot_restore", { file }),
  snapshotDiff: (file: string) => invoke<DiffEntry[]>("snapshot_diff", { file }),

  importScan: (path: string) => invoke<ImportReport>("import_scan", { path }),
  importApply: (path: string, mode: "replace" | "merge") =>
    invoke<ImportPayload>("import_apply", { path, mode }),
  importAssets: (paths: string[]) =>
    invoke<import("./types").AssetImportReport>("asset_import", { paths }),
  translate: (locale: string) =>
    invoke<import("./types").TranslateReport>("project_translate", { locale }),
  buildStart: (pkg: string, destination?: string) =>
    invoke<void>("project_build_start", { package: pkg, destination }),
  buildStatus: () => invoke<import("./types").BuildState>("project_build_status"),
  externalDiff: (files: string[]) =>
    invoke<import("./types").FileDiff[]>("external_diff", { files }),
  retargetEdge: (args: {
    sceneId: string;
    nodeId: string;
    target: string | null;
    optionId?: string | null;
    branch?: "then" | "else" | null;
  }) => invoke<StateEnvelope>("node_retarget", args),
  pluginsList: () => invoke<import("./types").PluginsPayload>("plugins_list"),
  pluginEnable: (id: string) => invoke<StateEnvelope>("plugin_enable", { id }),
  pluginDisable: (id: string) => invoke<StateEnvelope>("plugin_disable", { id }),
  pluginDocs: (id: string) => invoke<string | null>("plugin_docs", { id }),
  pluginsRoot: () => invoke<string>("plugins_root"),
  ensureSpriteFolder: (relative: string) =>
    invoke<string>("ensure_sprite_folder", { relative }),

  mcpStart: (port?: number) => invoke<{ running: boolean; port: number }>("mcp_start", { port }),  mcpStop: () => invoke<{ running: boolean }>("mcp_stop"),
  mcpStatus: () => invoke<{ running: boolean; port?: number; url?: string }>("mcp_status"),

  aiModels: () => invoke<import("./types").AiModelInfo[]>("ai_models"),
  aiChat: (messages: { role: string; content: string }[], sceneId?: string | null) =>
    invoke<import("./types").AiReply>("ai_chat", { messages, sceneId }),
  aiForgetKey: () => invoke<Settings>("ai_forget_key"),
  previewScreen: (screen: string) => invoke<RunResult>("project_preview_screen", { screen }),

  revealInExplorer: (path: string) => invoke<void>("open_in_explorer", { path }),
  defaultParams: (kind: string) =>
    invoke<Record<string, unknown>>("default_block_params", { kind }),
  projectPath: () => invoke<string | null>("project_path"),
};

export type { Scene, Project, Settings };
