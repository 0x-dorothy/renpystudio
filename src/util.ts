/** Small client-side helpers. */

export function newId(): string {
  // UUIDv7-ish: time-ordered, good enough for local UI drafts.
  const now = Date.now().toString(16).padStart(12, "0");
  const rand = Array.from({ length: 20 }, () => Math.floor(Math.random() * 16).toString(16)).join("");
  return `${now.slice(0, 8)}-${now.slice(8, 12)}-7${rand.slice(0, 3)}-${rand.slice(3, 7)}-${rand.slice(7, 19)}`;
}

export function formatTime(iso: string | null | undefined): string {
  if (!iso) return "—";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

export function formatDateTime(iso: string | null | undefined): string {
  if (!iso) return "—";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleString(undefined, {
    day: "2-digit",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export function basename(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** Custom protocol URL for previewing project assets in the webview. */
export function assetUrl(path: string): string {
  const encoded = path
    .split("/")
    .map((part) => encodeURIComponent(part))
    .join("/");
  return `http://studio-asset.localhost/${encoded}`;
}

export function slugify(value: string): string {
  return value
    .toLowerCase()
    .replace(/[а-яё]/g, (char) => {
      const map: Record<string, string> = {
        а: "a", б: "b", в: "v", г: "g", д: "d", е: "e", ё: "e", ж: "zh", з: "z", и: "i",
        й: "y", к: "k", л: "l", м: "m", н: "n", о: "o", п: "p", р: "r", с: "s", т: "t",
        у: "u", ф: "f", х: "h", ц: "ts", ч: "ch", ш: "sh", щ: "sch", ъ: "", ы: "y", ь: "",
        э: "e", ю: "yu", я: "ya",
      };
      return map[char] ?? "";
    })
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}
