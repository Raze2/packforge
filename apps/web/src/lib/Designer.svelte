<script lang="ts">
  import { onMount } from 'svelte';
  import { core, CoreError, loadCore } from './core';
  import { dirOf, isLocale, LOCALES, translate, type Locale } from './i18n';
  import {
    download,
    loadAutosave,
    loadPrefs,
    parseProject,
    saveAutosave,
    savePrefs,
  } from './storage';
  import {
    LINE_TYPES,
    type Board,
    type BoardRef,
    type ComponentInfo,
    type ComponentSummary,
    type ExportFormat,
    type LineType,
    type ParamValue,
    type Project,
    type RenderResult,
    type Units,
  } from './types';
  import { UNITS } from './units';
  import DielineView from './components/DielineView.svelte';
  import Library from './components/Library.svelte';
  import ParamForm from './components/ParamForm.svelte';
  import StatsBar from './components/StatsBar.svelte';

  export type Theme = 'auto' | 'light' | 'dark';
  export type Mode = 'full' | 'configurator';

  export interface ChangeDetail {
    project: Project;
    result: RenderResult | null;
    error: CoreError | null;
  }
  export interface ExportDetail {
    project: Project;
    format: ExportFormat;
    filename: string;
    mime: string;
    bytes: Uint8Array;
  }

  interface Props {
    component?: string;
    params?: Record<string, ParamValue>;
    board?: BoardRef;
    units?: Units;
    locale?: Locale;
    theme?: Theme;
    mode?: Mode;
    /** Remember the last design and preferences in this browser. */
    persist?: boolean;
    onready?: () => void;
    onchange?: (d: ChangeDetail) => void;
    /** Return true when the save was handled (skips the file download). */
    onsave?: (project: Project) => boolean | void;
    /** Return true when the export was handled (skips the file download). */
    onexport?: (d: ExportDetail) => boolean | void;
    onerror?: (e: CoreError) => void;
    /** Receives an API object for programmatic control (used by the web component). */
    bindApi?: (api: DesignerApi) => void;
  }

  export interface DesignerApi {
    getProject: () => Project;
    setProject: (p: Project) => void;
    exportFile: (format: ExportFormat) => ExportDetail | null;
  }

  let {
    component = 'fefco-0201',
    params = {},
    board = 'C-flute',
    units: unitsProp = 'mm',
    locale: localeProp = 'en',
    theme: themeProp = 'auto',
    mode = 'full',
    persist = false,
    onready,
    onchange,
    onsave,
    onexport,
    onerror,
    bindApi,
  }: Props = $props();

  // ── State ────────────────────────────────────────────────────────────────
  let ready = $state(false);
  let loadError = $state<string | null>(null);
  let components = $state<ComponentSummary[]>([]);
  let boards = $state<Board[]>([]);
  // Props seed the initial state; later changes go through setProject() or the UI.
  // svelte-ignore state_referenced_locally
  let project = $state<Project>({
    component_id: component,
    params: { ...params },
    board,
    units: unitsProp,
  });
  // svelte-ignore state_referenced_locally
  let locale = $state<Locale>(localeProp);
  // svelte-ignore state_referenced_locally
  let theme = $state<Theme>(themeProp);
  let hidden = $state(new Set<LineType>());
  let libraryOpen = $state(false);
  let prefersDark = $state(false);
  let fileInput: HTMLInputElement | undefined = $state();
  let openError = $state<string | null>(null);

  const t = (key: string, vars?: Record<string, string | number>, fallback?: string) =>
    translate(locale, key, vars, fallback);
  const dir = $derived(dirOf(locale));
  const resolvedTheme = $derived(theme === 'auto' ? (prefersDark ? 'dark' : 'light') : theme);

  const info: ComponentInfo | null = $derived.by(() => {
    if (!ready) return null;
    try {
      return core.describe(project.component_id);
    } catch {
      return null;
    }
  });

  const rendered = $derived.by(() => {
    if (!ready) return { result: null, error: null, ms: null };
    const t0 = performance.now();
    try {
      const result = core.render(project);
      return { result, error: null, ms: performance.now() - t0 };
    } catch (e) {
      const error = e instanceof CoreError ? e : new CoreError('unknown', String(e));
      return { result: null as RenderResult | null, error, ms: null };
    }
  });

  // Keep the last good drawing on screen while the user is mid-edit.
  let lastGood = $state<RenderResult | null>(null);
  $effect(() => {
    if (rendered.result) lastGood = rendered.result;
  });
  const shown = $derived(rendered.result ?? lastGood);
  const fitKey = $derived(
    `${project.component_id}|${typeof project.board === 'string' ? project.board : 'custom'}`,
  );

  // ── Lifecycle ────────────────────────────────────────────────────────────
  onMount(() => {
    const mq = window.matchMedia?.('(prefers-color-scheme: dark)');
    prefersDark = mq?.matches ?? false;
    const onMq = (e: MediaQueryListEvent) => (prefersDark = e.matches);
    mq?.addEventListener?.('change', onMq);

    if (persist) {
      const prefs = loadPrefs();
      if (isLocale(prefs.locale)) locale = prefs.locale;
      if (prefs.theme === 'light' || prefs.theme === 'dark' || prefs.theme === 'auto')
        theme = prefs.theme;
      const saved = loadAutosave();
      if (saved) project = saved;
    }

    loadCore()
      .then(() => {
        components = core.listComponents();
        boards = core.boards();
        ready = true;
        onready?.();
      })
      .catch((e: unknown) => {
        loadError = t('error.load');
        console.error(e);
      });

    bindApi?.({
      getProject: () => $state.snapshot(project) as Project,
      setProject: (p) => (project = { ...p, params: { ...p.params } }),
      exportFile: (f) => buildExport(f),
    });

    return () => mq?.removeEventListener?.('change', onMq);
  });

  $effect(() => {
    if (!ready) return;
    const snap = $state.snapshot(project) as Project;
    if (persist) saveAutosave(snap);
    onchange?.({ project: snap, result: rendered.result, error: rendered.error });
    if (rendered.error) onerror?.(rendered.error);
  });

  $effect(() => {
    if (persist) savePrefs({ units: project.units, locale, theme });
  });

  // ── Actions ──────────────────────────────────────────────────────────────
  function setParam(id: string, value: ParamValue) {
    project.params = { ...project.params, [id]: value };
  }

  function selectComponent(id: string) {
    if (id !== project.component_id) {
      project = { ...project, component_id: id, params: {} };
    }
    libraryOpen = false;
  }

  function resetParams() {
    project.params = {};
  }

  function newDesign() {
    project = { component_id: component, params: {}, board, units: project.units };
  }

  function save() {
    const snap = $state.snapshot(project) as Project;
    snap.component_version = shown?.component_version;
    if (onsave?.(snap) === true) return;
    download(JSON.stringify(snap, null, 2), `${snap.component_id}.pfproj.json`, 'application/json');
  }

  async function openFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    try {
      project = parseProject(await file.text());
      openError = null;
    } catch (err) {
      openError = t('error.open', { message: err instanceof Error ? err.message : String(err) });
    }
  }

  function buildExport(format: ExportFormat): ExportDetail | null {
    const snap = $state.snapshot(project) as Project;
    try {
      const bytes = core.exportFile(snap, format);
      return {
        project: snap,
        format,
        bytes,
        filename: `${snap.component_id}.${format}`,
        mime: format === 'svg' ? 'image/svg+xml' : 'application/dxf',
      };
    } catch (e) {
      if (e instanceof CoreError) onerror?.(e);
      return null;
    }
  }

  function exportAs(format: ExportFormat) {
    const d = buildExport(format);
    if (!d) return;
    if (onexport?.(d) === true) return;
    download(d.bytes, d.filename, d.mime);
  }

  function toggleLine(lt: LineType) {
    const next = new Set(hidden);
    if (next.has(lt)) next.delete(lt);
    else next.add(lt);
    hidden = next;
  }

  const presentLines = $derived(
    shown ? LINE_TYPES.filter((lt) => shown.dieline.entities.some((e) => e.line_type === lt)) : [],
  );
