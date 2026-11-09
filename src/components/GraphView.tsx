import { useCallback, useEffect, useMemo, useRef } from "react";
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
  type Edge,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import dagre from "@dagrejs/dagre";
import { AlertTriangle, Flag, Play, Split } from "lucide-react";
import { useStudio } from "../store";
import { Segmented } from "./ui";
import type { GraphData } from "../types";

interface SceneNodeData extends Record<string, unknown> {
  title: string;
  blocks: number;
  isStart: boolean;
  deadEnd: boolean;
  unreachable: boolean;
  color: string | null;
}

type SceneNodeType = Node<SceneNodeData, "scene">;

function SceneNode({ data, selected }: NodeProps<SceneNodeType>) {
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
            <AlertTriangle size={9} /> тупик
          </span>
        ) : null}
        {data.unreachable ? <span className="chip">недостижима</span> : null}
        {data.isStart ? <span className="chip chip-accent">старт</span> : null}
      </div>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

const nodeTypes = { scene: SceneNode, flow: FlowNode };

function FlowNode({ data, selected }: NodeProps<Node<{ label: string; tone: string }, "flow">>) {
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

function GraphInner() {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const graph = useStudio((s) => s.graph);
  const settings = useStudio((s) => s.settings);
  const saveSettings = useStudio((s) => s.saveSettings);
  const refreshGraph = useStudio((s) => s.refreshGraph);
  const selectScene = useStudio((s) => s.selectScene);
  const setTab = useStudio((s) => s.setTab);
  const activeSceneId = useStudio((s) => s.activeSceneId);
  const { fitView } = useReactFlow();
  const first = useRef(true);

  useEffect(() => {
    void refreshGraph();
  }, [refreshGraph]);

  const detail = settings.graphDetail;

  const { nodes, edges } = useMemo(() => {
    if (!project || !graph) return { nodes: [] as Node[], edges: [] as Edge[] };
    const data: GraphData = graph;
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
    const colorOf = (kind: string) =>
      kind === "choice" ? "#8b6cf2" : kind === "condition" ? "#f0b429" : "#6d6a85";

    if (detail === "scenes") {
      const merged = new Map<string, { from: string; to: string; labels: string[]; kinds: Set<string> }>();
      for (const edge of data.edges) {
        if (!edge.toScene) continue;
        const key = `${edge.fromScene}->${edge.toScene}`;
        const entry = merged.get(key) ?? { from: edge.fromScene, to: edge.toScene, labels: [], kinds: new Set<string>() };
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
        });
      }
    } else {
      // Blocks with outgoing edges become small nodes.
      const flowKinds = new Set(["menu", "if"]);
      for (const scene of project.scenes) {
        for (const node of scene.nodes) {
          if (!flowKinds.has(node.kind)) continue;
          const outgoing = data.edges.filter((e) => e.nodeId === node.id);
          if (outgoing.length === 0) continue;
          const id = `block:${node.id}`;
          flowNodes.push({
            id,
            position: { x: 0, y: 0 },
            data: {
              label: node.kind === "menu" ? t("block.menu") : t("block.if"),
              tone: node.kind === "menu" ? "var(--accent)" : "var(--warn)",
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
            });
          }
        }
      }
    }

    dagre.layout(g);
    const positioned = flowNodes.map((node) => {
      const pos = g.node(node.id);
      return pos
        ? { ...node, position: { x: pos.x - (pos.width ?? 220) / 2, y: pos.y - (pos.height ?? 76) / 2 } }
        : node;
    });
    return { nodes: positioned, edges: flowEdges };
  }, [project, graph, detail, activeSceneId, t]);

  useEffect(() => {
    if (nodes.length === 0) return;
    const timer = window.setTimeout(() => {
      void fitView({ padding: 0.2, duration: first.current ? 0 : 320, minZoom: 0.78, maxZoom: 1 });
      first.current = false;
    }, 60);
    return () => window.clearTimeout(timer);
  }, [nodes.length, detail, fitView]);

  const onNodeDoubleClick = useCallback(
    (_: unknown, node: Node) => {
      if (node.id.startsWith("scene:")) {
        selectScene(node.id.slice(6));
        setTab("scene");
      }
    },
    [selectScene, setTab],
  );

  if (!project) return null;

  return (
    <div className="relative flex h-full flex-col">
      <div className="flex h-[44px] flex-none items-center justify-between border-b border-[var(--border)] px-4">
        <div className="flex items-center gap-2">
          <span className="text-[13px] font-semibold">{t("graph.title")}</span>
          <span className="chip">{project.scenes.length}</span>
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
      <div className="relative flex-1">
        {nodes.length === 0 ? (
          <div className="flex h-full items-center justify-center text-[12px] text-[var(--muted)]">
            {t("graph.empty")}
          </div>
        ) : (
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            onNodeDoubleClick={onNodeDoubleClick}
            onNodeClick={(_, node) => {
              if (node.id.startsWith("scene:")) selectScene(node.id.slice(6));
            }}
            fitView
            minZoom={0.15}
            maxZoom={2}
            proOptions={{ hideAttribution: true }}
          >
            <Background variant={BackgroundVariant.Dots} gap={18} size={1} color="var(--border)" />
            <Controls showInteractive={false} position="bottom-right" />
            <MiniMap
              pannable
              zoomable
              nodeColor={(node) =>
                node.id.startsWith("scene:") ? "#8b6cf2" : "#f0b429"
              }
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
