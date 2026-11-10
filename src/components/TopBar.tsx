import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Box,
  ChevronDown,
  Clock,
  Download,
  FolderOpen,
  History,
  Import,
  Loader2,
  Play,
  Redo2,
  Settings,
  ShieldCheck,
  Sparkles,
  Square,
  Undo2,
} from "lucide-react";
import { useStudio } from "../store";
import { Button, IconButton } from "./ui";
import { api } from "../api";
import { pickFolder, revealProject } from "../dialogs";

export function TopBar() {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const status = useStudio((s) => s.status);
  const settings = useStudio((s) => s.settings);
  const busy = useStudio((s) => s.busy);
  const busyLabel = useStudio((s) => s.busyLabel);
  const openModal = useStudio((s) => s.openModal);
  const save = useStudio((s) => s.save);
  const undo = useStudio((s) => s.undo);
  const redo = useStudio((s) => s.redo);
  const run = useStudio((s) => s.run);
  const stop = useStudio((s) => s.stop);
  const exportProject = useStudio((s) => s.exportProject);
  const lintProject = useStudio((s) => s.lintProject);
  const openProject = useStudio((s) => s.openProject);
  const closeProject = useStudio((s) => s.closeProject);
  const sdk = useStudio((s) => s.sdk);
  const aiOpen = useStudio((s) => s.aiOpen);
  const toggleAi = useStudio((s) => s.toggleAi);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const [running, setRunning] = useState(false);

  useEffect(() => {
    const handler = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) setMenuOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const handleOpen = async () => {
    setMenuOpen(false);
    const folder = await pickFolder(t("menu.openProject"));
    if (folder) await openProject(folder);
  };

  const handleRun = async () => {
    if (running) {
      setRunning(false);
      await stop();
      return;
    }
    setRunning(true);
    await run();
    window.setTimeout(() => setRunning(false), 4000);
  };

  return (
    <header className="flex h-[46px] flex-none items-center justify-between gap-3 border-b border-[var(--border)] bg-[var(--panel)] px-3">
      <div className="flex min-w-0 items-center gap-2">
        <div className="relative" ref={menuRef}>
          <button
            type="button"
            onClick={() => setMenuOpen(!menuOpen)}
            className="flex items-center gap-1.5 rounded-lg px-2 py-1.5 text-[var(--muted)] hover:bg-[var(--panel-2)] hover:text-[var(--text)]"
          >
            <Box size={15} className="text-[var(--accent)]" />
            <span className="text-[12px] font-semibold tracking-tight">{t("app.name")}</span>
            <ChevronDown size={12} />
          </button>
          {menuOpen ? (
            <div className="absolute left-0 top-[38px] z-40 w-[248px] overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--panel)] py-1 shadow-2xl">
              <MenuItem
                icon={<FolderOpen size={14} />}
                label={t("menu.newProject")}
                onClick={() => {
                  setMenuOpen(false);
                  openModal("newProject");
                }}
              />
              <MenuItem
                icon={<FolderOpen size={14} />}
                label={t("menu.openProject")}
                onClick={handleOpen}
              />
              <MenuItem
                icon={<Import size={14} />}
                label={t("menu.importProject")}
                onClick={() => {
                  setMenuOpen(false);
                  openModal("import");
                }}
              />
              <div className="my-1 h-px bg-[var(--border)]" />
              <MenuItem
                icon={<Download size={14} />}
                label={t("menu.save")}
                hint="Ctrl+S"
                disabled={!project}
                onClick={() => {
                  setMenuOpen(false);
                  void save();
                }}
              />
              <MenuItem
                icon={<ShieldCheck size={14} />}
                label={t("menu.export")}
                disabled={!project}
                onClick={() => {
                  setMenuOpen(false);
                  void exportProject();
                }}
              />
              <MenuItem
                icon={<History size={14} />}
                label={t("menu.history")}
                disabled={!project}
                onClick={() => {
                  setMenuOpen(false);
                  openModal("history");
                }}
              />
              <div className="my-1 h-px bg-[var(--border)]" />
              <MenuItem
                icon={<Settings size={14} />}
                label={t("menu.settings")}
                onClick={() => {
                  setMenuOpen(false);
                  openModal("settings");
                }}
              />
              {project ? (
                <>
                  <MenuItem
                    icon={<FolderOpen size={14} />}
                    label={t("menu.reveal")}
                    onClick={() => {
                      setMenuOpen(false);
                      void revealProject();
                    }}
                  />
                  <MenuItem
                    icon={<Clock size={14} />}
                    label={t("menu.closeProject")}
                    onClick={() => {
                      setMenuOpen(false);
                      void closeProject();
                    }}
                  />
                </>
              ) : null}
            </div>
          ) : null}
        </div>

        {project ? (
          <div className="flex min-w-0 items-center gap-2 border-l border-[var(--border)] pl-3">
            <span className="truncate text-[13px] font-medium">{project.meta.title}</span>
            {status?.dirty ? (
              <span className="h-1.5 w-1.5 rounded-full bg-[var(--warn)]" title={t("status.notSaved")} />
            ) : null}
            {status ? <span className="chip">{t("status.revision", { revision: status.revision })}</span> : null}
          </div>
        ) : null}
      </div>

      <div className="flex items-center gap-1">
        {busy && busyLabel ? (
          <span className="mr-2 flex items-center gap-1.5 text-[11px] text-[var(--muted)]">
            <Loader2 size={12} className="animate-spin" /> {busyLabel}
          </span>
        ) : null}
        {project ? (
          <>
            <IconButton title={t("action.undo")} disabled={!status?.canUndo} onClick={() => void undo()}>
              <Undo2 size={15} />
            </IconButton>
            <IconButton title={t("action.redo")} disabled={!status?.canRedo} onClick={() => void redo()}>
              <Redo2 size={15} />
            </IconButton>
            <div className="mx-1 h-5 w-px bg-[var(--border)]" />
            <Button variant="ghost" size="sm" onClick={() => void lintProject()} title={t("action.lint")}>
              <ShieldCheck size={14} /> <span className="hidden lg:inline">{t("action.lint")}</span>
            </Button>
            <Button
              variant={running ? "danger" : "primary"}
              size="sm"
              onClick={() => void handleRun()}
              title={sdk ? undefined : t("error.sdkMissing")}
            >
              {running ? <Square size={13} /> : <Play size={14} />}
              {running ? t("action.stop") : t("action.run")}
            </Button>
            <Button variant="ghost" size="sm" onClick={() => openModal("build")} title={t("action.build")}>
              <Download size={14} /> <span className="hidden lg:inline">{t("action.build")}</span>
            </Button>
            <div className="mx-1 h-5 w-px bg-[var(--border)]" />
            <IconButton title={t("menu.history")} onClick={() => openModal("history")}>
              <History size={16} />
            </IconButton>
            <IconButton title={t("ai.title")} active={aiOpen} onClick={toggleAi}>
              <Sparkles size={16} />
            </IconButton>
            <IconButton title={t("menu.settings")} onClick={() => openModal("settings")}>
              <Settings size={16} />
            </IconButton>
          </>
        ) : (
          <Button variant="ghost" size="sm" onClick={() => openModal("settings")}>
            <Settings size={14} /> {t("menu.settings")}
          </Button>
        )}
      </div>
    </header>
  );
}

function MenuItem({
  icon,
  label,
  hint,
  onClick,
  disabled,
}: {
  icon: React.ReactNode;
  label: string;
  hint?: string;
  onClick: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className="flex w-full items-center gap-2.5 px-3 py-1.5 text-left text-[12.5px] text-[var(--text)] hover:bg-[var(--panel-2)] disabled:opacity-40"
    >
      <span className="text-[var(--muted)]">{icon}</span>
      <span className="flex-1">{label}</span>
      {hint ? <span className="text-[10px] text-[var(--muted-2)]">{hint}</span> : null}
    </button>
  );
}

export async function currentSettingsSdkCheck() {
  try {
    return await api.detectSdk(false);
  } catch {
    return null;
  }
}
