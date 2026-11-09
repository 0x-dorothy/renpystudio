import { useEffect, useRef, useState } from "react";

/**
 * Local draft with debounced persistence. Resets when `key` changes so switching
 * selection never leaks values between objects.
 */
export function useDebouncedValue<T>(
  key: string,
  value: T,
  onSave: (value: T) => void,
  delay = 450,
) {
  const [draft, setDraft] = useState<T>(value);
  const dirty = useRef(false);
  const timer = useRef<number | null>(null);
  const latest = useRef<T>(draft);
  latest.current = draft;
  const saveRef = useRef(onSave);
  saveRef.current = onSave;

  useEffect(() => {
    setDraft(value);
    dirty.current = false;
    if (timer.current) window.clearTimeout(timer.current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  useEffect(() => {
    if (!dirty.current) setDraft(value);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [JSON.stringify(value)]);

  useEffect(() => {
    return () => {
      if (timer.current) {
        window.clearTimeout(timer.current);
        if (dirty.current) saveRef.current(latest.current);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  const update = (next: T | ((prev: T) => T)) => {
    const resolved = typeof next === "function" ? (next as (prev: T) => T)(latest.current) : next;
    setDraft(resolved);
    latest.current = resolved;
    dirty.current = true;
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      timer.current = null;
      dirty.current = false;
      saveRef.current(resolved);
    }, delay);
  };

  return [draft, update] as const;
}
