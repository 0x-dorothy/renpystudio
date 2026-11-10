import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Background,
  BackgroundVariant,
  Controls,
  Handle,
  MarkerType,
  MiniMap,
  Position,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type Connection,
  type Edge,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import dagre from "@dagrejs/dagre";
import { AlertTriangle, Flag, Play, Search, Split } from "lucide-react";
import { useStudio } from "../store";
import { Input, Segmented } from "./ui";
import { blockDef, blockSummary } from "../blocks/registry";
import type { GraphData } from "../types";

interface SceneNodeData extends Record<string, unknown> {
  title: string;
  blocks: number;
  isStart: boolean;
  deadEnd: boolean;
  unreachable: boolean;
  color: string | null;
}

interface BlockNodeData extends Record<string, unknown> {
  kind: string;
  label: string;
  summary: string;
  tone: string;
}

type SceneNodeType = Node<SceneNodeData, "scene">;
type FlowNodeType = Node<{ label: string; tone: string }, "flow">;
type BlockNodeType = Node<BlockNodeData, "block">;

const TONES: Record<string, string> = {
  text: "#7aa2f7",
  visual: "#e96aa5",
  flow: "#8b6cf2",
  sound: "#43d9a3",
  data: "#f0b429",
  screens: "#2fc7c0",
  code: "#f2596a",
};

function toneFor(kind: string): string {
  const def = blockDef(kind);
  return def ? TONES[def.category] ?? "#8b6cf2" : "#8b6cf2";
}

