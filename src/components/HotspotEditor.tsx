import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Check,
  MousePointer2,
  Pentagon,
  Plus,
  Square,
  Trash2,
  X,
} from "lucide-react";
import { useStudio } from "../store";
import { Button, Field, IconButton, Input, Modal, Select } from "./ui";
import { useDebouncedValue } from "../hooks";
import { assetUrl, clamp, newId } from "../util";
import type { Hotspot, PointClickParams, StudioNode } from "../types";

type Tool = "select" | "rect" | "polygon";
type Handle = "nw" | "ne" | "sw" | "se";

type Drag =
  | { mode: "create"; start: { x: number; y: number } }
  | { mode: "move"; id: string; start: { x: number; y: number }; origin: Hotspot }
  | { mode: "resize"; id: string; handle: Handle; start: { x: number; y: number }; origin: Hotspot }
  | { mode: "vertex"; id: string; index: number; start: { x: number; y: number }; origin: Hotspot; originPoints: [number, number][] };

function normalize(params: Record<string, unknown>): PointClickParams {
  const raw = Array.isArray(params.hotspots) ? (params.hotspots as Record<string, unknown>[]) : [];
  return {
    bg: String(params.bg ?? ""),
    prompt: (params.prompt as Record<string, string> | undefined) ?? {},
    hoverSound: (params.hoverSound as string | null | undefined) ?? null,
    clickSound: (params.clickSound as string | null | undefined) ?? null,
    hotspots: raw.map((entry) => {
      const points = Array.isArray(entry.points)
        ? (entry.points as unknown[])
            .map((point) =>
              Array.isArray(point) && point.length >= 2
                ? ([Number(point[0]), Number(point[1])] as [number, number])
                : null,
            )
            .filter((point): point is [number, number] => Array.isArray(point))
        : [];
      const shape = entry.shape === "polygon" ? "polygon" : "rect";
      const action = entry.action === "tooltip" || entry.action === "return" ? entry.action : "jump";
      const base: Hotspot = {
        id: String(entry.id ?? newId()),
        label: (entry.label as Record<string, string> | undefined) ?? {},
        shape,
        x: Number(entry.x ?? 0),
        y: Number(entry.y ?? 0),
        w: Number(entry.w ?? 0),
        h: Number(entry.h ?? 0),
        points,
        action,
        target: (entry.target as string | null | undefined) ?? null,
      };
      if (shape === "polygon" && points.length >= 3) {
        const box = polyBounds(points);
        base.x = box.x;
        base.y = box.y;
        base.w = box.w;
        base.h = box.h;
      }
      return base;
    }),
  };
}

function polyBounds(points: [number, number][]) {
  const xs = points.map((point) => point[0]);
  const ys = points.map((point) => point[1]);
  const x = Math.min(...xs);
  const y = Math.min(...ys);
  const w = Math.max(...xs) - x;
  const h = Math.max(...ys) - y;
  return { x, y, w, h };
}

/** Modal wrapper: only mounts the editor while it is open and a block is selected. */
export function HotspotModal() {
  const { t } = useTranslation();
  const open = useStudio((s) => s.modal) === "hotspots";
  const close = useStudio((s) => s.closeModal);
  const project = useStudio((s) => s.project);
  const activeSceneId = useStudio((s) => s.activeSceneId);
  const selectedNodeId = useStudio((s) => s.selectedNodeId);
  const node =
    project?.scenes
      .find((scene) => scene.id === activeSceneId)
      ?.nodes.find((entry) => entry.id === selectedNodeId) ?? null;

  if (!open || !node || node.kind !== "pointclick") return null;

  return (
    <Modal
      open
      onClose={close}
      width={1160}
      title={t("pointclick.title")}
      subtitle={t("pointclick.subtitle")}
      footer={
        <Button variant="primary" size="sm" onClick={close}>
          <Check size={14} /> {t("common.done")}
        </Button>
      }
    >
      <HotspotEditor node={node} />
    </Modal>
  );
}

