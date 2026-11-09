import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Camera, RotateCcw } from "lucide-react";
import { useStudio } from "../store";
import { Button, Input, Modal, Spinner } from "./ui";
import { formatDateTime } from "../util";
import type { DiffEntry } from "../types";

export function HistoryModal() {
  const { t } = useTranslation();
  const open = useStudio((s) => s.modal) === "history";
  const close = useStudio((s) => s.closeModal);
  const snapshots = useStudio((s) => s.snapshots);
  const refreshSnapshots = useStudio((s) => s.refreshSnapshots);
  const takeSnapshot = useStudio((s) => s.takeSnapshot);
  const restoreSnapshot = useStudio((s) => s.restoreSnapshot);
  const diffSnapshot = useStudio((s) => s.diffSnapshot);
  const busy = useStudio((s) => s.busy);
  const [selected, setSelected] = useState<string | null>(null);
  const [diff, setDiff] = useState<DiffEntry[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [label, setLabel] = useState("");

  useEffect(() => {
    if (open) {
      void refreshSnapshots();
      setSelected(null);
      setDiff(null);
    }
  }, [open, refreshSnapshots]);

  useEffect(() => {
    if (!selected) {
      setDiff(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    void diffSnapshot(selected).then((entries) => {
      if (!cancelled) {
        setDiff(entries);
        setLoading(false);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [selected, diffSnapshot]);

  return (
    <Modal
      open={open}
      onClose={close}
      title={t("history.title")}
      width={900}
      footer={
        <Button variant="ghost" onClick={close}>
          {t("common.close")}
        </Button>
      }
    >
      <div className="mb-4 flex items-center gap-2">
        <Input
          placeholder={t("history.snapshotLabel")}
          value={label}
          onChange={(event) => setLabel(event.target.value)}
          className="max-w-[280px]"
        />
        <Button
          size="sm"
          disabled={busy}
          onClick={() => {
            void takeSnapshot(label || t("history.snapshot"));
            setLabel("");
          }}
        >
          {busy ? <Spinner /> : <Camera size={13} />} {t("history.snapshot")}
        </Button>
      </div>

      {snapshots.length === 0 ? (
        <div className="rounded-xl border border-dashed border-[var(--border)] px-4 py-8 text-center text-[12px] text-[var(--muted-2)]">
          {t("history.empty")}
        </div>
      ) : (
        <div className="grid grid-cols-[320px_minmax(0,1fr)] gap-4">
          <div className="scroll max-h-[440px] rounded-xl border border-[var(--border)]">
            {snapshots.map((snapshot) => (
              <button
                key={snapshot.file}
                type="button"
                onClick={() => setSelected(snapshot.file)}
                className={`flex w-full flex-col gap-0.5 border-b border-[var(--border-soft)] px-3 py-2 text-left last:border-b-0 ${
                  selected === snapshot.file ? "bg-[var(--accent-soft)]" : "hover:bg-[var(--panel-2)]"
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="truncate text-[12px] font-medium">
                    {snapshot.label || t(`history.kind.${snapshot.kind}`)}
                  </span>
                  <span className="chip">{t(`history.kind.${snapshot.kind}`)}</span>
                </div>
                <div className="flex items-center justify-between text-[10.5px] text-[var(--muted-2)]">
                  <span>{formatDateTime(snapshot.createdAt)}</span>
                  <span>
                    {snapshot.scenes} сцен · {snapshot.blocks} блоков
                  </span>
                </div>
              </button>
            ))}
          </div>

          <div className="flex min-h-[200px] flex-col rounded-xl border border-[var(--border)] bg-[var(--panel-2)] p-3">
            {!selected ? (
              <div className="flex flex-1 items-center justify-center text-[12px] text-[var(--muted-2)]">
                {t("history.diff")}
              </div>
            ) : loading ? (
              <div className="flex flex-1 items-center justify-center gap-2 text-[12px] text-[var(--muted)]">
                <Spinner /> {t("app.loading")}
              </div>
            ) : !diff || diff.length === 0 ? (
              <div className="flex flex-1 items-center justify-center text-[12px] text-[var(--muted-2)]">
                {t("history.diffEmpty")}
              </div>
            ) : (
              <>
                <div className="scroll max-h-[380px] flex-1">
                  {diff.map((entry, index) => (
                    <div key={index} className="mb-2 rounded-lg border border-[var(--border)] bg-[var(--panel)] px-2.5 py-2">
                      <div className="text-[12px] font-medium">{entry.summary}</div>
                      {entry.details.length > 0 ? (
                        <ul className="mt-1 space-y-0.5">
                          {entry.details.slice(0, 8).map((detail, detailIndex) => (
                            <li key={detailIndex} className="truncate text-[11px] text-[var(--muted)]">
                              {detail}
                            </li>
                          ))}
                        </ul>
                      ) : null}
                    </div>
                  ))}
                </div>
                <Button
                  variant="primary"
                  size="sm"
                  className="mt-2 self-end"
                  onClick={() => {
                    if (selected && window.confirm(t("history.restoreConfirm", { label: selected }))) {
                      void restoreSnapshot(selected);
                      close();
                    }
                  }}
                >
                  <RotateCcw size={13} /> {t("history.restore")}
                </Button>
              </>
            )}
          </div>
        </div>
      )}
    </Modal>
  );
}