</script>

<div class="pf-root" data-theme={resolvedTheme} {dir} lang={locale} class:configurator={mode === 'configurator'}>
  <header class="top">
    <div class="brand">
      {#if mode === 'full'}
        <button
          type="button"
          class="icon only-narrow"
          aria-label={t('action.library')}
          aria-expanded={libraryOpen}
          onclick={() => (libraryOpen = !libraryOpen)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16" /></svg>
        </button>
      {/if}
      <svg class="logo" viewBox="0 0 32 32" aria-hidden="true">
        <path d="M6 11 16 6l10 5v10l-10 5-10-5z" />
        <path d="M6 11l10 5 10-5M16 16v10" />
      </svg>
      <div class="title">
        <strong>{t('app.title')}</strong>
        <span>{t('app.tagline')}</span>
      </div>
    </div>

    <div class="actions">
      {#if mode === 'full'}
        <button type="button" class="ghost" onclick={newDesign}>{t('action.new')}</button>
        <button type="button" class="ghost" onclick={() => fileInput?.click()}>{t('action.open')}</button>
        <input
          bind:this={fileInput}
          type="file"
          accept=".json,application/json"
          hidden
          onchange={openFile}
        />
      {/if}
      <button type="button" class="ghost" onclick={save} disabled={!ready}>{t('action.save')}</button>
      <button type="button" class="primary" onclick={() => exportAs('dxf')} disabled={!rendered.result}>
        {t('action.export_dxf')}
      </button>
      <button type="button" class="ghost" onclick={() => exportAs('svg')} disabled={!rendered.result}>
        {t('action.export_svg')}
      </button>
    </div>

    <div class="prefs">
      <label>
        <span class="sr">{t('pref.units')}</span>
        <select bind:value={project.units} aria-label={t('pref.units')}>
          {#each UNITS as u (u)}<option value={u}>{u}</option>{/each}
        </select>
      </label>
      <label>
        <span class="sr">{t('pref.language')}</span>
        <select bind:value={locale} aria-label={t('pref.language')}>
          {#each LOCALES as l (l.id)}<option value={l.id}>{l.name}</option>{/each}
        </select>
      </label>
      <label>
        <span class="sr">{t('pref.theme')}</span>
        <select bind:value={theme} aria-label={t('pref.theme')}>
          <option value="auto">{t('theme.auto')}</option>
          <option value="light">{t('theme.light')}</option>
          <option value="dark">{t('theme.dark')}</option>
        </select>
      </label>
    </div>
  </header>

  {#if openError}
    <p class="banner error" role="alert">{openError}</p>
  {/if}

  <div class="body">
    {#if mode === 'full'}
      <aside class="library" class:open={libraryOpen} aria-label={t('library.title')}>
        <h2>{t('library.title')}</h2>
        {#if ready}
          <Library {components} selected={project.component_id} {t} onselect={selectComponent} />
        {/if}
      </aside>
    {/if}

    <main class="canvas">
      {#if loadError}
        <div class="state error">{loadError}</div>
      {:else if !ready}
        <div class="state">{t('loading')}</div>
      {:else}
        <DielineView
          result={shown}
          {hidden}
          units={project.units}
          {locale}
          {fitKey}
          labels={{ fit: t('action.fit'), zoomIn: t('action.zoom_in'), zoomOut: t('action.zoom_out') }}
        />
        {#if rendered.error}
          <div class="banner error floating" role="alert">
            <strong>{t('error.title')}</strong>
            <span>{rendered.error.message}</span>
          </div>
        {/if}
        {#if presentLines.length}
          <div class="legend" role="group" aria-label="Line types">
            {#each presentLines as lt (lt)}
              <button
                type="button"
                class="chip"
                class:off={hidden.has(lt)}
                aria-pressed={!hidden.has(lt)}
                onclick={() => toggleLine(lt)}
              >
                <span class="swatch {lt}"></span>{t(`line.${lt}`)}
              </button>
            {/each}
          </div>
        {/if}
      {/if}
    </main>

    <aside class="params" aria-label={t('params.title')}>
      {#if info}
        <div class="params-head">
          <h2>{t(`component.${info.id}`, undefined, info.name)}</h2>
          <span>{info.standard ?? info.id} · v{info.version}</span>
        </div>
        <ParamForm
          {info}
          params={project.params}
          board={project.board}
          {boards}
          units={project.units}
          {t}
          onparam={setParam}
          onboard={(b) => (project.board = b)}
          onreset={resetParams}
        />
      {/if}
    </aside>
  </div>

  <footer class="stats">
    <StatsBar
      stats={rendered.result?.stats ?? null}
      diagnostics={rendered.result?.diagnostics ?? []}
      units={project.units}
      {locale}
      renderMs={rendered.ms}
      {t}
    />
  </footer>
</div>

<style>
  .pf-root {
    --pf-font: system-ui, -apple-system, 'Segoe UI', Roboto, 'Noto Sans', 'Noto Sans Arabic', sans-serif;
    --pf-radius: 14px;
    --pf-bg: #f4f5f7;
    --pf-surface: #ffffff;
    --pf-canvas: #fbfbfc;
    --pf-grid: #dfe2e7;
    --pf-text: #17181c;
    --pf-muted: #6b6f78;
    --pf-border: #dcdfe4;
    --pf-hover: #eef0f3;
    --pf-input: #ffffff;
    --pf-accent: #3a5bd9;
    --pf-accent-text: #ffffff;
    --pf-accent-soft: rgba(58, 91, 217, 0.12);
    --pf-danger: #c62828;
    --pf-danger-soft: rgba(198, 40, 40, 0.1);
    --pf-warning: #8a5a00;
    --pf-warning-soft: rgba(230, 160, 0, 0.16);
    --pf-premium: #8a4b00;
    --pf-premium-soft: rgba(230, 140, 0, 0.16);
    --pf-shadow: 0 1px 2px rgba(16, 24, 40, 0.06), 0 1px 3px rgba(16, 24, 40, 0.08);
    --pf-line-cut: #17181c;
    --pf-line-crease: #d0021b;
    --pf-line-reverse: #9b1bd0;
    --pf-line-perf: #1a8f3c;
    --pf-line-half: #1f5fd0;
    --pf-line-bleed: #00a3c4;

    font-family: var(--pf-font);
    color: var(--pf-text);
    background: var(--pf-bg);
    /* Layout responds to the designer's own width, so it adapts inside narrow page columns. */
    container: pf / inline-size;
    display: grid;
    grid-template-rows: auto auto 1fr auto;
    height: 100%;
    min-height: 520px;
    box-sizing: border-box;
    font-size: 14px;
    line-height: 1.4;
  }
  .pf-root[data-theme='dark'] {
    --pf-bg: #0f1115;
    --pf-surface: #171a20;
    --pf-canvas: #12151a;
    --pf-grid: #262a33;
    --pf-text: #eceef2;
    --pf-muted: #9aa0ab;
    --pf-border: #2c313b;
    --pf-hover: #222630;
    --pf-input: #12151a;
    --pf-accent: #7d97ff;
    --pf-accent-text: #0f1115;
    --pf-accent-soft: rgba(125, 151, 255, 0.16);
    --pf-danger: #ff7b7b;
    --pf-danger-soft: rgba(255, 123, 123, 0.14);
    --pf-warning: #ffcc66;
    --pf-warning-soft: rgba(255, 204, 102, 0.14);
    --pf-premium: #ffb35c;
    --pf-premium-soft: rgba(255, 179, 92, 0.14);
    --pf-shadow: 0 1px 2px rgba(0, 0, 0, 0.4);
    --pf-line-cut: #eceef2;
    --pf-line-crease: #ff6b7a;
    --pf-line-reverse: #d38cff;
    --pf-line-perf: #5fd18a;
    --pf-line-half: #7aa7ff;
    --pf-line-bleed: #49d3ee;
  }
  .pf-root :global(*:focus-visible) {
    outline: 2px solid var(--pf-accent);
    outline-offset: 2px;
  }

  /* Header */
  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 16px;
    padding: 10px 16px;
    background: var(--pf-surface);
    border-bottom: 1px solid var(--pf-border);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-inline-end: auto;
  }
  .logo {
    width: 30px;
    height: 30px;
    fill: var(--pf-accent-soft);
    stroke: var(--pf-accent);
    stroke-width: 1.8;
    stroke-linejoin: round;
  }
  .title {
    display: flex;
    flex-direction: column;
    line-height: 1.15;
  }
  .title strong {
    font-size: 15px;
  }
  .title span {
    font-size: 12px;
    color: var(--pf-muted);
  }
  .actions,
  .prefs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  button.primary,
  button.ghost {
    height: 34px;
    padding-inline: 14px;
    border-radius: 9px;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  button.primary {
    border: 1px solid var(--pf-accent);
    background: var(--pf-accent);
    color: var(--pf-accent-text);
  }
  button.primary:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  button.ghost {
    border: 1px solid var(--pf-border);
    background: var(--pf-surface);
    color: var(--pf-text);
  }
  button.ghost:hover:not(:disabled) {
    background: var(--pf-hover);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .prefs select {
    height: 34px;
    padding-inline: 8px;
    border: 1px solid var(--pf-border);
    border-radius: 9px;
    background: var(--pf-surface);
    color: var(--pf-text);
    font: inherit;
    font-size: 13px;
  }
  .icon {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    border: 1px solid var(--pf-border);
    border-radius: 9px;
    background: var(--pf-surface);
    color: var(--pf-text);
    cursor: pointer;
  }
  .icon svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
  .only-narrow {
    display: none;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  /* Body */
  .body {
    display: grid;
    grid-template-columns: 260px minmax(0, 1fr) 300px;
    gap: 12px;
    padding: 12px;
    min-height: 0;
  }
  .configurator .body {
    grid-template-columns: minmax(0, 1fr) 300px;
  }
  .library,
  .params {
    background: var(--pf-surface);
    border: 1px solid var(--pf-border);
    border-radius: var(--pf-radius);
    padding: 14px;
    overflow: auto;
    min-height: 0;
  }
  h2 {
    margin: 0 0 12px;
    font-size: 15px;
  }
  .params-head {
    margin-block-end: 16px;
  }
  .params-head h2 {
    margin-block-end: 2px;
  }
  .params-head span {
    font-size: 12px;
    color: var(--pf-muted);
  }
  .canvas {
    position: relative;
    min-height: 0;
    border: 1px solid var(--pf-border);
    border-radius: var(--pf-radius);
    display: flex;
  }
  .state {
    margin: auto;
    color: var(--pf-muted);
  }
  .state.error {
    color: var(--pf-danger);
  }
  .legend {
    position: absolute;
    inset-block-start: 12px;
    inset-inline-start: 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    max-width: calc(100% - 24px);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding-inline: 10px;
    border: 1px solid var(--pf-border);
    border-radius: 99px;
    background: var(--pf-surface);
    color: var(--pf-text);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    box-shadow: var(--pf-shadow);
  }
  .chip.off {
    opacity: 0.45;
  }
  .swatch {
    width: 16px;
    height: 0;
    border-top: 2px solid var(--pf-line-cut);
  }
  .swatch.crease {
    border-top: 2px dashed var(--pf-line-crease);
  }
  .swatch.reverse-crease {
    border-top: 2px dashed var(--pf-line-reverse);
  }
  .swatch.perforation {
    border-top: 2px dotted var(--pf-line-perf);
  }
  .swatch.half-cut {
    border-top: 2px dashed var(--pf-line-half);
  }
  .swatch.bleed {
    border-top: 2px dotted var(--pf-line-bleed);
  }
  .banner {
    margin: 12px 12px 0;
    padding: 10px 14px;
    border-radius: 10px;
    font-size: 13px;
  }
  .banner.error {
    background: var(--pf-danger-soft);
    color: var(--pf-danger);
  }
  .banner.floating {
    position: absolute;
    inset-block-end: 12px;
    inset-inline-start: 12px;
    margin: 0;
    max-width: min(520px, calc(100% - 80px));
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: var(--pf-surface);
    border: 1px solid var(--pf-danger);
    box-shadow: var(--pf-shadow);
  }
  .stats {
    padding: 10px 16px;
    background: var(--pf-surface);
    border-top: 1px solid var(--pf-border);
  }

  /* Narrow designer: canvas on top, parameters below, library as a drawer. */
  @container pf (max-width: 820px) {
    .body,
    .configurator .body {
      grid-template-columns: minmax(0, 1fr);
      overflow: auto;
    }
    .canvas {
      min-height: 320px;
      height: min(56vh, 520px);
    }
    .only-narrow {
      display: grid;
    }
    .library {
      display: none;
    }
    .library.open {
      display: block;
    }
  }
</style>
