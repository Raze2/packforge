// Desktop (Tauri) integration: native dialogs and files, menu commands, files opened from the OS.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { DesignerApi } from './lib/Designer.svelte';
import type { OpenedDoc, Platform } from './lib/platform';

export const isDesktop = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const desktopPlatform: Platform = {
  open: () => invoke<OpenedDoc | null>('open_project', { path: null }),
  async save(contents, suggestedName, saveAs) {
    const saved = await invoke<{ name: string } | null>('save_project', {
      contents,
      saveAs,
      suggestedName,
    });
    return saved?.name ?? null;
  },
  async exportFile(project, format, suggestedName) {
    const path = await invoke<string | null>('export_file', { project, format, suggestedName });
    return path !== null;
  },
  newDocument: () => void invoke('new_document'),
  setDocument: (name, dirty) => void invoke('set_document', { name, dirty }),
};

/** Wires menu events and OS-opened files to the designer. Call once the designer is ready. */
export async function connectDesktop(api: DesignerApi): Promise<void> {
  await listen<string>('menu', (e) => api.command(e.payload));
  await listen<OpenedDoc>('open-file', (e) => api.openText(e.payload.name, e.payload.contents));
  await listen<string>('error', (e) => api.notify(e.payload));
  const startup = await invoke<OpenedDoc | null>('take_startup_file');
  if (startup) api.openText(startup.name, startup.contents);
}
