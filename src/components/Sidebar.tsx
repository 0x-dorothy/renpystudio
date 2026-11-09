import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ChevronRight,
  FileText,
  Image as ImageIcon,
  Music,
  Plus,
  Search,
  UserRound,
  Variable as VariableIcon,
} from "lucide-react";
import { useStudio } from "../store";
import { IconButton, Input, SectionHeader } from "./ui";
import { newId } from "../util";
import type { Character } from "../types";

export function Sidebar() {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const assets = useStudio((s) => s.assets);
  const activeSceneId = useStudio((s) => s.activeSceneId);
  const selectScene = useStudio((s) => s.selectScene);
  const setTab = useStudio((s) => s.setTab);
  const createScene = useStudio((s) => s.createScene);
  const reorderScene = useStudio((s) => s.reorderScene);
  const saveCharacter = useStudio((s) => s.saveCharacter);
  const saveVariable = useStudio((s) => s.saveVariable);
  const selectCharacter = useStudio((s) => s.selectCharacter);
  const selectVariable = useStudio((s) => s.selectVariable);
  const selectedCharacterId = useStudio((s) => s.selectedCharacterId);
  const selectedVariableId = useStudio((s) => s.selectedVariableId);
  const [query, setQuery] = useState("");
  const [dragScene, setDragScene] = useState<string | null>(null);
  const [expanded, setExpanded] = useState({ scenes: true, characters: true, variables: true, assets: true });

  const filtered = useMemo(() => {
    if (!project) return { scenes: [], characters: [], variables: [] };
    const q = query.trim().toLowerCase();
    return {
      scenes: project.scenes.filter((s) => !q || s.title.toLowerCase().includes(q)),
      characters: project.characters.filter((c) => !q || c.name.toLowerCase().includes(q)),
      variables: project.variables.filter((v) => !q || v.name.toLowerCase().includes(q)),
    };
  }, [project, query]);

  if (!project) return null;

  const images = assets.filter((a) => a.kind === "image").length;
  const audio = assets.filter((a) => a.kind === "audio").length;

  const addCharacter = () => {
    const character: Character = {
      id: newId(),
      name: t("characters.newName", { n: project.characters.length + 1 }),
      kind: "character",
      color: "#e96aa5",
      avatar: null,
      spritePrefix: null,
      voiceTag: null,
      prefix: null,
      suffix: null,
      notes: "",
    };
    void saveCharacter(character).then(() => {
      setTab("characters");
      selectCharacter(character.id);
    });
  };

  const addVariable = () => {
    const variable = {
      id: newId(),
      name: `variable_${project.variables.length + 1}`,
      varType: "number" as const,
      default: 0,
      description: "",
    };
    void saveVariable(variable).then(() => {
      setTab("variables");
      selectVariable(variable.id);
    });
  };

  return (
    <aside className="flex h-full w-[248px] flex-none flex-col border-r border-[var(--border)] bg-[var(--panel)]">
      <div className="flex h-[42px] flex-none items-center gap-2 border-b border-[var(--border)] px-2.5">
        <div className="relative flex-1">
          <Search size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-[var(--muted-2)]" />
          <Input
            className="h-[28px] pl-7 text-[12px]"
            placeholder={t("nav.search")}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </div>
      </div>

      <div className="scroll flex-1 py-1">
        <SectionHeader
          title={
            <button
              className="flex items-center gap-1 hover:text-[var(--text)]"
              onClick={() => setExpanded({ ...expanded, scenes: !expanded.scenes })}
            >
              <ChevronRight size={11} className={expanded.scenes ? "rotate-90 transition-transform" : "transition-transform"} />
              {t("nav.scenes")}
              <span className="ml-1 text-[var(--muted-2)]">{project.scenes.length}</span>
            </button>
          }
          right={
            <IconButton
              title={t("action.addScene")}
              onClick={() => void createScene(t("scene.newTitle"))}
            >
              <Plus size={13} />
            </IconButton>
          }
        />
        {expanded.scenes ? (
          <div className="px-1.5 pb-1">
            {filtered.scenes.map((scene) => {
              const isStart = project.meta.startScene === scene.id;
              return (
                <div
                  key={scene.id}
                  draggable={!query}
                  onDragStart={() => setDragScene(scene.id)}
                  onDragOver={(event) => event.preventDefault()}
                  onDrop={() => {
                    if (!dragScene || dragScene === scene.id) return;
                    const target = project.scenes.findIndex((s) => s.id === scene.id);
                    void reorderScene(dragScene, target);
                    setDragScene(null);
                  }}
                  onClick={() => {
                    selectScene(scene.id);
                    setTab("scene");
                  }}
                  className={`group flex cursor-pointer items-center gap-2 rounded-md px-2 py-[5px] text-[12.5px] ${
                    activeSceneId === scene.id
                      ? "bg-[var(--accent-soft)] text-[var(--text)]"
                      : "text-[var(--muted)] hover:bg-[var(--panel-2)] hover:text-[var(--text)]"
                  }`}
                >
                  <FileText size={13} className={isStart ? "text-[var(--accent)]" : "text-[var(--muted-2)]"} />
                  <span className="flex-1 truncate">{scene.title}</span>
                  {isStart ? <span className="h-1.5 w-1.5 rounded-full bg-[var(--accent)]" /> : null}
                  <span className="text-[10px] text-[var(--muted-2)] opacity-0 group-hover:opacity-100">
                    {scene.nodes.length}
                  </span>
                </div>
              );
            })}
            {filtered.scenes.length === 0 ? (
              <div className="px-2 py-1 text-[11px] text-[var(--muted-2)]">—</div>
            ) : null}
          </div>
        ) : null}

        <SectionHeader
          title={
            <button
              className="flex items-center gap-1 hover:text-[var(--text)]"
              onClick={() => setExpanded({ ...expanded, characters: !expanded.characters })}
            >
              <ChevronRight size={11} className={expanded.characters ? "rotate-90 transition-transform" : "transition-transform"} />
              {t("nav.characters")}
              <span className="ml-1 text-[var(--muted-2)]">{project.characters.length}</span>
            </button>
          }
          right={
            <IconButton title={t("action.addCharacter")} onClick={addCharacter}>
              <Plus size={13} />
            </IconButton>
          }
        />
        {expanded.characters ? (
          <div className="px-1.5 pb-1">
            {filtered.characters.map((character) => (
              <div
                key={character.id}
                onClick={() => {
                  setTab("characters");
                  selectCharacter(character.id);
                }}
                className={`flex cursor-pointer items-center gap-2 rounded-md px-2 py-[5px] text-[12.5px] ${
                  selectedCharacterId === character.id
                    ? "bg-[var(--accent-soft)] text-[var(--text)]"
                    : "text-[var(--muted)] hover:bg-[var(--panel-2)] hover:text-[var(--text)]"
                }`}
              >
                <span
                  className="h-2 w-2 flex-none rounded-full"
                  style={{ background: character.color ?? "var(--muted-2)" }}
                />
                <span className="flex-1 truncate">{character.name}</span>
                <UserRound size={11} className="text-[var(--muted-2)]" />
              </div>
            ))}
            {filtered.characters.length === 0 ? (
              <div className="px-2 py-1 text-[11px] text-[var(--muted-2)]">—</div>
            ) : null}
          </div>
        ) : null}

        <SectionHeader
          title={
            <button
              className="flex items-center gap-1 hover:text-[var(--text)]"
              onClick={() => setExpanded({ ...expanded, variables: !expanded.variables })}
            >
              <ChevronRight size={11} className={expanded.variables ? "rotate-90 transition-transform" : "transition-transform"} />
              {t("nav.variables")}
              <span className="ml-1 text-[var(--muted-2)]">{project.variables.length}</span>
            </button>
          }
          right={
            <IconButton title={t("action.addVariable")} onClick={addVariable}>
              <Plus size={13} />
            </IconButton>
          }
        />
        {expanded.variables ? (
          <div className="px-1.5 pb-1">
            {filtered.variables.map((variable) => (
              <div
                key={variable.id}
                onClick={() => {
                  setTab("variables");
                  selectVariable(variable.id);
                }}
                className={`flex cursor-pointer items-center gap-2 rounded-md px-2 py-[5px] text-[12.5px] ${
                  selectedVariableId === variable.id
                    ? "bg-[var(--accent-soft)] text-[var(--text)]"
                    : "text-[var(--muted)] hover:bg-[var(--panel-2)] hover:text-[var(--text)]"
                }`}
              >
                <VariableIcon size={12} className="text-[var(--muted-2)]" />
                <span className="mono flex-1 truncate text-[12px]">{variable.name}</span>
                <span className="text-[10px] text-[var(--muted-2)]">
                  {variable.varType === "number" ? "123" : variable.varType === "bool" ? "T/F" : "abc"}
                </span>
              </div>
            ))}
            {filtered.variables.length === 0 ? (
              <div className="px-2 py-1 text-[11px] text-[var(--muted-2)]">—</div>
            ) : null}
          </div>
        ) : null}

        <SectionHeader
          title={
            <button
              className="flex items-center gap-1 hover:text-[var(--text)]"
              onClick={() => setExpanded({ ...expanded, assets: !expanded.assets })}
            >
              <ChevronRight size={11} className={expanded.assets ? "rotate-90 transition-transform" : "transition-transform"} />
              {t("nav.assets")}
            </button>
          }
        />
        {expanded.assets ? (
          <div className="px-1.5 pb-2">
            <button
              onClick={() => setTab("assets")}
              className="flex w-full items-center gap-2 rounded-md px-2 py-[5px] text-[12.5px] text-[var(--muted)] hover:bg-[var(--panel-2)] hover:text-[var(--text)]"
            >
              <ImageIcon size={12} /> <span className="flex-1 text-left">{t("assets.images")}</span>
              <span className="text-[10px] text-[var(--muted-2)]">{images}</span>
            </button>
            <button
              onClick={() => setTab("assets")}
              className="flex w-full items-center gap-2 rounded-md px-2 py-[5px] text-[12.5px] text-[var(--muted)] hover:bg-[var(--panel-2)] hover:text-[var(--text)]"
            >
              <Music size={12} /> <span className="flex-1 text-left">{t("assets.audio")}</span>
              <span className="text-[10px] text-[var(--muted-2)]">{audio}</span>
            </button>
          </div>
        ) : null}
      </div>
    </aside>
  );
}
