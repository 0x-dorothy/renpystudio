import { memo, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Copy,
  GripVertical,
  Plus,
  Search,
  Trash2,
  X,
} from "lucide-react";
import { useStudio } from "../store";
import { Button, Empty, IconButton, Input, Modal } from "./ui";
import { BLOCKS, CATEGORY_ORDER, blockDef, blockSummary } from "../blocks/registry";
import type { BlockCategory, BlockDef } from "../blocks/registry";
import type { StudioNode } from "../types";

const CATEGORY_COLORS: Record<BlockCategory, string> = {
  text: "#7aa2f7",
  visual: "#e96aa5",
  flow: "#8b6cf2",
  sound: "#43d9a3",
  data: "#f0b429",
  screens: "#2fc7c0",
  code: "#f2596a",
};

export function SceneEditor() {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const activeSceneId = useStudio((s) => s.activeSceneId);
  const selectedNodeId = useStudio((s) => s.selectedNodeId);
  const selectNode = useStudio((s) => s.selectNode);
  const openPalette = useStudio((s) => s.openPalette);
  const paletteIndex = useStudio((s) => s.paletteIndex);
  const moveNode = useStudio((s) => s.moveNode);
  const duplicateNode = useStudio((s) => s.duplicateNode);
  const deleteNode = useStudio((s) => s.deleteNode);
  const setTab = useStudio((s) => s.setTab);
  const [dropIndex, setDropIndex] = useState<number | null>(null);
  const [menu, setMenu] = useState<{ nodeId: string; index: number; x: number; y: number } | null>(null);

  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    window.addEventListener("mousedown", close);
    window.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("scroll", close, true);
    };
  }, [menu]);

  const scene = project?.scenes.find((s) => s.id === activeSceneId) ?? null;
  const paletteOpen = paletteIndex !== null;

  if (!scene) {
    return <div className="flex flex-1 items-center justify-center text-[12px] text-[var(--muted)]">—</div>;
  }

  const handleDrop = (index: number, event: React.DragEvent) => {
    event.preventDefault();
    const nodeId = event.dataTransfer.getData("application/studio-node");
    setDropIndex(null);
    if (!nodeId) return;
    const from = scene.nodes.findIndex((n) => n.id === nodeId);
    if (from === -1) return;
    let target = index;
    if (from < target) target = Math.max(0, target - 1);
    if (target === from) return;
    void moveNode(nodeId, target);
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-[52px] flex-none items-center justify-between border-b border-[var(--border)] px-5">
        <div className="flex min-w-0 items-center gap-2.5">
          <h2 className="truncate text-[14px] font-semibold">{scene.title}</h2>
          <span className="chip">{t("scene.count", { count: scene.nodes.length })}</span>
          {project?.meta.startScene === scene.id ? (
            <span className="chip chip-accent">{t("scene.startBadge")}</span>
          ) : null}
        </div>
        <Button size="sm" variant="ghost" onClick={() => openPalette(scene.nodes.length)}>
          <Plus size={14} /> {t("action.addBlock")}
        </Button>
      </div>

      <div className="scroll flex-1 px-8 py-5">
        <div className="mx-auto flex max-w-[820px] flex-col">
          <InsertZone index={0} active={dropIndex === 0} onHover={setDropIndex} onDrop={handleDrop} onAdd={() => openPalette(0)} />
          {scene.nodes.map((node, index) => (
            <div key={node.id}>
              <BlockCard
                node={node}
                index={index}
                selected={node.id === selectedNodeId}
                onSelect={() => selectNode(node.id)}
                onDragLeaveZone={() => setDropIndex(null)}
                onContextMenu={(event) => {
                  event.preventDefault();
                  selectNode(node.id);
                  setMenu({ nodeId: node.id, index, x: event.clientX, y: event.clientY });
                }}
              />
              <InsertZone
                index={index + 1}
                active={dropIndex === index + 1}
                onHover={setDropIndex}
                onDrop={handleDrop}
                onAdd={() => openPalette(index + 1)}
              />
            </div>
          ))}
          {scene.nodes.length === 0 ? (
            <div className="mt-10">
              <Empty
                title={t("scene.empty")}
                hint={t("scene.emptyHint")}
                action={
                  <Button variant="primary" size="sm" onClick={() => openPalette(0)}>
                    <Plus size={14} /> {t("scene.addFirst")}
                  </Button>
                }
              />
            </div>
          ) : null}
        </div>
      </div>

      <BlockPalette open={paletteOpen} onClose={() => useStudio.getState().closePalette()} />

      {menu ? (
        <div
          className="fixed z-[65] w-[196px] overflow-hidden rounded-lg border border-[var(--border)] bg-[var(--panel)] py-1 shadow-2xl"
          style={{ left: menu.x, top: menu.y }}
          onMouseDown={(event) => event.stopPropagation()}
        >
          {[
            {
              label: t("action.duplicate"),
              run: () => void duplicateNode(menu.nodeId),
            },
            {
              label: t("action.moveUp"),
              run: () => void moveNode(menu.nodeId, Math.max(0, menu.index - 1)),
            },
            {
              label: t("action.moveDown"),
              run: () => void moveNode(menu.nodeId, Math.min(scene.nodes.length - 1, menu.index + 1)),
            },
            {
              label: t("action.showOnGraph"),
              run: () => {
                selectNode(menu.nodeId);
                setTab("graph");
              },
            },
            {
              label: t("common.delete"),
              danger: true,
              run: () => void deleteNode(menu.nodeId),
            },
          ].map((item) => (
            <button
              key={item.label}
              type="button"
              onClick={() => {
                item.run();
                setMenu(null);
              }}
              className={`flex w-full items-center px-3 py-1.5 text-left text-[12px] hover:bg-[var(--panel-2)] ${
                item.danger ? "text-[var(--danger)]" : "text-[var(--text)]"
              }`}
            >
              {item.label}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function InsertZone({
  index,
  active,
  onHover,
  onDrop,
  onAdd,
}: {
  index: number;
  active: boolean;
  onHover: (index: number | null) => void;
  onDrop: (index: number, event: React.DragEvent) => void;
  onAdd: () => void;
}) {
  return (
    <div
      className="group relative h-[18px] flex-none"
      onDragOver={(event) => {
        event.preventDefault();
        onHover(index);
      }}
      onDragLeave={() => onHover(null)}
      onDrop={(event) => onDrop(index, event)}
    >
      <div
        className={`absolute left-0 right-0 top-1/2 h-[2px] -translate-y-1/2 rounded-full transition-colors ${
          active ? "bg-[var(--accent)]" : "bg-transparent group-hover:bg-[var(--border)]"
        }`}
      />
      <button
        type="button"
        onClick={onAdd}
        className="absolute left-1/2 top-1/2 flex h-[18px] w-[18px] -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border border-[var(--border)] bg-[var(--panel-2)] text-[var(--muted)] opacity-0 transition-opacity group-hover:opacity-100"
        title="+"
      >
        <Plus size={11} />
      </button>
    </div>
  );
}

function BlockCard({
  node,
  index,
  selected,
  onSelect,
  onDragLeaveZone,
  onContextMenu,
}: {
  node: StudioNode;
  index: number;
  selected: boolean;
  onSelect: () => void;
  onDragLeaveZone: () => void;
  onContextMenu?: (event: React.MouseEvent) => void;
}) {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const remove = useStudio((s) => s.deleteNode);
  const duplicate = useStudio((s) => s.duplicateNode);
  const selectedNodeId = useStudio((s) => s.selectedNodeId);
  const def = blockDef(node.kind);
  const Icon = def?.icon;
  const summary = blockSummary(node.kind, node.params, project, t);
  const color = def ? CATEGORY_COLORS[def.category] : "#8b6cf2";

  return (
    <div
      draggable
      onDragStart={(event) => {
        event.dataTransfer.setData("application/studio-node", node.id);
        event.dataTransfer.effectAllowed = "move";
      }}
      onDragEnd={onDragLeaveZone}
      onClick={onSelect}
      onContextMenu={onContextMenu}
      className={`card group relative flex cursor-pointer items-start gap-3 px-3 py-2.5 ${
        selected ? "selected" : ""
      }`}
      style={{
        borderLeft: `3px solid ${color}55`,
        contentVisibility: "auto",
        containIntrinsicSize: "68px",
      }}
    >
      <div className="mt-0.5 flex flex-none flex-col items-center gap-1">
        <GripVertical size={13} className="text-[var(--muted-2)] opacity-0 transition-opacity group-hover:opacity-100" />
        <div
          className="flex h-7 w-7 items-center justify-center rounded-md"
          style={{ background: `${color}1f`, color }}
        >
          {Icon ? <Icon size={15} /> : null}
        </div>
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <span className="text-[12px] font-semibold text-[var(--text)]">
            {def ? t(def.labelKey) : node.kind}
          </span>
          {node.source === "mcp" ? <span className="chip chip-accent">MCP</span> : null}
          {node.locked ? <span className="chip chip-warn">{t("scene.codeChip")}</span> : null}
        </div>
        <p className="mt-0.5 line-clamp-2 text-[12px] leading-snug text-[var(--muted)]">
          {summary || <span className="text-[var(--muted-2)]">{t("summary.empty")}</span>}
        </p>
      </div>
      <div className="flex flex-none items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
        <IconButton
          title={t("action.duplicate")}
          onClick={(event) => {
            event.stopPropagation();
            void duplicate(node.id);
          }}
        >
          <Copy size={13} />
        </IconButton>
        <IconButton
          title={t("common.delete")}
          onClick={(event) => {
            event.stopPropagation();
            void remove(node.id);
          }}
        >
          <Trash2 size={13} className="text-[var(--danger)]" />
        </IconButton>
      </div>
      <span className="mono absolute right-2 top-1 text-[9px] text-[var(--muted-2)] opacity-0 group-hover:opacity-60">
        {index + 1}
      </span>
    </div>
  );
}

function BlockPalette({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const addNode = useStudio((s) => s.addNode);
  const paletteIndex = useStudio((s) => s.paletteIndex);

  const grouped = useMemo(() => {
    const q = query.trim().toLowerCase();
    const byCategory = new Map<BlockCategory, BlockDef[]>();
    for (const def of BLOCKS) {
      const label = t(def.labelKey).toLowerCase();
      if (q && !label.includes(q) && !def.kind.includes(q)) continue;
      const list = byCategory.get(def.category) ?? [];
      list.push(def);
      byCategory.set(def.category, list);
    }
    return byCategory;
  }, [query, t]);

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t("palette.title")}
      width={860}
      subtitle={t("scene.emptyHint")}
    >
      <div className="mb-4 flex items-center gap-2">
        <div className="relative flex-1">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--muted-2)]" />
          <Input
            autoFocus
            className="pl-8"
            placeholder={t("palette.search")}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </div>
        <IconButton onClick={onClose}>
          <X size={16} />
        </IconButton>
      </div>

      {grouped.size === 0 ? (
        <div className="py-10 text-center text-[12px] text-[var(--muted)]">{t("palette.empty")}</div>
      ) : null}

      <div className="flex flex-col gap-5">
        {CATEGORY_ORDER.filter((category) => grouped.has(category)).map((category) => (
          <div key={category}>
            <div className="mb-2 text-[10px] font-bold uppercase tracking-[0.08em] text-[var(--muted-2)]">
              {t(`palette.cat.${category}`)}
            </div>
            <div className="grid grid-cols-3 gap-2">
              {(grouped.get(category) ?? []).map((def) => {
                const Icon = def.icon;
                return (
                  <button
                    key={def.kind}
                    type="button"
                    onClick={() => {
                      void addNode(def.kind, paletteIndex ?? undefined);
                      onClose();
                    }}
                    className="card flex items-start gap-2.5 px-3 py-2.5 text-left hover:bg-[var(--panel-3)]"
                  >
                    <div
                      className="mt-0.5 flex h-8 w-8 flex-none items-center justify-center rounded-lg"
                      style={{
                        background: `${CATEGORY_COLORS[def.category]}1f`,
                        color: CATEGORY_COLORS[def.category],
                      }}
                    >
                      <Icon size={16} />
                    </div>
                    <div className="min-w-0">
                      <div className="text-[12.5px] font-medium">{t(def.labelKey)}</div>
                      <div className="mt-0.5 line-clamp-2 text-[11px] leading-snug text-[var(--muted-2)]">
                        {t(`palette.cat.${def.category}`)}
                      </div>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </div>
    </Modal>
  );
}
