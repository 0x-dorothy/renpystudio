import { useTranslation } from "react-i18next";
import { BookOpen, FolderOpen, Import, Plus, Sparkles } from "lucide-react";
import { useStudio } from "../store";
import { Button } from "./ui";
import { pickFolder } from "../dialogs";

export function Welcome() {
  const { t } = useTranslation();
  const openModal = useStudio((s) => s.openModal);
  const openProject = useStudio((s) => s.openProject);
  const recent = useStudio((s) => s.settings.recentProjects);
  const sdk = useStudio((s) => s.sdk);
  const version = useStudio((s) => s.version);

  const handleOpen = async () => {
    const folder = await pickFolder(t("menu.openProject"));
    if (folder) await openProject(folder);
  };

  return (
    <div className="scroll flex-1">
      <div className="mx-auto flex min-h-full max-w-[980px] flex-col justify-center px-10 py-12">
        <div className="mb-10 flex items-start gap-5">
          <div
            className="flex h-16 w-16 flex-none items-center justify-center rounded-2xl"
            style={{
              background: "linear-gradient(135deg, var(--accent), #e96aa5)",
              boxShadow: "0 18px 44px var(--accent-soft)",
            }}
          >
            <Sparkles size={28} className="text-white" />
          </div>
          <div>
            <h1 className="text-[28px] font-bold tracking-tight">{t("welcome.title")}</h1>
            <p className="mt-2 max-w-[560px] text-[13.5px] leading-relaxed text-[var(--muted)]">
              {t("welcome.subtitle")}
            </p>
            <div className="mt-3 flex items-center gap-2 text-[11.5px]">
              {sdk ? (
                <span className="chip chip-ok">{t("welcome.sdkFound", { version: sdk.version.replace("Ren'Py ", "Ren'Py ") })}</span>
              ) : (
                <button className="chip chip-warn" onClick={() => openModal("settings")}>
                  {t("welcome.sdkMissing")}
                </button>
              )}
              <span className="chip">v{version}</span>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-3 gap-3">
          <ActionCard
            icon={<Plus size={18} />}
            title={t("welcome.new")}
            hint={t("newProject.hint")}
            accent
            onClick={() => openModal("newProject")}
          />
          <ActionCard
            icon={<FolderOpen size={18} />}
            title={t("welcome.open")}
            hint={t("menu.openProject")}
            onClick={() => void handleOpen()}
          />
          <ActionCard
            icon={<Import size={18} />}
            title={t("welcome.import")}
            hint={t("import.backupNote")}
            onClick={() => openModal("import")}
          />
        </div>

        <div className="mt-10">
          <div className="mb-3 flex items-center gap-2">
            <BookOpen size={14} className="text-[var(--muted-2)]" />
            <span className="text-[11px] font-bold uppercase tracking-[0.08em] text-[var(--muted-2)]">
              {t("welcome.recent")}
            </span>
          </div>
          {recent.length === 0 ? (
            <div className="rounded-xl border border-dashed border-[var(--border)] px-4 py-6 text-[12px] text-[var(--muted-2)]">
              {t("welcome.noRecent")}
            </div>
          ) : (
            <div className="flex flex-col gap-1.5">
              {recent.map((entry) => (
                <button
                  key={entry.path}
                  type="button"
                  onClick={() => void openProject(entry.path)}
                  className="card flex items-center gap-3 px-3.5 py-2.5 text-left hover:bg-[var(--panel-3)]"
                >
                  <FolderOpen size={15} className="text-[var(--accent)]" />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[12.5px] font-medium">{entry.title}</div>
                    <div className="truncate text-[10.5px] text-[var(--muted-2)]">{entry.path}</div>
                  </div>
                  <span className="text-[10px] text-[var(--muted-2)]">{entry.openedAt.slice(0, 10)}</span>
                </button>
              ))}
            </div>
          )}
        </div>

        <p className="mt-10 text-[11.5px] leading-relaxed text-[var(--muted-2)]">{t("welcome.hint")}</p>
      </div>
    </div>
  );
}

function ActionCard({
  icon,
  title,
  hint,
  onClick,
  accent,
}: {
  icon: React.ReactNode;
  title: string;
  hint: string;
  onClick: () => void;
  accent?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`card group flex flex-col items-start gap-2 p-4 text-left transition-all hover:-translate-y-0.5 hover:bg-[var(--panel-3)] ${
        accent ? "border-[var(--accent)]/60" : ""
      }`}
      style={accent ? { boxShadow: "0 12px 34px var(--accent-soft)" } : undefined}
    >
      <span
        className="flex h-9 w-9 items-center justify-center rounded-lg"
        style={{ background: "var(--accent-soft)", color: "var(--accent)" }}
      >
        {icon}
      </span>
      <span className="text-[13px] font-semibold">{title}</span>
      <span className="line-clamp-2 text-[11.5px] leading-snug text-[var(--muted-2)]">{hint}</span>
    </button>
  );
}
