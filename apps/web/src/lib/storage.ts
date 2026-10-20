// Per-browser conveniences. Every access is guarded: storage can be blocked or unavailable.

import type { Project } from './types';

const AUTOSAVE_KEY = 'packforge:autosave:v1';
const PREFS_KEY = 'packforge:prefs:v1';

export interface Prefs {
  units?: string;
  locale?: string;
  theme?: string;
}

function read<T>(key: string): T | null {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : null;
  } catch {
    return null;
  }
}

function write(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage full or blocked: autosave is best-effort.
  }
}

export const loadAutosave = (): Project | null => read<Project>(AUTOSAVE_KEY);
export const saveAutosave = (p: Project): void => write(AUTOSAVE_KEY, p);
export const loadPrefs = (): Prefs => read<Prefs>(PREFS_KEY) ?? {};
export const savePrefs = (p: Prefs): void => write(PREFS_KEY, p);

export function download(bytes: Uint8Array | string, filename: string, mime: string): void {
  const blob = new Blob([bytes as BlobPart], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/** Validates the shape of a project loaded from a file. */
export function parseProject(text: string): Project {
  const p = JSON.parse(text) as Partial<Project>;
  if (typeof p.component_id !== 'string' || typeof p.params !== 'object' || p.board == null) {
    throw new Error('not a PackForge project file');
  }
  return {
    component_id: p.component_id,
    component_version: p.component_version,
    params: p.params ?? {},
    board: p.board,
    units: p.units ?? 'mm',
  };
}
