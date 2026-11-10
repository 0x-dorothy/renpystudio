import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Code2, FileText, FolderOpen, Package, PowerOff, RefreshCw, Search } from "lucide-react";
import { useStudio } from "../store";
import { api } from "../api";
import { Button, Empty, IconButton, Input } from "./ui";

function sizeLabel(bytes: number): string {
  if (bytes > 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} МБ`;
  if (bytes > 1024) return `${Math.round(bytes / 1024)} КБ`;
  return `${bytes} Б`;
}

export function PluginsTab() {
  const { t } = useTranslation();
  const plugins = useStudio((s) => s.plugins);
  const root = useStudio((s) => s.pluginsRoot);
  const refresh = useStudio((s) => s.refreshPlugins);
  const enable = useStudio((s) => s.enablePlugin);
  const disable = useStudio((s) => s.disablePlugin);
  const showDocs = useStudio((s) => s.showPluginDocs);
  const insertSnippet = useStudio((s) => s.insertPluginSnippet);
  const busy = useStudio((s) => s.busy);
  const [query, setQuery] = useState("");

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return plugins;
    return plugins.filter(
      (plugin) =>
        plugin.name.toLowerCase().includes(q) ||
        plugin.id.includes(q) ||
        plugin.description.toLowerCase().includes(q),
    );
  }, [plugins, query]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-[52px] flex-none items-center justify-between gap-3 border-b border-[var(--border)] px-5">
        <div className="flex items-center gap-2">
          <h2 className="text-[14px] font-semibold">{t("plugins.title")}</h2>
          <span className="chip">{plugins.length}</span>
          <span className="chip chip-ok">{plugins.filter((p) => p.enabled).length}</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="relative">
            <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--muted-2)]" />
            <Input
              className="h-[30px] w-[220px] pl-7 text-[12px]"
              placeholder={t("common.search")}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </div>
          <IconButton title={t("plugins.refresh")} onClick={() => void refresh()}>
            <RefreshCw size={14} />
          </IconButton>
          <Button size="sm" variant="ghost" onClick={() => root && void api.revealInExplorer(root)}>
            <FolderOpen size={14} /> {t("plugins.openFolder")}
          </Button>
        </div>
      </div>

      <div className="scroll flex-1 px-5 py-4">
        <p className="mb-3 text-[12px] text-[var(--muted)]">{t("plugins.hint")}</p>
        {filtered.length === 0 ? (
          <Empty
            icon={<Package size={22} />}
            title={t("plugins.empty")}
            hint={root ? `${t("plugins.root")}: ${root}` : undefined}
          />
        ) : (
          <div className="grid grid-cols-2 gap-3">
            {filtered.map((plugin) => (
              <div key={plugin.id} className={`card flex flex-col gap-2 p-3 ${plugin.enabled ? "border-[var(--ok)]/50" : ""}`}>
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <div className="flex items-center gap-1.5">
                      <span className="truncate text-[13px] font-semibold">{plugin.name}</span>
                      {plugin.enabled ? <span className="chip chip-ok">{t("plugins.enabledBadge")}</span> : null}
                    </div>
                    <div className="mono mt-0.5 text-[10px] text-[var(--muted-2)]">#{plugin.id}</div>
                  </div>
                  <span className="chip">{t(`plugins.types.${plugin.kind}`)}</span>
                </div>
                <p className="line-clamp-3 min-h-[42px] text-[11.5px] leading-snug text-[var(--muted)]">
                  {plugin.description || "—"}
                </p>
                <div className="flex items-center gap-2 text-[10.5px] text-[var(--muted-2)]">
                  <span>{t("plugins.files", { count: plugin.files.length })}</span>
                  <span>·</span>
                  <span>{sizeLabel(plugin.size)}</span>
                </div>
                <div className="mt-auto flex flex-wrap items-center gap-1.5">
                  {plugin.enabled ? (
                    <Button size="sm" variant="danger" disabled={busy} onClick={() => void disable(plugin.id)}>
                      <PowerOff size={13} /> {t("plugins.disable")}
                    </Button>
                  ) : (
                    <Button size="sm" variant="primary" disabled={busy} onClick={() => void enable(plugin.id)}>
                      <Package size={13} /> {t("plugins.enable")}
                    </Button>
                  )}
                  <Button size="sm" variant="ghost" onClick={() => void showDocs(plugin.id)}>
                    <FileText size={13} /> {t("plugins.docs")}
                  </Button>
                  {plugin.snippet ? (
                    <Button size="sm" variant="ghost" onClick={() => void insertSnippet(plugin.snippet!)}>
                      <Code2 size={13} /> {t("plugins.snippet")}
                    </Button>
                  ) : null}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
