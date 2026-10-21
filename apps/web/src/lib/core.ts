// Typed wrapper around the pf-core WebAssembly module.

import init, * as wasm from './wasm/pf_wasm.js';
import wasmUrl from './wasm/pf_wasm_bg.wasm?url';
import type {
  Board,
  ComponentInfo,
  ComponentSummary,
  ExportFormat,
  CaseCandidate,
  CaseRequest,
  ExportOptions,
  Folded,
  PalletRequest,
  PalletResult,
  Nesting,
  NestSummary,
  Project,
  RenderResult,
} from './types';

export class CoreError extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = 'CoreError';
  }
}

let ready: Promise<void> | null = null;

/** Loads the WASM core once; safe to call many times. */
export function loadCore(): Promise<void> {
  ready ??= init({ module_or_path: wasmUrl }).then(() => undefined);
  return ready;
}

function toCoreError(e: unknown): CoreError {
  if (typeof e === 'string') {
    try {
      const parsed = JSON.parse(e) as { code?: string; message?: string };
      return new CoreError(parsed.code ?? 'unknown', parsed.message ?? e);
    } catch {
      return new CoreError('unknown', e);
    }
  }
  if (e instanceof Error) return new CoreError('unknown', e.message);
  return new CoreError('unknown', String(e));
}

function call<T>(f: () => string): T {
  let out: string;
  try {
    out = f();
  } catch (e) {
    throw toCoreError(e);
  }
  return JSON.parse(out) as T;
}

export const core = {
  version: (): string => wasm.version(),
  listComponents: (): ComponentSummary[] => call(() => wasm.listComponents()),
  describe: (id: string): ComponentInfo => call(() => wasm.describe(id)),
  boards: (): Board[] => call(() => wasm.boards()),
  render: (project: Project): RenderResult => call(() => wasm.render(JSON.stringify(project))),
  fold: (project: Project, progress: number): Folded =>
    call(() => wasm.fold(JSON.stringify(project), progress)),
  palletPlan: (project: Project, req: PalletRequest): PalletResult =>
    call(() => wasm.palletPlan(JSON.stringify(project), JSON.stringify(req))),
  palletReport(project: Project, req: PalletRequest): Uint8Array {
    try {
      return wasm.palletReport(JSON.stringify(project), JSON.stringify(req));
    } catch (e) {
      throw toCoreError(e);
    }
  },
  caseOptimize: (req: CaseRequest): CaseCandidate[] =>
    call(() => wasm.caseOptimize(JSON.stringify(req))),
  nest: (project: Project, nesting: Nesting): NestSummary =>
    call(() => wasm.nest(JSON.stringify(project), JSON.stringify(nesting))),
  exportFile(project: Project, format: ExportFormat, options?: ExportOptions): Uint8Array {
    try {
      return wasm.exportFile(
        JSON.stringify(project),
        format,
        options ? JSON.stringify(options) : undefined,
      );
    } catch (e) {
      throw toCoreError(e);
    }
  },
};