function SceneNode({ data, selected }: NodeProps<SceneNodeType>) {
  const { t } = useTranslation();
  return (
    <div
      className={`w-[200px] rounded-xl border bg-[var(--panel-2)] px-3 py-2 transition-shadow ${
        selected ? "border-[var(--accent)] shadow-[0_0_0_3px_var(--accent-soft)]" : "border-[var(--border)]"
      } ${data.unreachable ? "opacity-75" : ""}`}
      style={data.color ? { borderLeft: `3px solid ${data.color}` } : undefined}
    >
      <Handle type="target" position={Position.Left} />
      <div className="flex items-center gap-1.5">
        {data.isStart ? <Play size={11} className="text-[var(--accent)]" /> : null}
        <span className="truncate text-[12.5px] font-semibold text-[var(--text)]">{data.title}</span>
      </div>
      <div className="mt-1 flex items-center gap-1.5">
        <span className="chip">{data.blocks}</span>
        {data.deadEnd ? (
          <span className="chip chip-warn">
            <AlertTriangle size={9} /> {t("graph.deadEndChip")}
          </span>
        ) : null}
        {data.unreachable ? <span className="chip">{t("graph.unreachableChip")}</span> : null}
        {data.isStart ? <span className="chip chip-accent">{t("graph.start")}</span> : null}
      </div>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

function FlowNode({ data, selected }: NodeProps<FlowNodeType>) {
  return (
    <div
      className={`flex w-[170px] items-center gap-1.5 rounded-lg border px-2.5 py-1.5 ${
        selected ? "border-[var(--accent)]" : "border-[var(--border)]"
      }`}
      style={{ background: "var(--panel-3)", color: "var(--text)" }}
    >
      <Handle type="target" position={Position.Left} />
      <span className="h-1.5 w-1.5 flex-none rounded-full" style={{ background: data.tone }} />
      <span className="truncate text-[11px] font-medium">{data.label}</span>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

function BlockNode({ data, selected }: NodeProps<BlockNodeType>) {
  const def = blockDef(data.kind);
  const Icon = def?.icon;
  return (
    <div
      className={`flex w-[240px] items-center gap-2 rounded-lg border bg-[var(--panel-2)] px-2 py-1.5 ${
        selected ? "border-[var(--accent)] shadow-[0_0_0_2px_var(--accent-soft)]" : "border-[var(--border)]"
      }`}
      style={{ borderLeft: `3px solid ${data.tone}` }}
    >
      <Handle type="target" position={Position.Top} />
      <span className="flex h-6 w-6 flex-none items-center justify-center rounded" style={{ background: `${data.tone}22`, color: data.tone }}>
        {Icon ? <Icon size={13} /> : null}
      </span>
      <div className="min-w-0">
        <div className="truncate text-[11.5px] font-medium leading-tight">{data.label}</div>
        {data.summary ? (
          <div className="truncate text-[10px] leading-tight text-[var(--muted-2)]">{data.summary}</div>
        ) : null}
      </div>
      <Handle type="source" position={Position.Bottom} />
    </div>
  );
}

const nodeTypes = { scene: SceneNode, flow: FlowNode, block: BlockNode };

function GraphInner() {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const graph = useStudio((s) => s.graph);
  const settings = useStudio((s) => s.settings);
  const saveSettings = useStudio((s) => s.saveSettings);
  const refreshGraph = useStudio((s) => s.refreshGraph);
  const selectScene = useStudio((s) => s.selectScene);
  const setTab = useStudio((s) => s.setTab);
  const selectNode = useStudio((s) => s.selectNode);
  const activeSceneId = useStudio((s) => s.activeSceneId);
  const retargetEdge = useStudio((s) => s.retargetEdge);
  const { fitView } = useReactFlow();
  const first = useRef(true);
  const [query, setQuery] = useState("");

  useEffect(() => {
    void refreshGraph();
  }, [refreshGraph]);

  const detail = settings.graphDetail;
  const totalBlocks = project?.scenes.reduce((sum, scene) => sum + scene.nodes.length, 0) ?? 0;

  const { nodes, edges } = useMemo(() => {
    if (!project || !graph) return { nodes: [] as Node[], edges: [] as Edge[] };
    const data: GraphData = graph;
    const colorOf = (kind: string) =>
      kind === "choice" ? "#8b6cf2" : kind === "condition" ? "#f0b429" : "#6d6a85";

    // ---- Block-level map: scenes in columns, blocks chained vertically -----
    if (detail === "all") {
      const colWidth = 300;
      const rowHeight = 56;
      const headerHeight = 92;
      const flowNodes: Node[] = [];
      const flowEdges: Edge[] = [];
      const firstBlock = new Map<string, string>();
      const labelNode = new Map<string, string>();

      project.scenes.forEach((scene, sceneIndex) => {
        flowNodes.push({
          id: `scene:${scene.id}`,
          type: "scene",
          position: { x: sceneIndex * colWidth, y: 0 },
          draggable: false,
          data: {
            title: scene.title,
            blocks: scene.nodes.length,
            isStart: project.meta.startScene === scene.id,
            deadEnd: data.deadEnds.includes(scene.id),
            unreachable: data.unreachable.includes(scene.id),
            color: scene.color,
          },
        });
        scene.nodes.forEach((node, blockIndex) => {
          const def = blockDef(node.kind);
          if (node.kind === "label") {
            const name = String((node.params as { name?: string }).name ?? "");
            labelNode.set(name, `b:${node.id}`);
          }
          flowNodes.push({
            id: `b:${node.id}`,
            type: "block",
            position: { x: sceneIndex * colWidth + 8, y: headerHeight + blockIndex * rowHeight },
            draggable: false,
            data: {
              kind: node.kind,
              label: def ? t(def.labelKey) : node.kind,
              summary: blockSummary(node.kind, node.params, project, t).slice(0, 42),
              tone: toneFor(node.kind),
            },
          });
          if (blockIndex === 0) firstBlock.set(scene.id, `b:${node.id}`);
          if (blockIndex > 0 && totalBlocks <= 600) {
            const prev = scene.nodes[blockIndex - 1];
            flowEdges.push({
              id: `seq:${prev.id}`,
              source: `b:${prev.id}`,
              target: `b:${node.id}`,
              type: "smoothstep",
              style: { stroke: "#2c2c40", strokeWidth: 1 },
              reconnectable: false,
            });
          }
        });
        if (scene.nodes.length > 0) {
          flowEdges.push({
            id: `enter:${scene.id}`,
            source: `scene:${scene.id}`,
            target: `b:${scene.nodes[0].id}`,
            type: "smoothstep",
            style: { stroke: "#3a3a52", strokeWidth: 1 },
            reconnectable: false,
          });
        }
      });

      for (const edge of data.edges) {
        const targetScene = edge.toScene
          ? project.scenes.find((scene) => scene.id === edge.toScene)
          : undefined;
        const target = targetScene
          ? firstBlock.get(targetScene.id)
          : edge.toScene
            ? labelNode.get(edge.toScene)
            : undefined;
        if (!target) continue;
        flowEdges.push({
          id: `flow:${edge.id}`,
          source: `b:${edge.nodeId}`,
          target,
          label: edge.label.slice(0, 34),
          type: "smoothstep",
          animated: edge.kind === "condition",
          style: { stroke: colorOf(edge.kind) },
          markerEnd: { type: MarkerType.ArrowClosed, color: colorOf(edge.kind) },
          labelStyle: { fill: "var(--muted)", fontSize: 9 },
          labelBgStyle: { fill: "var(--panel)", fillOpacity: 0.85 },
          reconnectable: false,
        });
      }
      return { nodes: flowNodes, edges: flowEdges };
    }

    // ---- Scene / choice map (dagre) ---------------------------------------
    const g = new dagre.graphlib.Graph();
    g.setDefaultEdgeLabel(() => ({}));
    g.setGraph({ rankdir: "LR", nodesep: 26, ranksep: 74, marginx: 24, marginy: 24 });

    const sceneNodes: SceneNodeType[] = project.scenes.map((scene) => ({
      id: `scene:${scene.id}`,
      type: "scene" as const,
      position: { x: 0, y: 0 },
      data: {
        title: scene.title,
        blocks: scene.nodes.length,
        isStart: project.meta.startScene === scene.id,
        deadEnd: data.deadEnds.includes(scene.id),
        unreachable: data.unreachable.includes(scene.id),
        color: scene.color,
      },
      selected: scene.id === activeSceneId,
    }));
    for (const node of sceneNodes) {
      g.setNode(node.id, { width: 200, height: 72 });
    }

    const flowNodes: Node[] = [...sceneNodes];
    const flowEdges: Edge[] = [];

    if (detail === "scenes") {
      const merged = new Map<string, { from: string; to: string; labels: string[]; kinds: Set<string>; nodeId: string; optionId?: string }>();
      for (const edge of data.edges) {
        if (!edge.toScene) continue;
        const key = `${edge.fromScene}->${edge.toScene}`;
        const entry = merged.get(key) ?? {
          from: edge.fromScene,
          to: edge.toScene,
          labels: [],
          kinds: new Set<string>(),
          nodeId: edge.nodeId,
          optionId: edge.kind === "choice" ? edge.id.split(":")[1] : undefined,
        };
        if (edge.label) entry.labels.push(edge.label);
        entry.kinds.add(edge.kind);
        merged.set(key, entry);
      }
      for (const [key, entry] of merged) {
        flowEdges.push({
          id: key,
          source: `scene:${entry.from}`,
          target: `scene:${entry.to}`,
          label: entry.labels.slice(0, 2).join(" / ").slice(0, 46),
          type: "smoothstep",
          animated: entry.kinds.has("condition"),
          style: { stroke: colorOf(entry.kinds.has("choice") ? "choice" : entry.kinds.has("condition") ? "condition" : "jump") },
          markerEnd: { type: MarkerType.ArrowClosed, color: colorOf(entry.kinds.has("choice") ? "choice" : "jump") },
          labelStyle: { fill: "var(--muted)", fontSize: 10 },
          labelBgStyle: { fill: "var(--panel)", fillOpacity: 0.85 },
          reconnectable: true,
          data: { sceneId: entry.from, nodeId: entry.nodeId, optionId: entry.optionId ?? null },
        });
      }
    } else {
      const flowKinds = ["menu", "if", "pointclick"];
      for (const scene of project.scenes) {
        for (const node of scene.nodes) {
          if (!flowKinds.includes(node.kind)) continue;
          const outgoing = data.edges.filter((e) => e.nodeId === node.id);
          if (outgoing.length === 0) continue;
          const id = `block:${node.id}`;
          flowNodes.push({
            id,
            position: { x: 0, y: 0 },
            data: {
              label:
                node.kind === "menu"
                  ? t("block.menu")
                  : node.kind === "if"
                    ? t("block.if")
                    : t("block.pointclick"),
              tone: node.kind === "menu" ? "var(--accent)" : node.kind === "if" ? "var(--warn)" : "var(--accent)",
            },
            type: "flow",
            style: { width: 170 },
          });
          g.setNode(id, { width: 170, height: 34 });
          g.setEdge(`scene:${scene.id}`, id);
          flowEdges.push({
            id: `in:${node.id}`,
            source: `scene:${scene.id}`,
            target: id,
            type: "smoothstep",
            style: { stroke: "#3a3a52", strokeDasharray: "3 3" },
            reconnectable: false,
          });
          for (const edge of outgoing) {
            if (!edge.toScene) continue;
            g.setEdge(id, `scene:${edge.toScene}`);
            flowEdges.push({
              id: edge.id,
              source: id,
              target: `scene:${edge.toScene}`,
              label: edge.label.slice(0, 30),
              type: "smoothstep",
              animated: edge.kind === "condition",
              style: { stroke: colorOf(edge.kind) },
              markerEnd: { type: MarkerType.ArrowClosed, color: colorOf(edge.kind) },
              labelStyle: { fill: "var(--muted)", fontSize: 10 },
              labelBgStyle: { fill: "var(--panel)", fillOpacity: 0.85 },
              reconnectable: true,
              data: {
                sceneId: scene.id,
                nodeId: node.id,
                optionId: edge.kind === "choice" ? edge.id.split(":")[1] : null,
                branch: edge.kind === "condition" ? (edge.id.endsWith(":else") ? "else" : "then") : null,
              },
            });
          }
        }
      }
    }

    dagre.layout(g);
    const positioned = flowNodes.map((node) => {
      const pos = g.node(node.id);
      return pos
        ? { ...node, position: { x: pos.x - (pos.width ?? 200) / 2, y: pos.y - (pos.height ?? 72) / 2 } }
        : node;
    });
    return { nodes: positioned, edges: flowEdges };
  }, [project, graph, detail, activeSceneId, t, totalBlocks]);

  useEffect(() => {
    if (nodes.length === 0) return;
    const timer = window.setTimeout(() => {
      void fitView({ padding: 0.2, duration: first.current ? 0 : 320, minZoom: detail === "all" ? 0.3 : 0.78, maxZoom: 1 });
      first.current = false;
    }, 60);
    return () => window.clearTimeout(timer);
  }, [nodes.length, detail, fitView]);

  const matches = useMemo(() => {
    if (!project || !query.trim()) return [];
    const q = query.trim().toLowerCase();
    return project.scenes.filter((scene) => scene.title.toLowerCase().includes(q));
  }, [project, query]);

  const displayNodes = useMemo(() => {
    if (!query.trim()) return nodes;
    const matchIds = new Set(matches.map((scene) => `scene:${scene.id}`));
    return nodes.map((node) =>
      node.type === "scene" || node.type === "block"
        ? { ...node, style: { ...(node.style ?? {}), opacity: node.type === "scene" && !matchIds.has(node.id) ? 0.25 : 1 } }
        : node,
    );
  }, [nodes, matches, query]);

  const jumpToScene = useCallback(
    (sceneId: string) => {
      void fitView({ nodes: [{ id: `scene:${sceneId}` }], duration: 420, maxZoom: 1.1, minZoom: 0.4 });
    },
    [fitView],
  );

  const onReconnect = useCallback(
    (oldEdge: Edge, connection: Connection) => {
      const target = connection.target;
      if (!target?.startsWith("scene:")) return;
      const meta = oldEdge.data as
        | { sceneId?: string; nodeId?: string; optionId?: string | null; branch?: "then" | "else" | null }
        | undefined;
      if (!meta?.sceneId || !meta.nodeId) return;
      void retargetEdge({
        sceneId: meta.sceneId,
        nodeId: meta.nodeId,
        target: target.slice(6),
        optionId: meta.optionId ?? null,
        branch: meta.branch ?? null,
      });
    },
    [retargetEdge],
  );

  const onNodeDoubleClick = useCallback(
    (_: unknown, node: Node) => {
      if (node.id.startsWith("scene:")) {
        selectScene(node.id.slice(6));
        setTab("scene");
      } else if (node.id.startsWith("b:")) {
        const nodeId = node.id.slice(2);
        const scene = project?.scenes.find((entry) => entry.nodes.some((item) => item.id === nodeId));
        if (scene) {
          selectScene(scene.id);
          selectNode(nodeId);
          setTab("scene");
        }
      }
    },
    [selectScene, selectNode, setTab, project],
  );

  if (!project) return null;

  return (
    <div className="relative flex h-full flex-col">
      <div className="flex h-[44px] flex-none items-center justify-between gap-3 border-b border-[var(--border)] px-4">
        <div className="flex items-center gap-2">
          <span className="text-[13px] font-semibold">{t("graph.title")}</span>
          <span className="chip">{project.scenes.length}</span>
          {detail === "all" ? <span className="chip">{t("graph.blocksCount", { count: totalBlocks })}</span> : null}
        </div>
        <div className="flex items-center gap-2">
          <div className="relative">
            <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--muted-2)]" />
            <Input
              className="h-[28px] w-[190px] pl-7 text-[12px]"
              placeholder={t("graph.search")}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && matches[0]) jumpToScene(matches[0].id);
              }}
            />
          </div>
          <Segmented
            value={detail}
            onChange={(value) => void saveSettings({ ...settings, graphDetail: value })}
            options={[
              { value: "scenes", label: t("graph.detail.scenes") },
              { value: "choices", label: t("graph.detail.choices") },
              { value: "all", label: t("graph.detail.all") },
            ]}
          />
        </div>
      </div>
      <div className="relative flex-1">
        {nodes.length === 0 ? (
          <div className="flex h-full items-center justify-center text-[12px] text-[var(--muted)]">
            {t("graph.empty")}
          </div>
        ) : (
          <ReactFlow
            nodes={displayNodes}
            edges={edges}
            nodeTypes={nodeTypes}
            onNodeDoubleClick={onNodeDoubleClick}
            onNodeClick={(_, node) => {
              if (node.id.startsWith("scene:")) selectScene(node.id.slice(6));
            }}
            onReconnect={onReconnect}
            onlyRenderVisibleElements
            fitView
            minZoom={0.1}
            maxZoom={2}
            proOptions={{ hideAttribution: true }}
          >
            <Background variant={BackgroundVariant.Dots} gap={18} size={1} color="var(--border)" />
            <Controls showInteractive={false} position="bottom-right" />
            <MiniMap
              pannable
              zoomable
              nodeColor={(node) => (node.id.startsWith("scene:") ? "#8b6cf2" : node.id.startsWith("b:") ? "#2fc7c0" : "#f0b429")}
              maskColor="rgba(0,0,0,0.12)"
              style={{ width: 150, height: 96 }}
            />
          </ReactFlow>
        )}
        <div className="pointer-events-none absolute bottom-3 left-3 flex items-center gap-3 rounded-lg border border-[var(--border)] bg-[var(--panel)]/90 px-2.5 py-1.5 text-[10px] text-[var(--muted)]">
          <span className="flex items-center gap-1">
            <span className="h-[2px] w-4 bg-[var(--muted-2)]" /> {t("graph.legend.jump")}
          </span>
          <span className="flex items-center gap-1">
            <span className="h-[2px] w-4 bg-[#8b6cf2]" /> {t("graph.legend.choice")}
          </span>
          <span className="flex items-center gap-1">
            <span className="h-[2px] w-4 bg-[#f0b429]" /> {t("graph.legend.condition")}
          </span>
          <span className="flex items-center gap-1">
            <Flag size={10} /> {t("graph.start")}
          </span>
          <span className="flex items-center gap-1">
            <Split size={10} /> {t("block.menu")}
          </span>
        </div>
      </div>
    </div>
  );
}

export function GraphView() {
  return (
    <ReactFlowProvider>
      <GraphInner />
    </ReactFlowProvider>
  );
}