function HotspotEditor({ node }: { node: StudioNode }) {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const assets = useStudio((s) => s.assets);
  const editLocale = useStudio((s) => s.editLocale);
  const updateNodeParams = useStudio((s) => s.updateNodeParams);

  const [draft, setDraft] = useDebouncedValue<PointClickParams>(
    node.id,
    normalize(node.params),
    (value) => {
      void updateNodeParams(node.id, value as unknown as Record<string, unknown>);
    },
  );
  const draftRef = useRef(draft);
  draftRef.current = draft;
  const updateRef = useRef(setDraft);
  updateRef.current = setDraft;

  const [tool, setTool] = useState<Tool>("select");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [pendingPoints, setPendingPoints] = useState<[number, number][]>([]);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [preview, setPreview] = useState<{ x: number; y: number; w: number; h: number } | null>(null);
  const canvasRef = useRef<HTMLDivElement | null>(null);

  const selected = draft.hotspots.find((hotspot) => hotspot.id === selectedId) ?? null;
  const imageAssets = useMemo(
    () => assets.filter((asset) => asset.kind === "image"),
    [assets],
  );
  const sceneOptions = useMemo(
    () => [
      { value: "", label: t("pointclick.noTarget") },
      ...(project?.scenes ?? []).map((scene) => ({ value: scene.id, label: scene.title })),
    ],
    [project?.scenes, t],
  );

  const toLocal = (clientX: number, clientY: number) => {
    const box = canvasRef.current?.getBoundingClientRect();
    if (!box || box.width === 0 || box.height === 0) return { x: 0, y: 0 };
    return {
      x: clamp((clientX - box.left) / box.width, 0, 1),
      y: clamp((clientY - box.top) / box.height, 0, 1),
    };
  };

  // One set of window listeners for the whole drag gesture: pointerup always
  // ends it, even outside the canvas.
  useEffect(() => {
    if (!drag) return;
    const move = (event: PointerEvent) => {
      const point = toLocal(event.clientX, event.clientY);
      const dx = point.x - drag.start.x;
      const dy = point.y - drag.start.y;
      if (drag.mode === "create") {
        setPreview({
          x: Math.min(drag.start.x, point.x),
          y: Math.min(drag.start.y, point.y),
          w: Math.abs(point.x - drag.start.x),
          h: Math.abs(point.y - drag.start.y),
        });
        return;
      }
      updateRef.current((prev) => ({
        ...prev,
        hotspots: prev.hotspots.map((hotspot) => {
          if (hotspot.id !== drag.id) return hotspot;
          if (drag.mode === "move") {
            const x = clamp(drag.origin.x + dx, 0, 1 - drag.origin.w);
            const y = clamp(drag.origin.y + dy, 0, 1 - drag.origin.h);
            const shiftX = x - drag.origin.x;
            const shiftY = y - drag.origin.y;
            return {
              ...hotspot,
              x,
              y,
              points: hotspot.points.map(
                (point) => [point[0] + shiftX, point[1] + shiftY] as [number, number],
              ),
            };
          }
          if (drag.mode === "vertex") {
            const points = drag.originPoints.map((point, index) =>
              index === drag.index ? ([point[0] + dx, point[1] + dy] as [number, number]) : point,
            );
            return { ...hotspot, points, ...polyBounds(points) };
          }
          const origin = drag.origin;
          let x1 = origin.x;
          let y1 = origin.y;
          let x2 = origin.x + origin.w;
          let y2 = origin.y + origin.h;
          if (drag.handle.includes("w")) x1 = clamp(origin.x + dx, 0, x2 - 0.01);
          if (drag.handle.includes("e")) x2 = clamp(x2 + dx, x1 + 0.01, 1);
          if (drag.handle.includes("n")) y1 = clamp(origin.y + dy, 0, y2 - 0.01);
          if (drag.handle.includes("s")) y2 = clamp(y2 + dy, y1 + 0.01, 1);
          return { ...hotspot, x: x1, y: y1, w: x2 - x1, h: y2 - y1 };
        }),
      }));
    };
    const up = () => {
      if (drag.mode === "create") {
        const box = preview;
        if (box && box.w > 0.012 && box.h > 0.012) {
          const hotspot: Hotspot = {
            id: newId(),
            label: {},
            shape: "rect",
            ...box,
            points: [],
            action: "jump",
            target: null,
          };
          updateRef.current((prev) => ({ ...prev, hotspots: [...prev.hotspots, hotspot] }));
          setSelectedId(hotspot.id);
        }
        setPreview(null);
        setTool("select");
      }
      setDrag(null);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", up);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [drag, preview]);

  // Delete removes the selected zone unless the user is typing in a field.
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.tagName === "SELECT")) {
        return;
      }
      if (event.key !== "Delete" && event.key !== "Backspace") return;
      if (pendingPoints.length > 0) return;
      if (!selectedId) return;
      event.preventDefault();
      updateRef.current((prev) => ({
        ...prev,
        hotspots: prev.hotspots.filter((hotspot) => hotspot.id !== selectedId),
      }));
      setSelectedId(null);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [selectedId, pendingPoints.length]);

  const patchHotspot = (id: string, patch: Partial<Hotspot>) => {
    setDraft((prev) => ({
      ...prev,
      hotspots: prev.hotspots.map((hotspot) =>
        hotspot.id === id ? { ...hotspot, ...patch } : hotspot,
      ),
    }));
  };

  const finishPolygon = () => {
    if (pendingPoints.length < 3) return;
    const box = polyBounds(pendingPoints);
    const hotspot: Hotspot = {
      id: newId(),
      label: {},
      shape: "polygon",
      ...box,
      points: pendingPoints,
      action: "jump",
      target: null,
    };
    setDraft((prev) => ({ ...prev, hotspots: [...prev.hotspots, hotspot] }));
    setSelectedId(hotspot.id);
    setPendingPoints([]);
    setTool("select");
  };

  const startZoneDrag = (
    event: React.PointerEvent,
    id: string,
    mode: "move" | "resize" | "vertex",
    extra?: { handle?: Handle; index?: number },
  ) => {
    event.stopPropagation();
    event.preventDefault();
    const hotspot = draftRef.current.hotspots.find((entry) => entry.id === id);
    if (!hotspot) return;
    const point = toLocal(event.clientX, event.clientY);
    if (mode === "move") {
      setDrag({ mode: "move", id, start: point, origin: { ...hotspot } });
    } else if (mode === "resize") {
      setDrag({ mode: "resize", id, start: point, origin: { ...hotspot }, handle: extra?.handle ?? "se" });
    } else {
      setDrag({
        mode: "vertex",
        id,
        start: point,
        origin: { ...hotspot },
        originPoints: hotspot.points.map((entry) => [entry[0], entry[1]] as [number, number]),
        index: extra?.index ?? 0,
      });
    }
    setSelectedId(id);
    if (tool !== "select") setTool("select");
  };

  return (
    <div className="flex gap-4">
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="mb-2 flex items-center gap-1.5">
          <button
            type="button"
            className={`tab ${tool === "select" ? "active" : ""}`}
            onClick={() => {
              setTool("select");
              setPendingPoints([]);
            }}
          >
            <MousePointer2 size={13} /> {t("pointclick.tool.select")}
          </button>
          <button
            type="button"
            className={`tab ${tool === "rect" ? "active" : ""}`}
            onClick={() => {
              setTool("rect");
              setPendingPoints([]);
            }}
          >
            <Square size={13} /> {t("pointclick.tool.rect")}
          </button>
          <button
            type="button"
            className={`tab ${tool === "polygon" ? "active" : ""}`}
            onClick={() => {
              setTool("polygon");
              setPendingPoints([]);
            }}
          >
            <Pentagon size={13} /> {t("pointclick.tool.polygon")}
          </button>
          <div className="flex-1" />
          {tool === "polygon" ? (
            <>
              <Button size="sm" variant="ghost" onClick={() => setPendingPoints([])}>
                <X size={13} /> {t("common.cancel")}
              </Button>
              <Button size="sm" variant="primary" disabled={pendingPoints.length < 3} onClick={finishPolygon}>
                <Check size={13} /> {t("pointclick.finishPolygon")}
              </Button>
            </>
          ) : null}
        </div>

        <div
          ref={canvasRef}
          className="relative w-full select-none overflow-hidden rounded-xl border border-[var(--border)] bg-[#0a0912]"
          style={{ aspectRatio: "16 / 9", cursor: tool === "select" ? "default" : "crosshair" }}
          onPointerDown={(event) => {
            if (tool === "polygon") {
              const point = toLocal(event.clientX, event.clientY);
              setPendingPoints((prev) => [...prev, [point.x, point.y]]);
              return;
            }
            if (tool !== "rect") {
              setSelectedId(null);
              return;
            }
            const point = toLocal(event.clientX, event.clientY);
            setPreview({ x: point.x, y: point.y, w: 0, h: 0 });
            setDrag({ mode: "create", start: point });
          }}
        >
          {draft.bg ? (
            <img
              src={assetUrl(draft.bg)}
              alt=""
              draggable={false}
              className="pointer-events-none absolute inset-0 h-full w-full object-contain"
            />
          ) : (
            <div className="absolute inset-0 flex items-center justify-center text-[12px] text-[var(--muted-2)]">
              {t("pointclick.noBackground")}
            </div>
          )}

          {/* Zones */}
          {draft.hotspots.map((hotspot, index) => {
            const active = hotspot.id === selectedId;
            const rectStyle = {
              left: `${hotspot.x * 100}%`,
              top: `${hotspot.y * 100}%`,
              width: `${hotspot.w * 100}%`,
              height: `${hotspot.h * 100}%`,
            };
            if (hotspot.shape === "polygon") {
              return (
                <div key={hotspot.id} className="pointer-events-none absolute inset-0">
                  <svg
                    className="pointer-events-none absolute inset-0 h-full w-full"
                    viewBox="0 0 1 1"
                    preserveAspectRatio="none"
                  >
                    <polygon
                      points={hotspot.points.map((point) => `${point[0]},${point[1]}`).join(" ")}
                      fill={active ? "rgba(139,108,242,0.28)" : "rgba(139,108,242,0.14)"}
                      stroke={active ? "#b9a7ff" : "#8b6cf2"}
                      strokeWidth={2}
                      vectorEffect="non-scaling-stroke"
                    />
                  </svg>
                  {/* Bounding box drag area: the polygon itself is drawn above. */}
                  <div
                    className="pointer-events-auto absolute"
                    style={{ ...rectStyle, cursor: "move" }}
                    onPointerDown={(event) => startZoneDrag(event, hotspot.id, "move")}
                  />
                  <span
                    className="pointer-events-none absolute flex h-5 w-5 items-center justify-center rounded-full bg-[var(--accent)] text-[10px] font-semibold text-white"
                    style={{ left: `${hotspot.x * 100}%`, top: `${hotspot.y * 100}%` }}
                  >
                    {index + 1}
                  </span>
                  {active
                    ? hotspot.points.map((point, pointIndex) => (
                        <span
                          key={pointIndex}
                          className="pointer-events-auto absolute h-3 w-3 -translate-x-1/2 -translate-y-1/2 cursor-move rounded-full border border-white bg-[var(--accent)]"
                          style={{ left: `${point[0] * 100}%`, top: `${point[1] * 100}%` }}
                          onPointerDown={(event) =>
                            startZoneDrag(event, hotspot.id, "vertex", { index: pointIndex })
                          }
                        />
                      ))
                    : null}
                </div>
              );
            }
            return (
              <div
                key={hotspot.id}
                className="absolute border-2"
                style={{
                  ...rectStyle,
                  borderColor: active ? "#b9a7ff" : "#8b6cf2",
                  background: active ? "rgba(139,108,242,0.28)" : "rgba(139,108,242,0.14)",
                  cursor: "move",
                }}
                onPointerDown={(event) => startZoneDrag(event, hotspot.id, "move")}
              >
                <span className="pointer-events-none absolute left-1 top-1 flex h-5 w-5 items-center justify-center rounded-full bg-[var(--accent)] text-[10px] font-semibold text-white">
                  {index + 1}
                </span>
                {active
                  ? (["nw", "ne", "sw", "se"] as Handle[]).map((handle) => (
                      <span
                        key={handle}
                        className="absolute h-3 w-3 -translate-x-1/2 -translate-y-1/2 rounded-sm border border-white bg-[var(--accent)]"
                        style={{
                          left: handle.includes("w") ? 0 : "100%",
                          top: handle.includes("n") ? 0 : "100%",
                          cursor: `${handle}-resize`,
                        }}
                        onPointerDown={(event) => startZoneDrag(event, hotspot.id, "resize", { handle })}
                      />
                    ))
                  : null}
              </div>
            );
          })}

          {/* Rect being drawn */}
          {preview ? (
            <div
              className="pointer-events-none absolute border-2 border-dashed border-[var(--accent)] bg-[var(--accent)]/20"
              style={{
                left: `${preview.x * 100}%`,
                top: `${preview.y * 100}%`,
                width: `${preview.w * 100}%`,
                height: `${preview.h * 100}%`,
              }}
            />
          ) : null}

          {/* Polygon being drawn */}
          {pendingPoints.length > 0 ? (
            <svg className="pointer-events-none absolute inset-0 h-full w-full" viewBox="0 0 1 1" preserveAspectRatio="none">
              <polyline
                points={pendingPoints.map((point) => `${point[0]},${point[1]}`).join(" ")}
                fill="rgba(139,108,242,0.18)"
                stroke="#b9a7ff"
                strokeWidth={2}
                vectorEffect="non-scaling-stroke"
                strokeDasharray="6 4"
              />
              {pendingPoints.map((point, index) => (
                <circle key={index} cx={point[0]} cy={point[1]} r={0.006} fill="#b9a7ff" />
              ))}
            </svg>
          ) : null}
        </div>

        <p className="mt-2 text-[11px] leading-relaxed text-[var(--muted-2)]">
          {tool === "polygon" ? t("pointclick.hint.polygon") : t("pointclick.hint.draw")}
        </p>
      </div>

      <div className="flex w-[300px] flex-none flex-col">
        <Field label={t("field.bg")}>
          <Input
            value={draft.bg}
            placeholder="images/bg/hall.png"
            list="pointclick-bg"
            onChange={(event) => setDraft((prev) => ({ ...prev, bg: event.target.value }))}
          />
          <datalist id="pointclick-bg">
            {imageAssets.map((asset) => (
              <option key={asset.path} value={asset.path} />
            ))}
          </datalist>
        </Field>
        <Field label={t("field.prompt")} hint={t("pointclick.promptHint")}>
          <Input
            value={draft.prompt?.[editLocale] ?? ""}
            onChange={(event) =>
              setDraft((prev) => ({
                ...prev,
                prompt: { ...(prev.prompt ?? {}), [editLocale]: event.target.value },
              }))
            }
          />
        </Field>

        <div className="grid grid-cols-2 gap-2">
          <Field label={t("field.hoverSound")}>
            <Input
              className="mono text-[11px]"
              list="pointclick-audio"
              value={draft.hoverSound ?? ""}
              placeholder="audio/hover.ogg"
              onChange={(event) => setDraft((prev) => ({ ...prev, hoverSound: event.target.value || null }))}
            />
          </Field>
          <Field label={t("field.clickSound")}>
            <Input
              className="mono text-[11px]"
              list="pointclick-audio"
              value={draft.clickSound ?? ""}
              placeholder="audio/click.ogg"
              onChange={(event) => setDraft((prev) => ({ ...prev, clickSound: event.target.value || null }))}
            />
          </Field>
        </div>
        <datalist id="pointclick-audio">
          {assets
            .filter((asset) => asset.kind === "audio")
            .map((asset) => (
              <option key={asset.path} value={asset.path} />
            ))}
        </datalist>

        <div className="mt-1 flex items-center justify-between">
          <span className="text-[11px] font-semibold uppercase tracking-[0.07em] text-[var(--muted-2)]">
            {t("pointclick.zones", { count: draft.hotspots.length })}
          </span>
          <IconButton
            title={t("pointclick.addRect")}
            onClick={() => {
              setTool("rect");
              setPendingPoints([]);
            }}
          >
            <Plus size={14} />
          </IconButton>
        </div>

        <div className="scroll mt-2 flex-1 space-y-2 pr-1">
          {draft.hotspots.length === 0 ? (
            <p className="text-[11px] leading-relaxed text-[var(--muted-2)]">{t("pointclick.empty")}</p>
          ) : null}
          {draft.hotspots.map((hotspot, index) => (
            <div
              key={hotspot.id}
              className={`card cursor-pointer p-2 ${
                hotspot.id === selectedId ? "border-[var(--accent)]" : ""
              }`}
              onClick={() => setSelectedId(hotspot.id)}
            >
              <div className="flex items-center gap-1.5">
                <span className="mono text-[10px] text-[var(--muted-2)]">{index + 1}</span>
                <Input
                  className="h-[28px]"
                  value={hotspot.label?.[editLocale] ?? ""}
                  placeholder={t("pointclick.zoneLabel")}
                  onChange={(event) =>
                    patchHotspot(hotspot.id, {
                      label: { ...(hotspot.label ?? {}), [editLocale]: event.target.value },
                    })
                  }
                />
                <IconButton
                  title={t("common.delete")}
                  onClick={(event) => {
                    event.stopPropagation();
                    setDraft((prev) => ({
                      ...prev,
                      hotspots: prev.hotspots.filter((entry) => entry.id !== hotspot.id),
                    }));
                    if (selectedId === hotspot.id) setSelectedId(null);
                  }}
                >
                  <Trash2 size={13} />
                </IconButton>
              </div>
              <div className="mt-1.5 grid grid-cols-2 gap-1.5">
                <Select
                  className="h-[28px] text-[11px]"
                  value={hotspot.action}
                  onChange={(event) =>
                    patchHotspot(hotspot.id, {
                      action: event.target.value as Hotspot["action"],
                    })
                  }
                  options={[
                    { value: "jump", label: t("pointclick.action.jump") },
                    { value: "tooltip", label: t("pointclick.action.tooltip") },
                    { value: "return", label: t("pointclick.action.return") },
                  ]}
                />
                {hotspot.action === "jump" ? (
                  <Select
                    className="h-[28px] text-[11px]"
                    value={hotspot.target ?? ""}
                    onChange={(event) =>
                      patchHotspot(hotspot.id, { target: event.target.value || null })
                    }
                    options={sceneOptions}
                  />
                ) : (
                  <span className="text-[10px] text-[var(--muted-2)]">
                    {hotspot.shape === "polygon" ? t("pointclick.shape.polygon") : t("pointclick.shape.rect")}
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
