import { useTranslation } from "react-i18next";
import { useMemo } from "react";
import { FolderOpen, Music, Image as ImageIcon, RefreshCw } from "lucide-react";
import { useStudio } from "../store";
import { Button, Empty, IconButton, Input } from "./ui";
import { useState } from "react";
import { api } from "../api";
import { assetUrl, basename } from "../util";

const KINDS = [
  { key: "image", label: "assets.images", icon: ImageIcon },
  { key: "audio", label: "assets.audio", icon: Music },
] as const;

export function AssetsTab() {
  const { t } = useTranslation();
  const assets = useStudio((s) => s.assets);
  const refreshAssets = useStudio((s) => s.refreshAssets);
  const status = useStudio((s) => s.status);
  const [kind, setKind] = useState<"image" | "audio">("image");
  const [query, setQuery] = useState("");

  const filtered = useMemo(
    () =>
      assets
        .filter((asset) => asset.kind === kind)
        .filter((asset) => !query || asset.path.toLowerCase().includes(query.toLowerCase())),
    [assets, kind, query],
  );

  const openFolder = async () => {
    if (!status?.root) return;
    await api.revealInExplorer(`${status.root}/game/${kind === "image" ? "images" : "audio"}`);
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-[52px] flex-none items-center justify-between gap-3 border-b border-[var(--border)] px-5">
        <div className="flex items-center gap-1.5">
          {KINDS.map((entry) => {
            const Icon = entry.icon;
            return (
              <button
                key={entry.key}
                type="button"
                onClick={() => setKind(entry.key)}
                className={`flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[12.5px] ${
                  kind === entry.key
                    ? "bg-[var(--accent-soft)] text-[var(--accent)]"
                    : "text-[var(--muted)] hover:bg-[var(--panel-2)]"
                }`}
              >
                <Icon size={14} /> {t(entry.label)}
                <span className="text-[10px] text-[var(--muted-2)]">
                  {assets.filter((asset) => asset.kind === entry.key).length}
                </span>
              </button>
            );
          })}
        </div>
        <div className="flex items-center gap-2">
          <Input
            className="h-[30px] w-[220px] text-[12px]"
            placeholder={t("assets.search")}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <IconButton title={t("common.reload")} onClick={() => void refreshAssets()}>
            <RefreshCw size={14} />
          </IconButton>
          <Button size="sm" variant="ghost" onClick={() => void openFolder()}>
            <FolderOpen size={14} /> {t("common.openFolder")}
          </Button>
        </div>
      </div>

      <div className="scroll flex-1 px-5 py-4">
        {filtered.length === 0 ? (
          <Empty title={t("assets.empty")} hint={t("assets.dropHint")} />
        ) : kind === "image" ? (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-3">
            {filtered.map((asset) => (
              <div key={asset.path} className="card overflow-hidden">
                <div className="aspect-[16/10] w-full bg-[var(--bg-soft)]">
                  <img
                    src={assetUrl(asset.path)}
                    alt={asset.name}
                    loading="lazy"
                    className="h-full w-full object-cover"
                    draggable={false}
                  />
                </div>
                <div className="px-2 py-1.5">
                  <div className="truncate text-[11.5px]">{asset.name}</div>
                  <div className="truncate text-[10px] text-[var(--muted-2)]">{asset.path}</div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="flex flex-col gap-1.5">
            {filtered.map((asset) => (
              <div key={asset.path} className="card flex items-center gap-3 px-3 py-2">
                <Music size={15} className="text-[var(--muted-2)]" />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[12px]">{basename(asset.path)}</div>
                  <div className="truncate text-[10px] text-[var(--muted-2)]">{asset.path}</div>
                </div>
                <audio controls preload="none" src={assetUrl(asset.path)} className="h-7 max-w-[240px]" />
              </div>
            ))}
          </div>
        )}
        <p className="mt-6 text-[11px] text-[var(--muted-2)]">{t("assets.dropHint")}</p>
      </div>
    </div>
  );
}
