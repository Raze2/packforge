// Host integration for shells that provide native files (the desktop app).
// Without a platform, the designer uses browser downloads and a file picker.

import type { ExportFormat, Project } from './types';

export interface OpenedDoc {
  name: string;
  contents: string;
}

export interface Platform {
  /** Shows an open dialog; resolves null when cancelled. */
  open(): Promise<OpenedDoc | null>;
  /** Saves to the current file (or asks where); resolves the document name, null if cancelled. */
  save(contents: string, suggestedName: string, saveAs: boolean): Promise<string | null>;
  /** Exports with a save dialog; resolves false when cancelled. */
  exportFile(project: Project, format: ExportFormat, suggestedName: string): Promise<boolean>;
  newDocument(): void;
  /** Reports the document name and unsaved state (window title, close prompt). */
  setDocument(name: string | null, dirty: boolean): void;
}

/** Commands the host can send (menu items, shortcuts). */
export type DesignerCommand =
  | 'file.new'
  | 'file.open'
  | 'file.save'
  | 'file.save-as'
  | 'file.export-dxf'
  | 'file.export-svg'
  | 'view.fit';
