import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertTriangle,
  ArrowUp,
  RefreshCw,
  Settings2,
  Sparkles,
  Trash2,
  X,
} from "lucide-react";
import { useStudio } from "../store";
import { Button, Field, IconButton, Select, Spinner } from "./ui";
import type { AiAction } from "../types";

/**
 * Right-hand assistant drawer. The API key lives in settings and is used only by
 * the Rust side; the panel never sees it and never logs it.
 */
export function AiPanel() {
  const { t } = useTranslation();
  const open = useStudio((s) => s.aiOpen);
  if (!open) return null;
  return <AiPanelBody />;
}

function AiPanelBody() {
  const { t } = useTranslation();
  const close = useStudio((s) => s.toggleAi);
  const settings = useStudio((s) => s.settings);
  const saveSettings = useStudio((s) => s.saveSettings);
  const aiModels = useStudio((s) => s.aiModels);
  const aiMessages = useStudio((s) => s.aiMessages);
  const aiBusy = useStudio((s) => s.aiBusy);
  const aiError = useStudio((s) => s.aiError);
  const refreshAiModels = useStudio((s) => s.refreshAiModels);
  const sendAiMessage = useStudio((s) => s.sendAiMessage);
  const applyAiActions = useStudio((s) => s.applyAiActions);
  const clearAiChat = useStudio((s) => s.clearAiChat);
  const openModal = useStudio((s) => s.openModal);
  const project = useStudio((s) => s.project);

  const [draft, setDraft] = useState("");
  const listRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const node = listRef.current;
    if (node) node.scrollTop = node.scrollHeight;
  }, [aiMessages.length, aiBusy]);

  const configured = Boolean(settings.ai.baseUrl.trim() && settings.ai.model.trim());
  const canSend = configured && draft.trim().length > 0 && !aiBusy;

  // A saved model may be missing from a freshly loaded list (provider renamed or
  // the request failed): keep it selectable so the choice is never lost.
  const modelOptions = useMemo(() => {
    const options = aiModels.map((model) => ({ value: model.id, label: model.id }));
    const current = settings.ai.model.trim();
    if (current && !options.some((option) => option.value === current)) {
      options.unshift({ value: current, label: current });
    }
    return options;
  }, [aiModels, settings.ai.model]);

  const submit = () => {
    if (!canSend) return;
    const text = draft;
    setDraft("");
    void sendAiMessage(text);
  };

  const sceneTitle = (sceneId: string | undefined) =>
    project?.scenes.find((scene) => scene.id === sceneId)?.title ?? sceneId ?? "…";

  return (
    <aside className="fixed bottom-[26px] right-0 top-[46px] z-30 flex w-[400px] flex-none flex-col border-l border-[var(--border)] bg-[var(--panel)] shadow-2xl">
      <div className="flex h-[42px] flex-none items-center justify-between gap-2 border-b border-[var(--border)] px-3">
        <div className="flex items-center gap-2">
          <Sparkles size={15} className="text-[var(--accent)]" />
          <span className="text-[11px] font-bold uppercase tracking-[0.08em] text-[var(--muted-2)]">
            {t("ai.title")}
          </span>
        </div>
        <div className="flex items-center gap-1">
          <IconButton title={t("ai.refreshModels")} onClick={() => void refreshAiModels()}>
            <RefreshCw size={14} />
          </IconButton>
          <IconButton title={t("ai.clear")} onClick={clearAiChat}>
            <Trash2 size={14} />
          </IconButton>
          <IconButton title={t("common.close")} onClick={() => void close()}>
            <X size={15} />
          </IconButton>
        </div>
      </div>

      <div className="flex-none border-b border-[var(--border)] px-3 py-2">
        <Field label={t("ai.model")}>
          <div className="flex items-center gap-1.5">
            <Select
              className="h-[30px] text-[12px]"
              value={settings.ai.model}
              placeholder={aiModels.length ? t("ai.pickModel") : t("ai.noModels")}
              options={modelOptions}
              onChange={(event) => {
                const model = event.target.value;
                void saveSettings({ ...settings, ai: { ...settings.ai, model } });
              }}
            />
            <IconButton title={t("menu.settings")} onClick={() => openModal("settings")}>
              <Settings2 size={14} />
            </IconButton>
          </div>
        </Field>
        {!configured ? (
          <div className="flex items-start gap-2 rounded-lg border border-[var(--warn)]/40 bg-[var(--warn)]/10 px-2 py-1.5 text-[11px] leading-snug text-[var(--warn)]">
            <AlertTriangle size={13} className="mt-0.5 flex-none" />
            <span>
              {t("ai.notConfigured")}{" "}
              <button
                type="button"
                className="underline"
                onClick={() => openModal("settings")}
              >
                {t("ai.openSettings")}
              </button>
            </span>
          </div>
        ) : !settings.ai.hasKey && !settings.ai.apiKey ? (
          <div className="text-[10.5px] leading-snug text-[var(--warn)]">{t("ai.keyMissing")}</div>
        ) : null}
      </div>

      <div ref={listRef} className="scroll flex-1 space-y-3 px-3 py-3">
        {aiMessages.length === 0 ? (
          <div className="mt-6 flex flex-col items-center gap-2 px-2 text-center">
            <div className="flex h-10 w-10 items-center justify-center rounded-full bg-[var(--accent-soft)] text-[var(--accent)]">
              <Sparkles size={18} />
            </div>
            <p className="text-[12px] leading-relaxed text-[var(--muted)]">{t("ai.empty")}</p>
            <div className="mt-1 flex flex-col gap-1.5">
              {[t("ai.example1"), t("ai.example2"), t("ai.example3")].map((example) => (
                <button
                  key={example}
                  type="button"
                  className="rounded-lg border border-[var(--border)] bg-[var(--bg-soft)] px-2.5 py-1.5 text-left text-[11px] text-[var(--muted)] transition-colors hover:border-[var(--accent)]/50 hover:text-[var(--text)]"
                  onClick={() => setDraft(example)}
                >
                  {example}
                </button>
              ))}
            </div>
          </div>
        ) : null}

        {aiMessages.map((message) => (
          <div
            key={message.id}
            className={`flex ${message.role === "user" ? "justify-end" : "justify-start"}`}
          >
            <div
              className={`max-w-[92%] rounded-xl px-3 py-2 text-[12.5px] leading-relaxed whitespace-pre-wrap ${
                message.role === "user"
                  ? "bg-[var(--accent-soft)] text-[var(--text)]"
                  : message.error
                    ? "border border-[var(--danger)]/40 bg-[var(--danger)]/10 text-[var(--danger)]"
                    : "border border-[var(--border)] bg-[var(--bg-soft)] text-[var(--text)]"
              }`}
            >
              {message.content ? message.content : null}
              {!message.content && message.actions?.length ? (
                <span className="text-[11px] text-[var(--muted-2)]">{t("ai.actionsReady")}</span>
              ) : null}
              {message.actions && message.actions.length > 0 ? (
                <div className="mt-2 rounded-lg border border-[var(--accent)]/40 bg-[var(--panel)] p-2">
                  <div className="mb-1 text-[10px] font-semibold uppercase tracking-[0.07em] text-[var(--accent)]">
                    {t("ai.proposed", { count: message.actions.length })}
                  </div>
                  <ul className="mb-2 space-y-1">
                    {message.actions.map((action, index) => (
                      <li key={index} className="text-[11px] text-[var(--muted)]">
                        • {describeAction(action, t, sceneTitle)}
                      </li>
                    ))}
                  </ul>
                  <Button
                    size="sm"
                    variant="primary"
                    disabled={message.applied}
                    onClick={() => void applyAiActions(message.id)}
                  >
                    {message.applied ? t("ai.appliedShort") : t("ai.apply")}
                  </Button>
                </div>
              ) : null}
            </div>
          </div>
        ))}

        {aiBusy ? (
          <div className="flex items-center gap-2 text-[11px] text-[var(--muted)]">
            <Spinner size={12} /> {t("ai.thinking")}
          </div>
        ) : null}
        {aiError && !aiBusy ? (
          <div className="rounded-lg border border-[var(--danger)]/40 bg-[var(--danger)]/10 px-2 py-1.5 text-[11px] leading-snug text-[var(--danger)]">
            {aiError}
          </div>
        ) : null}
      </div>

      <div className="flex-none border-t border-[var(--border)] p-2.5">
        <textarea
          className="textarea min-h-[64px] resize-none text-[12.5px]"
          placeholder={t("ai.placeholder")}
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              submit();
            }
          }}
        />
        <div className="mt-1.5 flex items-center justify-between">
          <span className="text-[10px] text-[var(--muted-2)]">{t("ai.hintSend")}</span>
          <Button variant="primary" size="sm" disabled={!canSend} onClick={submit}>
            <ArrowUp size={14} /> {t("ai.send")}
          </Button>
        </div>
      </div>
    </aside>
  );
}

function describeAction(
  action: AiAction,
  t: (key: string, options?: Record<string, unknown>) => string,
  sceneTitle: (sceneId: string | undefined) => string,
): string {
  switch (action.type) {
    case "add_scene":
      return t("ai.action.addScene", { title: action.title ?? "" });
    case "add_block":
      return t("ai.action.addBlock", {
        kind: action.kind ?? "",
        scene: sceneTitle(action.sceneId),
      });
    case "update_block":
      return t("ai.action.updateBlock", { scene: sceneTitle(action.sceneId) });
    case "delete_block":
      return t("ai.action.deleteBlock", { scene: sceneTitle(action.sceneId) });
    case "set_start_scene":
      return t("ai.action.setStart", { scene: sceneTitle(action.sceneId) });
    default:
      return action.type;
  }
}
