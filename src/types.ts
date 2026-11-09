// Mirrors the Rust model (serde camelCase). Keep in sync with src-tauri/src/model.rs.

export type LocalizedText = Record<string, string>;

export type CharacterKind = "character" | "thought" | "narrator";
export type VarType = "number" | "bool" | "text";
export type NodeSource = "studio" | "mcp" | "import";
export type Position = "auto" | "left" | "center" | "right" | "far_left" | "far_right" | "custom";

export interface Meta {
  title: string;
  author: string;
  description: string;
  renpyVersion: string;
  gameSlug: string;
  startScene: string | null;
  defaultLocale: string;
  createdAt: string;
  updatedAt: string;
}

export interface Character {
  id: string;
  name: string;
  kind: CharacterKind;
  color: string | null;
  avatar: string | null;
  spritePrefix: string | null;
  voiceTag: string | null;
  prefix: string | null;
  suffix: string | null;
  notes: string;
}

export interface Variable {
  id: string;
  name: string;
  varType: VarType;
  default: unknown;
  description: string;
}

export interface StudioNode {
  id: string;
  kind: string;
  params: Record<string, unknown>;
  source: NodeSource;
  locked: boolean;
}

export interface Scene {
  id: string;
  title: string;
  slug: string;
  labelName?: string | null;
  summary: string;
  color: string | null;
  nodes: StudioNode[];
}

export interface MenuButtonFlags {
  showStart: boolean;
  showLoad: boolean;
  showPreferences: boolean;
  showAbout: boolean;
  showQuit: boolean;
}

export interface MainMenuConfig extends MenuButtonFlags {
  template: string;
  background: string | null;
  logo: string | null;
  title: LocalizedText;
  tagline: LocalizedText;
  music: string | null;
  accent: string;
  textColor: string;
  buttonAlign: "left" | "center" | "right";
}

export interface ScreenElement {
  id: string;
  kind: string;
  text: LocalizedText;
  image: string | null;
  action: string | null;
  x: number;
  y: number;
  style: string;
}

export interface ScreenDef {
  id: string;
  name: string;
  template: string;
  title: LocalizedText;
  elements: ScreenElement[];
}

export interface UiConfig {
  mainMenu: MainMenuConfig;
  screens: ScreenDef[];
}

export interface Project {
  formatVersion: number;
  meta: Meta;
  locales: string[];
  characters: Character[];
  variables: Variable[];
  scenes: Scene[];
  initScript: string;
  ui: UiConfig;
}

export interface StoreStatus {
  root: string | null;
  title: string;
  revision: number;
  dirty: boolean;
  canUndo: boolean;
  canRedo: boolean;
  lastSaved: string | null;
  sceneCount: number;
  blockCount: number;
}

export interface Issue {
  severity: "error" | "warning" | "info";
  code: string;
  message: string;
  sceneId: string | null;
  nodeId: string | null;
}

export interface ValidationReport {
  issues: Issue[];
  errors: number;
  warnings: number;
  unreachable: string[];
  deadEnds: string[];
}

export interface StateEnvelope {
  project: Project;
  status: StoreStatus;
  issues: ValidationReport;
}

export interface GraphEdge {
  id: string;
  fromScene: string;
  toScene: string | null;
  kind: "jump" | "choice" | "condition";
  label: string;
  nodeId: string;
  condition: string | null;
}

export interface GraphData {
  edges: GraphEdge[];
  deadEnds: string[];
  unreachable: string[];
  nodeCounts: Record<string, number>;
}

export interface AssetInfo {
  path: string;
  name: string;
  kind: "image" | "audio" | "font" | "video";
  size: number;
  extension: string;
}

export interface SdkInfo {
  root: string;
  exe: string;
  version: string;
  launcher: boolean;
}

export interface AiSettings {
  baseUrl: string;
  apiKey: string;
  model: string;
}

export interface Settings {
  locale: string;
  theme: "dark" | "light";
  accent: string;
  sdkPath: string | null;
  autosaveSeconds: number;
  mcpEnabled: boolean;
  mcpPort: number;
  mcpRequireConfirm: boolean;
  graphDetail: "scenes" | "choices" | "all";
  lastProject: string | null;
  recentProjects: { path: string; title: string; openedAt: string }[];
  ai: AiSettings;
  editorFontSize: number;
  showBlockIds: boolean;
}

export interface Bootstrap {
  settings: Settings;
  sdk: SdkInfo | null;
  project: StateEnvelope | null;
  recent: Settings["recentProjects"];
  version: string;
  mcpRunning: boolean;
  mcpPort: number | null;
}

export interface Conflict {
  path: string;
  reason: string;
}

export interface ExportReport {
  generated: string[];
  written: string[];
  skipped: string[];
  removed: string[];
  warnings: string[];
  conflicts: Conflict[];
  manifestPath: string;
  totalBlocks: number;
}

export interface LintProblem {
  file: string;
  line: number;
  message: string;
  severity: "error" | "warning";
}

export interface LintReport {
  ok: boolean;
  problems: LintProblem[];
  raw: string;
  ran: boolean;
  message: string;
}

export interface MappedProblem extends LintProblem {
  nodeId: string | null;
  sceneId: string | null;
  sceneTitle: string | null;
}

export interface LintPayload {
  report: LintReport;
  mapped: MappedProblem[];
}

export interface RunResult {
  ok: boolean;
  message: string;
  pid: string | null;
}

export interface BuildReport {
  ok: boolean;
  log: string;
  artifact: string | null;
}

export interface SnapshotInfo {
  id: string;
  file: string;
  createdAt: string;
  kind: string;
  label: string;
  revision: number;
  scenes: number;
  blocks: number;
}

export interface DiffEntry {
  kind: string;
  summary: string;
  sceneId: string | null;
  nodeId: string | null;
  details: string[];
}

export interface ImportReport {
  files: number;
  scenes: number;
  recognized: number;
  codeBlocks: number;
  characters: number;
  variables: number;
  warnings: string[];
  log: string[];
  skippedFiles: string[];
}

export interface ImportPayload {
  project: Project | null;
  report: ImportReport;
}

export interface ExternalChange {
  rpy: string[];
  assets: string[];
  projectFile: boolean;
}

export interface AssetImportReport {
  imported: string[];
  skipped: string[];
  assets: AssetInfo[];
}

export interface TranslateReport {
  ok: boolean;
  locale: string;
  log: string;
  files: number;
}

export interface BuildState {
  running: boolean;
  package: string;
  log: string[];
  ok: boolean | null;
  artifact: string | null;
}

export interface FileDiff {
  path: string;
  status: "modified" | "added" | "removed";
  lines: string[];
}
