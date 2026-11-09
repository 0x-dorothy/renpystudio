import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  FileCode2,
  GitBranch,
  Image as ImageIcon,
  LayoutDashboard,
  SlidersHorizontal,
  UserRound,
} from "lucide-react";
import { useStudio, type Tab } from "./store";
import { TopBar } from "./components/TopBar";
import { StatusBar } from "./components/StatusBar";
import { Sidebar } from "./components/Sidebar";
import { Inspector } from "./components/Inspector";
import { SceneEditor } from "./components/SceneEditor";
import { GraphView } from "./components/GraphView";
import { CharactersTab } from "./components/CharactersTab";
import { VariablesTab } from "./components/VariablesTab";
import { AssetsTab } from "./components/AssetsTab";
import { ScreensTab } from "./components/ScreensTab";
import { Welcome } from "./components/Welcome";
import { StudioModals } from "./components/Modals";
import { HistoryModal } from "./components/HistoryModal";
import { ExternalChangeBanner, Toasts } from "./components/Toasts";
import { getCurrentWebview } from "@tauri-apps/api/webview";

const TABS: { id: Tab; icon: typeof FileCode2; labelKey: string }[] = [
  { id: "scene", icon: FileCode2, labelKey: "nav.scene" },
  { id: "graph", icon: GitBranch, labelKey: "nav.graph" },
  { id: "screens", icon: LayoutDashboard, labelKey: "nav.screens" },
  { id: "characters", icon: UserRound, labelKey: "nav.characters" },
  { id: "variables", icon: SlidersHorizontal, labelKey: "nav.variables" },
  { id: "assets", icon: ImageIcon, labelKey: "nav.assets" },
];

