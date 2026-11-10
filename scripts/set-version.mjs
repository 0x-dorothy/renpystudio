// Single source of truth for the app version: one command updates every copy.
// Usage: node scripts/set-version.mjs 0.1.1  (also runs as an npm `version` hook)
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const here = path.dirname(fileURLToPath(import.meta.url)); // studio/scripts
const root = path.dirname(here); // studio/

const version = process.argv[2] || process.env.npm_package_new_version;
if (!version || !/^\d+\.\d+\.\d+(-[\w.]+)?$/.test(version)) {
  console.error("Usage: node scripts/set-version.mjs <x.y.z[-suffix]>");
  process.exit(1);
}

const files = {
  "package.json": (text) => {
    const json = JSON.parse(text);
    if (json.version === version) return null;
    json.version = version;
    return `${JSON.stringify(json, null, 2)}\n`;
  },
  "src-tauri/tauri.conf.json": (text) => {
    const json = JSON.parse(text);
    if (json.version === version) return null;
    json.version = version;
    return `${JSON.stringify(json, null, 2)}\n`;
  },
  "src-tauri/Cargo.toml": (text) => {
    // Anchored to a line start: only the [package] version, never dependencies.
    const next = text.replace(/^version\s*=\s*"[^"]*"/m, `version = "${version}"`);
    return next === text ? null : next;
  },
  "src/version.ts": (text) => {
    const next = text.replace(/export const API_VERSION = "[^"]*";/, `export const API_VERSION = "${version}";`);
    return next === text ? null : next;
  },
};

let changed = 0;
for (const [file, transform] of Object.entries(files)) {
  const full = path.join(root, file);
  const before = readFileSync(full, "utf8");
  const after = transform(before);
  if (after === null) {
    console.log(`  ok    ${file} (already ${version})`);
  } else {
    writeFileSync(full, after, "utf8");
    console.log(`  set   ${file} -> ${version}`);
    changed += 1;
  }
}
console.log(`Version ${version}: ${changed} file(s) updated.`);
