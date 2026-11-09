import { useTranslation } from "react-i18next";
import { AlertTriangle, Check, Info, X } from "lucide-react";
import { useStudio } from "../store";
import { Button } from "./ui";

export function Toasts() {
  const toasts = useStudio((s) => s.toasts);
  const dismiss = useStudio((s) => s.dismissToast);

  return (
    <div className="pointer-events-none fixed bottom-9 right-4 z-[60] flex w-[360px] flex-col gap-2">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`animate-in pointer-events-auto flex items-start gap-2 rounded-xl border px-3 py-2.5 text-[12px] shadow-lg backdrop-blur ${
            toast.kind === "error"
              ? "border-[var(--danger)]/50 bg-[var(--panel)]"
              : toast.kind === "success"
                ? "border-[var(--ok)]/40 bg-[var(--panel)]"
                : "border-[var(--border)] bg-[var(--panel)]"
          }`}
          style={{ boxShadow: "var(--shadow)" }}
        >
          <span
            className={`mt-0.5 ${
              toast.kind === "error"
                ? "text-[var(--danger)]"
                : toast.kind === "success"
                  ? "text-[var(--ok)]"
                  : "text-[var(--accent)]"
            }`}
          >
            {toast.kind === "error" ? <AlertTriangle size={14} /> : toast.kind === "success" ? <Check size={14} /> : <Info size={14} />}
          </span>
          <span className="flex-1 leading-snug">{toast.message}</span>
          <button type="button" onClick={() => dismiss(toast.id)} className="text-[var(--muted-2)] hover:text-[var(--text)]">
            <X size={13} />
          </button>
        </div>
      ))}
    </div>
  );
}

export function ExternalChangeBanner() {
  const { t } = useTranslation();
  const change = useStudio((s) => s.externalChange);
  const dismiss = useStudio((s) => s.dismissExternal);
  const reload = useStudio((s) => s.reloadProject);

  if (!change) return null;
  const rpyCount = change.rpy.length;
  const assetCount = change.assets.length;

  return (
    <div className="flex flex-none items-center gap-3 border-b border-[var(--warn)]/40 bg-[var(--warn)]/10 px-4 py-2 text-[12px]">
      <AlertTriangle size={14} className="text-[var(--warn)]" />
      <div className="flex-1">
        <span className="font-medium text-[var(--warn)]">{t("conflict.externalTitle")}</span>{" "}
        <span className="text-[var(--muted)]">
          {t("conflict.externalText")}
          {rpyCount > 0 ? ` · ${rpyCount} .rpy` : ""}
          {assetCount > 0 ? ` · ${t("conflict.assetsCount", { count: assetCount })}` : ""}
        </span>
      </div>
      <Button size="sm" variant="ghost" onClick={dismiss}>
        {t("conflict.ignore")}
      </Button>
      <Button
        size="sm"
        variant="primary"
        onClick={() => {
          void reload();
          dismiss();
        }}
      >
        {t("conflict.reload")}
      </Button>
    </div>
  );
}