export default function App() {
  const { t } = useTranslation();
  const ready = useStudio((s) => s.ready);
  const project = useStudio((s) => s.project);
  const tab = useStudio((s) => s.tab);
  const setTab = useStudio((s) => s.setTab);
  const init = useStudio((s) => s.init);
  const editLocale = useStudio((s) => s.editLocale);
  const setEditLocale = useStudio((s) => s.setEditLocale);
  const [draggingFiles, setDraggingFiles] = useState(false);

  useEffect(() => {
    void init();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Files dragged from Explorer become project assets (game/images, game/audio, ...).
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          setDraggingFiles(true);
        } else if (payload.type === "leave") {
          setDraggingFiles(false);
        } else if (payload.type === "drop") {
          setDraggingFiles(false);
          if (payload.paths.length > 0 && useStudio.getState().project) {
            void useStudio.getState().importAssets(payload.paths);
          }
        }
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => undefined);
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (project && !project.locales.includes(editLocale)) {
      setEditLocale(project.meta.defaultLocale || project.locales[0] || "ru");
    }
  }, [project, editLocale, setEditLocale]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      const mod = event.ctrlKey || event.metaKey;
      if (mod && event.key.toLowerCase() === "s") {
        event.preventDefault();
        void useStudio.getState().save();
      } else if (mod && event.key.toLowerCase() === "z" && !event.shiftKey) {
        event.preventDefault();
        void useStudio.getState().undo();
      } else if (mod && (event.key.toLowerCase() === "y" || (event.shiftKey && event.key.toLowerCase() === "z"))) {
        event.preventDefault();
        void useStudio.getState().redo();
      } else if (event.key === "F5") {
        event.preventDefault();
        void useStudio.getState().run();
      } else if (mod && event.key.toLowerCase() === "e") {
        event.preventDefault();
        void useStudio.getState().exportProject();
      } else if (mod && event.key.toLowerCase() === "h") {
        event.preventDefault();
        useStudio.getState().openModal("history");
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const typing =
        !!target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.tagName === "SELECT" ||
          target.isContentEditable);
      if (typing) return;
      const state = useStudio.getState();
      const mod = event.ctrlKey || event.metaKey;
      const scene = state.project?.scenes.find((entry) => entry.id === state.activeSceneId) ?? null;
      if (mod && event.key.toLowerCase() === "k") {
        event.preventDefault();
        state.openPalette(scene?.nodes.length ?? 0);
      } else if ((event.key === "Delete" || event.key === "Backspace") && state.selectedNodeId) {
        event.preventDefault();
        void state.deleteNode(state.selectedNodeId);
      } else if (mod && event.key.toLowerCase() === "d" && state.selectedNodeId) {
        event.preventDefault();
        void state.duplicateNode(state.selectedNodeId);
      } else if (event.altKey && (event.key === "ArrowUp" || event.key === "ArrowDown") && state.selectedNodeId && scene) {
        event.preventDefault();
        const index = scene.nodes.findIndex((node) => node.id === state.selectedNodeId);
        if (index === -1) return;
        const next = event.key === "ArrowUp" ? Math.max(0, index - 1) : Math.min(scene.nodes.length - 1, index + 1);
        if (next !== index) void state.moveNode(state.selectedNodeId, next);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  if (!ready) {
    return (
      <div className="flex h-full items-center justify-center bg-[var(--bg)]">
        <div className="flex flex-col items-center gap-3">
          <div className="h-10 w-10 animate-pulse rounded-xl bg-[var(--accent)]/40" />
          <span className="text-[12px] text-[var(--muted)]">{t("app.loading")}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col bg-[var(--bg)] text-[var(--text)]">
      <TopBar />
      <ExternalChangeBanner />
      {draggingFiles ? (
        <div className="pointer-events-none fixed inset-0 z-[70] flex items-center justify-center border-4 border-dashed border-[var(--accent)]/60 bg-[var(--accent-soft)]/20">
          <div className="rounded-xl border border-[var(--accent)]/50 bg-[var(--panel)] px-4 py-3 text-[13px] font-medium shadow-2xl">
            {t("assets.dropHint")}
          </div>
        </div>
      ) : null}
      <div className="flex min-h-0 flex-1">
        {project ? (
          <>
            <Sidebar />
            <main className="flex min-w-0 flex-1 flex-col bg-[var(--bg)]">
              <div className="flex h-[40px] flex-none items-center gap-0.5 border-b border-[var(--border)] bg-[var(--panel)] px-3">
                {TABS.map((entry) => {
                  const Icon = entry.icon;
                  return (
                    <button
                      key={entry.id}
                      type="button"
                      onClick={() => setTab(entry.id)}
                      className={`tab ${tab === entry.id ? "active" : ""}`}
                    >
                      <Icon size={13} />
                      {t(entry.labelKey)}
                    </button>
                  );
                })}
                <div className="flex-1" />
                {project.locales.length > 1 ? (
                  <div className="flex items-center gap-1 text-[11px] text-[var(--muted-2)]">
                    <span>{t("app.textLocale")}</span>
                    {project.locales.map((locale) => (
                      <button
                        key={locale}
                        type="button"
                        onClick={() => setEditLocale(locale)}
                        className={`rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase ${
                          editLocale === locale
                            ? "bg-[var(--accent-soft)] text-[var(--accent)]"
                            : "hover:text-[var(--text)]"
                        }`}
                      >
                        {locale}
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
              <div className="min-h-0 flex-1">
                {tab === "scene" ? <SceneEditor /> : null}
                {tab === "graph" ? <GraphView /> : null}
                {tab === "screens" ? <ScreensTab /> : null}
                {tab === "characters" ? <CharactersTab /> : null}
                {tab === "variables" ? <VariablesTab /> : null}
                {tab === "assets" ? <AssetsTab /> : null}
              </div>
            </main>
            <Inspector />
          </>
        ) : (
          <Welcome />
        )}
      </div>
      <StatusBar />
      <StudioModals />
      <HistoryModal />
      <Toasts />
    </div>
  );
}
