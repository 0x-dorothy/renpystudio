import { useTranslation } from "react-i18next";
import { AlertTriangle, Check, Moon, Sun, Wifi } from "lucide-react";
import { useStudio } from "../store";
import { API_VERSION } from "../version";
import { formatTime } from "../util";

export function StatusBar() {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const status = useStudio((s) => s.status);
  const issues = useStudio((s) => s.issues);
  const settings = useStudio((s) => s.settings);
  const sdk = useStudio((s) => s.sdk);
  const mcp = useStudio((s) => s.mcp);
  const saveSettings = useStudio((s) => s.saveSettings);
  const openModal = useStudio((s) => s.openModal);
  const version = useStudio((s) => s.version);

  const errors = issues?.errors ?? 0;
  const warnings = issues?.warnings ?? 0;

  return (
    <footer className="flex h-[26px] flex-none items-center justify-between gap-4 border-t border-[var(--border)] bg-[var(--panel)] px-3 text-[11px] text-[var(--muted-2)]">
      <div className="flex min-w-0 items-center gap-3">
        {project ? (
          <>
            <span className="flex items-center gap-1.5">
              <span
                className={`h-1.5 w-1.5 rounded-full ${
                  status?.dirty ? "bg-[var(--warn)]" : "bg-[var(--ok)]"
                }`}
              />
              {status?.lastSaved
                ? t("status.saved", { time: formatTime(status.lastSaved) })
                : t("status.notSaved")}
            </span>
            <span className="text-[var(--muted-2)]">·</span>
            <span>{t("status.revision", { revision: status?.revision ?? 0 })}</span>
            <button
              type="button"
              onClick={() => openModal("lint")}
              className={`flex items-center gap-1 rounded px-1 hover:bg-[var(--panel-2)] ${
                errors > 0 ? "text-[var(--danger)]" : warnings > 0 ? "text-[var(--warn)]" : "text-[var(--ok)]"
              }`}
            >
              {errors > 0 || warnings > 0 ? <AlertTriangle size={11} /> : <Check size={11} />}
              {errors > 0
                ? t("status.errors", { count: errors })
                : warnings > 0
                  ? t("status.warnings", { count: warnings })
                  : t("status.clean")}
            </button>
            <span className="hidden truncate text-[var(--muted-2)] xl:inline">{status?.root}</span>
          </>
        ) : (
          <span>{t("app.name")} {version || API_VERSION}</span>
        )}
      </div>

      <div className="flex flex-none items-center gap-2">
        {sdk ? (
          <span className="flex items-center gap-1" title={sdk.root}>
            <Check size={10} className="text-[var(--ok)]" /> {t("status.sdk", { version: sdk.version.replace("Ren'Py ", "") })}
          </span>
        ) : (
          <button
            type="button"
            onClick={() => openModal("settings")}
            className="flex items-center gap-1 text-[var(--warn)] hover:underline"
          >
            <AlertTriangle size={10} /> {t("status.sdkMissing")}
          </button>
        )}
        {mcp.running ? (
          <button
            type="button"
            onClick={() => openModal("settings")}
            className="flex items-center gap-1 text-[var(--accent)] hover:underline"
          >
            <Wifi size={10} /> {t("status.mcp", { port: mcp.port ?? "" })}
          </button>
        ) : null}
        <div className="mx-1 h-3 w-px bg-[var(--border)]" />
        <div className="flex items-center overflow-hidden rounded border border-[var(--border)]">
          {["ru", "en"].map((locale) => (
            <button
              key={locale}
              type="button"
              onClick={() => void saveSettings({ ...settings, locale })}
              className={`px-1.5 py-[1px] text-[10px] font-semibold uppercase ${
                settings.locale === locale
                  ? "bg-[var(--accent-soft)] text-[var(--accent)]"
                  : "text-[var(--muted-2)] hover:text-[var(--text)]"
              }`}
            >
              {locale}
            </button>
          ))}
        </div>
        <button
          type="button"
          title={settings.theme === "dark" ? t("settings.theme.light") : t("settings.theme.dark")}
          onClick={() =>
            void saveSettings({ ...settings, theme: settings.theme === "dark" ? "light" : "dark" })
          }
          className="flex h-4 w-4 items-center justify-center text-[var(--muted)] hover:text-[var(--text)]"
        >
          {settings.theme === "dark" ? <Moon size={11} /> : <Sun size={11} />}
        </button>
      </div>
    </footer>
  );
}
