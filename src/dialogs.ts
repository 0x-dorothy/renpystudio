import { open } from "@tauri-apps/plugin-dialog";
import { api } from "./api";

export async function pickFolder(title?: string): Promise<string | null> {
  const result = await open({ directory: true, multiple: false, title });
  return typeof result === "string" ? result : null;
}

export async function pickFile(filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  const result = await open({ multiple: false, filters });
  return typeof result === "string" ? result : null;
}

export async function pickFiles(filters?: { name: string; extensions: string[] }[]): Promise<string[]> {
  const result = await open({ multiple: true, filters });
  if (Array.isArray(result)) return result;
  return typeof result === "string" ? [result] : [];
}

export async function revealProject() {
  const path = await api.projectPath();
  if (path) await api.revealInExplorer(path);
}
