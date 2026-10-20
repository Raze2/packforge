<script lang="ts">
  import { core, CoreError } from '../core';
  import type { ExportFormat, ExportOptions, Nesting, NestSummary, Project } from '../types';

  interface Props {
    open: boolean;
    project: Project;
    t: (key: string, vars?: Record<string, string | number>) => string;
    onclose: () => void;
    onexport: (format: ExportFormat, options: ExportOptions) => void;
  }

  let { open, project, t, onclose, onexport }: Props = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let format = $state<ExportFormat>('pdf');
  let rotate = $state<0 | 90 | 180 | 270>(0);
  let mirror = $state(false);
  let page = $state<'actual' | 'a4' | 'a3' | 'letter'>('actual');
  let profile = $state<'standard' | 'cut-crease'>('standard');
  let nestOn = $state(false);
  let sheetW = $state(2400);
  let sheetH = $state(1600);
  let gap = $state(8);
  let margin = $state(10);
  let allowRotation = $state(true);

  const FORMATS: ExportFormat[] = ['pdf', 'dxf', 'cf2', 'hpgl', 'svg', 'glb'];
  const is3d = $derived(format === 'glb');
  const machine = $derived(format === 'dxf' || format === 'cf2' || format === 'hpgl');

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });

  const nesting = $derived<Nesting | null>(
    nestOn && !is3d
      ? { sheet_width: sheetW, sheet_height: sheetH, gap, margin, allow_rotation: allowRotation }
      : null,
  );

  const preview = $derived.by((): { ok: NestSummary } | { error: string } | null => {
    if (!nesting || !open) return null;
    try {
      return { ok: core.nest($state.snapshot(project) as Project, nesting) };
    } catch (e) {
      return { error: e instanceof CoreError ? e.message : String(e) };
    }
  });

  function options(): ExportOptions {
    if (is3d) return {};
    return {
      rotate,
      mirror,
      nesting,
      page: format === 'pdf' ? page : null,
      profile: machine ? profile : null,
    };
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    onexport(format, options());
  }

  const num = (v: string, fallback: number) => {
    const n = Number(v.replace(',', '.'));
    return Number.isFinite(n) && n >= 0 ? n : fallback;
  };
</script>

