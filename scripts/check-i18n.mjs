// Reports i18n keys used in the UI but missing from the Russian dictionary.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, extname } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const ruSource = readFileSync(join(root, "src", "locales", "ru.ts"), "utf8");
const known = new Set([...ruSource.matchAll(/"([a-zA-Z0-9_.]+)":/g)].map((m) => m[1]));

const files = [];
function walk(dir) {
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) walk(path);
    else if ([".ts", ".tsx"].includes(extname(path))) files.push(path);
  }
}
walk(join(root, "src"));

const missing = new Map();
for (const file of files) {
  if (file.includes("locales")) continue;
  const text = readFileSync(file, "utf8");
  for (const match of text.matchAll(/\bt\(\s*"([a-zA-Z0-9_.]+)"/g)) {
    const key = match[1];
    if (!known.has(key) && !key.startsWith("palette.cat.")) {
      missing.set(key, file.replace(root, ""));
    }
  }
  for (const match of text.matchAll(/labelKey:\s*"([a-zA-Z0-9_.]+)"/g)) {
    if (!known.has(match[1])) missing.set(match[1], file.replace(root, ""));
  }
}

if (missing.size === 0) {
  console.log("i18n: all keys present");
} else {
  console.log(`i18n: ${missing.size} missing keys`);
  for (const [key, file] of [...missing].sort()) console.log(`  ${key}  (${file})`);
  process.exitCode = 1;
}
