import { useTranslation } from "react-i18next";
import { Plus, Trash2, UserRound } from "lucide-react";
import { useStudio } from "../store";
import { Button, Empty, IconButton } from "./ui";
import { newId } from "../util";
import type { Character } from "../types";

export function CharactersTab() {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const saveCharacter = useStudio((s) => s.saveCharacter);
  const deleteCharacter = useStudio((s) => s.deleteCharacter);
  const selectCharacter = useStudio((s) => s.selectCharacter);
  const selectedCharacterId = useStudio((s) => s.selectedCharacterId);

  if (!project) return null;

  const add = () => {
    const character: Character = {
      id: newId(),
      name: `${t("characters.title")} ${project.characters.length + 1}`,
      kind: "character",
      color: "#e96aa5",
      avatar: null,
      spritePrefix: null,
      voiceTag: null,
      prefix: null,
      suffix: null,
      notes: "",
    };
    void saveCharacter(character).then(() => selectCharacter(character.id));
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-[52px] flex-none items-center justify-between border-b border-[var(--border)] px-5">
        <div className="flex items-center gap-2">
          <h2 className="text-[14px] font-semibold">{t("characters.title")}</h2>
          <span className="chip">{project.characters.length}</span>
        </div>
        <Button size="sm" variant="primary" onClick={add}>
          <Plus size={14} /> {t("action.addCharacter")}
        </Button>
      </div>
      <div className="scroll flex-1 px-6 py-4">
        <p className="mb-4 text-[12px] text-[var(--muted)]">{t("characters.hint")}</p>
        {project.characters.length === 0 ? (
          <Empty title={t("characters.empty")} action={<Button size="sm" onClick={add}><Plus size={14} /> {t("action.addCharacter")}</Button>} />
        ) : (
          <div className="overflow-hidden rounded-xl border border-[var(--border)]">
            <table className="w-full border-collapse text-left text-[12.5px]">
              <thead className="bg-[var(--panel-2)] text-[10px] uppercase tracking-[0.06em] text-[var(--muted-2)]">
                <tr>
                  <th className="px-3 py-2 font-semibold">{t("characters.name")}</th>
                  <th className="px-3 py-2 font-semibold">{t("characters.kind")}</th>
                  <th className="px-3 py-2 font-semibold">{t("characters.spritePrefix")}</th>
                  <th className="px-3 py-2 font-semibold">{t("characters.color")}</th>
                  <th className="px-3 py-2" />
                </tr>
              </thead>
              <tbody>
                {project.characters.map((character) => (
                  <tr
                    key={character.id}
                    onClick={() => selectCharacter(character.id)}
                    className={`cursor-pointer border-t border-[var(--border-soft)] ${
                      selectedCharacterId === character.id ? "bg-[var(--accent-soft)]" : "hover:bg-[var(--panel-2)]"
                    }`}
                  >
                    <td className="px-3 py-2">
                      <div className="flex items-center gap-2">
                        <span className="h-2 w-2 rounded-full" style={{ background: character.color ?? "var(--muted-2)" }} />
                        <span className="font-medium">{character.name || "—"}</span>
                      </div>
                    </td>
                    <td className="px-3 py-2 text-[var(--muted)]">
                      {t(`characters.kind.${character.kind}`)}
                    </td>
                    <td className="mono px-3 py-2 text-[11.5px] text-[var(--muted)]">
                      {character.spritePrefix ?? "—"}
                    </td>
                    <td className="px-3 py-2">
                      <span className="mono text-[11px] text-[var(--muted)]">{character.color ?? "—"}</span>
                    </td>
                    <td className="px-2 py-1 text-right">
                      <IconButton
                        title={t("common.delete")}
                        onClick={(event) => {
                          event.stopPropagation();
                          if (window.confirm(t("characters.deleteConfirm", { name: character.name }))) {
                            void deleteCharacter(character.id);
                          }
                        }}
                      >
                        <Trash2 size={13} className="text-[var(--danger)]" />
                      </IconButton>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        <div className="mt-6 flex items-start gap-3 rounded-xl border border-[var(--border)] bg-[var(--panel-2)] p-3.5">
          <UserRound size={16} className="mt-0.5 text-[var(--accent)]" />
          <div className="text-[12px] leading-relaxed text-[var(--muted)]">
            <span className="mono text-[11.5px] text-[var(--text)]">
              define mira = Character("Мира", color="#e96aa5", image="mira")
            </span>
            <p className="mt-1">{t("characters.hint")}</p>
          </div>
        </div>
      </div>
    </div>
  );
}