<dialog bind:this={dialog} onclose={onclose} aria-labelledby="pf-export-title">
  <form method="dialog" onsubmit={submit}>
    <header>
      <h2 id="pf-export-title">{t('export.title')}</h2>
      <button type="button" class="x" aria-label={t('export.cancel')} onclick={onclose}>×</button>
    </header>

    <fieldset class="formats">
      <legend>{t('export.format')}</legend>
      {#each FORMATS as f (f)}
        <label class="card" class:active={format === f}>
          <input type="radio" name="format" value={f} bind:group={format} />
          <strong>{f === 'hpgl' ? 'PLT' : f.toUpperCase()}</strong>
          <span>{t(`export.format.${f}`)}</span>
        </label>
      {/each}
    </fieldset>

    {#if !is3d}
      <fieldset>
        <legend>{t('export.layout')}</legend>
        <div class="row">
          <span class="label">{t('export.rotate')}</span>
          <div class="segmented" role="radiogroup" aria-label={t('export.rotate')}>
            {#each [0, 90, 180, 270] as r (r)}
              <button
                type="button"
                role="radio"
                aria-checked={rotate === r}
                class:active={rotate === r}
                onclick={() => (rotate = r as 0 | 90 | 180 | 270)}>{r}°</button
              >
            {/each}
          </div>
        </div>
        <label class="check"><input type="checkbox" bind:checked={mirror} /> {t('export.mirror')}</label>
        {#if format === 'pdf'}
          <div class="row">
            <label class="label" for="pf-page">{t('export.page')}</label>
            <select id="pf-page" bind:value={page}>
              <option value="actual">{t('export.page.actual')}</option>
              <option value="a4">A4</option>
              <option value="a3">A3</option>
              <option value="letter">Letter</option>
            </select>
          </div>
        {/if}
        {#if machine}
          <div class="row">
            <label class="label" for="pf-profile">{t('export.profile')}</label>
            <select id="pf-profile" bind:value={profile}>
              <option value="standard">{t('export.profile.standard')}</option>
              <option value="cut-crease">{t('export.profile.cut-crease')}</option>
            </select>
          </div>
        {/if}
      </fieldset>

      <fieldset>
        <legend>{t('export.nesting')}</legend>
        <label class="check"><input type="checkbox" bind:checked={nestOn} /> {t('export.nesting.on')}</label>
        {#if nestOn}
          <div class="grid">
            <label>{t('export.sheet_w')}<input inputmode="decimal" value={sheetW} oninput={(e) => (sheetW = num(e.currentTarget.value, sheetW))} /></label>
            <label>{t('export.sheet_h')}<input inputmode="decimal" value={sheetH} oninput={(e) => (sheetH = num(e.currentTarget.value, sheetH))} /></label>
            <label>{t('export.gap')}<input inputmode="decimal" value={gap} oninput={(e) => (gap = num(e.currentTarget.value, gap))} /></label>
            <label>{t('export.margin')}<input inputmode="decimal" value={margin} oninput={(e) => (margin = num(e.currentTarget.value, margin))} /></label>
          </div>
          <label class="check"><input type="checkbox" bind:checked={allowRotation} /> {t('export.allow_rotation')}</label>
          {#if preview && 'ok' in preview}
            <p class="preview" aria-live="polite">
              {t('export.nest_result', {
                count: preview.ok.count,
                cols: preview.ok.cols,
                rows: preview.ok.rows,
                used: Math.round(preview.ok.utilisation * 100),
              })}
            </p>
          {:else if preview && 'error' in preview}
            <p class="preview error" role="alert">{preview.error}</p>
          {/if}
        {/if}
      </fieldset>
    {/if}

    <footer>
      <button type="button" class="ghost" onclick={onclose}>{t('export.cancel')}</button>
      <button type="submit" class="primary" disabled={!!(preview && 'error' in preview)}>
        {t('export.go', { format: format === 'hpgl' ? 'PLT' : format.toUpperCase() })}
      </button>
    </footer>
  </form>
</dialog>

<style>
  dialog {
    width: min(560px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    padding: 0;
    border: 1px solid var(--pf-border);
    border-radius: 16px;
    background: var(--pf-surface);
    color: var(--pf-text);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
    font-family: var(--pf-font);
  }
  dialog::backdrop {
    background: rgba(10, 12, 16, 0.45);
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 18px 20px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h2 {
    margin: 0;
    font-size: 17px;
  }
  .x {
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--pf-muted);
    font-size: 22px;
    cursor: pointer;
  }
  .x:hover {
    background: var(--pf-hover);
  }
  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  legend {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--pf-muted);
    margin-block-end: 8px;
  }
  .formats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }
  .formats legend {
    grid-column: 1 / -1;
  }
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 12px;
    border: 1px solid var(--pf-border);
    border-radius: 10px;
    cursor: pointer;
  }
  .card input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .card:has(input:focus-visible) {
    outline: 2px solid var(--pf-accent);
    outline-offset: 2px;
  }
  .card.active {
    border-color: var(--pf-accent);
    background: var(--pf-accent-soft);
  }
  .card strong {
    font-size: 14px;
  }
  .card span {
    font-size: 11.5px;
    color: var(--pf-muted);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .label {
    font-size: 13px;
    font-weight: 500;
  }
  select,
  .grid input {
    height: 34px;
    padding-inline: 10px;
    border: 1px solid var(--pf-border);
    border-radius: 8px;
    background: var(--pf-input);
    color: var(--pf-text);
    font: inherit;
    font-size: 13px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .check input {
    accent-color: var(--pf-accent);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }
  .grid label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--pf-muted);
  }
  .segmented {
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--pf-hover);
    border-radius: 9px;
  }
  .segmented button {
    min-width: 46px;
    height: 28px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--pf-text);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .segmented button.active {
    background: var(--pf-surface);
    box-shadow: var(--pf-shadow);
    font-weight: 600;
  }
  .preview {
    margin: 0;
    padding: 8px 12px;
    border-radius: 8px;
    background: var(--pf-accent-soft);
    font-size: 13px;
  }
  .preview.error {
    background: var(--pf-danger-soft);
    color: var(--pf-danger);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .primary,
  .ghost {
    height: 36px;
    padding-inline: 16px;
    border-radius: 9px;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .primary {
    border: 1px solid var(--pf-accent);
    background: var(--pf-accent);
    color: var(--pf-accent-text);
  }
  .primary:disabled {
    opacity: 0.5;
  }
  .ghost {
    border: 1px solid var(--pf-border);
    background: var(--pf-surface);
    color: var(--pf-text);
  }
  @media (max-width: 520px) {
    .formats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
