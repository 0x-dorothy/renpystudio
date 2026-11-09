import { useTranslation } from "react-i18next";
import { Plus, Trash2 } from "lucide-react";
import { useStudio } from "../store";
import { Button, Empty, IconButton } from "./ui";
import { newId } from "../util";

export function VariablesTab() {
  const { t } = useTranslation();
  const project = useStudio((s) => s.project);
  const saveVariable = useStudio((s) => s.saveVariable);
  const deleteVariable = useStudio((s) => s.deleteVariable);
  const selectVariable = useStudio((s) => s.selectVariable);
  const selectedVariableId = useStudio((s) => s.selectedVariableId);

  if (!project) return null;

  const add = () => {
    const variable = {
      id: newId(),
      name: `variable_${project.variables.length + 1}`,
      varType: "number" as const,
      default: 0,
      description: "",
    };
    void saveVariable(variable).then(() => selectVariable(variable.id));
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-[52px] flex-none items-center justify-between border-b border-[var(--border)] px-5">
        <div className="flex items-center gap-2">
          <h2 className="text-[14px] font-semibold">{t("variables.title")}</h2>
          <span className="chip">{project.variables.length}</span>
        </div>
        <Button size="sm" variant="primary" onClick={add}>
          <Plus size={14} /> {t("action.addVariable")}
        </Button>
      </div>
      <div className="scroll flex-1 px-6 py-4">
        <p className="mb-4 text-[12px] text-[var(--muted)]">{t("variables.hint")}</p>
        {project.variables.length === 0 ? (
          <Empty title={t("variables.empty")} action={<Button size="sm" onClick={add}><Plus size={14} /> {t("action.addVariable")}</Button>} />
        ) : (
          <div className="overflow-hidden rounded-xl border border-[var(--border)]">
            <table className="w-full border-collapse text-left text-[12.5px]">
              <thead className="bg-[var(--panel-2)] text-[10px] uppercase tracking-[0.06em] text-[var(--muted-2)]">
                <tr>
                  <th className="px-3 py-2 font-semibold">{t("variables.name")}</th>
                  <th className="px-3 py-2 font-semibold">{t("variables.type")}</th>
                  <th className="px-3 py-2 font-semibold">{t("variables.default")}</th>
                  <th className="px-3 py-2 font-semibold">{t("common.description")}</th>
                  <th className="px-3 py-2" />
                </tr>
              </thead>
              <tbody>
                {project.variables.map((variable) => (
                  <tr
                    key={variable.id}
                    onClick={() => selectVariable(variable.id)}
                    className={`cursor-pointer border-t border-[var(--border-soft)] ${
                      selectedVariableId === variable.id ? "bg-[var(--accent-soft)]" : "hover:bg-[var(--panel-2)]"
                    }`}
                  >
                    <td className="mono px-3 py-2 text-[12px] font-medium">{variable.name}</td>
                    <td className="px-3 py-2 text-[var(--muted)]">{t(`variables.type.${variable.varType}`)}</td>
                    <td className="mono px-3 py-2 text-[11.5px] text-[var(--muted)]">
                      {JSON.stringify(variable.default ?? null)}
                    </td>
                    <td className="max-w-[280px] truncate px-3 py-2 text-[12px] text-[var(--muted)]">
                      {variable.description || "—"}
                    </td>
                    <td className="px-2 py-1 text-right">
                      <IconButton
                        title={t("common.delete")}
                        onClick={(event) => {
                          event.stopPropagation();
                          if (window.confirm(t("variables.deleteConfirm", { name: variable.name }))) {
                            void deleteVariable(variable.id);
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
        <div className="mt-6 rounded-xl border border-[var(--border)] bg-[var(--panel-2)] p-3.5">
          <div className="mono text-[11.5px] leading-relaxed text-[var(--muted)]">
            {project.variables.slice(0, 4).map((variable) => (
              <div key={variable.id}>
                default <span className="text-[var(--text)]">{variable.name}</span> ={" "}
                <span className="text-[var(--accent)]">{JSON.stringify(variable.default ?? null)}</span>
              </div>
            ))}
            {project.variables.length > 4 ? <div>…</div> : null}
          </div>
        </div>
      </div>
    </div>
  );
}
