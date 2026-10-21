<script lang="ts">
  import { core, CoreError } from '../core';
  import { download } from '../storage';
  import type { PalletPreset, PalletRequest, PalletResult, Project } from '../types';

  interface Props {
    project: Project;
    locale: string;
    t: (key: string, vars?: Record<string, string | number>) => string;
  }

  let { project, locale, t }: Props = $props();

  const PRESETS: PalletPreset[] = ['eur', 'eur-half', 'iso-1200x1000', 'gma', 'asia'];
  let pallet = $state<PalletPreset>('eur');
  let maxHeight = $state(1800);
  let boxWeight = $state<number | null>(null);
  let maxWeight = $state<number | null>(null);
  let overhang = $state(0);
  let onSide = $state(false);

  const request = $derived<PalletRequest>({
    pallet,
    max_height: maxHeight,
    box_weight: boxWeight,
    max_weight: maxWeight,
    overhang,
    allow_on_side: onSide,
  });

  const result = $derived.by((): { ok: PalletResult } | { error: string } => {
    try {
      return { ok: core.palletPlan($state.snapshot(project) as Project, request) };
    } catch (e) {
      return { error: e instanceof CoreError ? e.message : String(e) };
    }
  });

  const fmt = (v: number, d = 0) => v.toLocaleString(locale, { maximumFractionDigits: d });
  const num = (v: string): number | null => {
    const n = Number(v.replace(',', '.'));
    return v.trim() !== '' && Number.isFinite(n) && n >= 0 ? n : null;
  };

  function report() {
    try {
      const bytes = core.palletReport($state.snapshot(project) as Project, request);
      download(bytes, `${project.component_id}-pallet.pdf`, 'application/pdf');
    } catch {
      // the same error is already shown in the panel
    }
  }

  // Side view: pallet plus layers, scaled into the same viewBox width as the top view.
  const side = $derived.by(() => {
    if (!('ok' in result)) return null;
    const p = result.ok.plan;
    return { w: p.pallet.length, h: p.load_height };
  });
</script>

<div class="pallet">
  <form class="controls" onsubmit={(e) => e.preventDefault()}>
    <label>
      <span>{t('pallet.type')}</span>
      <select bind:value={pallet}>
        {#each PRESETS as p (p)}<option value={p}>{t(`pallet.preset.${p}`)}</option>{/each}
      </select>
    </label>
    <label>
      <span>{t('pallet.max_height')}</span>
      <input inputmode="decimal" value={maxHeight} oninput={(e) => (maxHeight = num(e.currentTarget.value) ?? maxHeight)} />
    </label>
    <label>
      <span>{t('pallet.box_weight')}</span>
      <input inputmode="decimal" placeholder="—" value={boxWeight ?? ''} oninput={(e) => (boxWeight = num(e.currentTarget.value))} />
    </label>
    <label>
      <span>{t('pallet.max_weight')}</span>
      <input inputmode="decimal" placeholder="—" value={maxWeight ?? ''} oninput={(e) => (maxWeight = num(e.currentTarget.value))} />
    </label>
    <label>
      <span>{t('pallet.overhang')}</span>
      <input inputmode="decimal" value={overhang} oninput={(e) => (overhang = num(e.currentTarget.value) ?? 0)} />
    </label>
    <label class="check"><input type="checkbox" bind:checked={onSide} /> {t('pallet.on_side')}</label>
  </form>

  {#if 'error' in result}
    <p class="error" role="alert">{result.error}</p>
  {:else}
    {@const r = result.ok}
    {@const p = r.plan}
    <div class="views">
      <figure>
        <svg viewBox="-20 -20 {p.pallet.length + 40} {p.pallet.width + 40}" role="img" aria-label={t('pallet.top')}>
          <rect class="board" x="0" y="0" width={p.pallet.length} height={p.pallet.width} />
          {#each p.layer as b, i (i)}
            <rect
              class="box"
              class:turned={b.w < b.h !== r.box_outer[0] < r.box_outer[1]}
              x={b.x}
              y={p.pallet.width - b.y - b.h}
              width={b.w}
              height={b.h}
            />
          {/each}
        </svg>
        <figcaption>{t('pallet.top')}</figcaption>
      </figure>
      {#if side}
        <figure>
          <svg viewBox="-20 -20 {side.w + 40} {side.h + 40}" role="img" aria-label={t('pallet.side')}>
            <rect class="board" x="0" y={side.h - p.pallet.height} width={side.w} height={p.pallet.height} />
            {#each Array(p.layers) as _, k (k)}
              <rect class="box" class:turned={p.alternate && k % 2 === 1} x="0" y={side.h - p.pallet.height - (k + 1) * p.layer_height} width={side.w} height={p.layer_height} />
            {/each}
          </svg>
          <figcaption>{t('pallet.side')}</figcaption>
        </figure>
      {/if}
    </div>
    <dl class="figures">
      <div><dt>{t('pallet.total')}</dt><dd>{fmt(p.total)}</dd></div>
      <div><dt>{t('pallet.per_layer')}</dt><dd>{p.per_layer} × {p.layers}</dd></div>
      <div><dt>{t('pallet.pattern')}</dt><dd>{t(`pallet.pattern.${p.pattern}`)}</dd></div>
      <div><dt>{t('pallet.area')}</dt><dd>{fmt(p.area_efficiency * 100)}%</dd></div>
      <div><dt>{t('pallet.load_height')}</dt><dd>{fmt(p.load_height)} mm</dd></div>
      {#if p.load_weight !== null}<div><dt>{t('pallet.load_weight')}</dt><dd>{fmt(p.load_weight)} kg</dd></div>{/if}
      <div><dt>{t('pallet.box_outer')}</dt><dd>{fmt(r.box_outer[0])} × {fmt(r.box_outer[1])} × {fmt(r.box_outer[2])} mm</dd></div>
      <div><dt>{t('pallet.limited_by')}</dt><dd>{t(`pallet.limit.${p.limited_by}`)}</dd></div>
    </dl>
    <div class="actions">
      <button type="button" class="primary" onclick={report}>{t('pallet.report')}</button>
      <span class="note">{t('pallet.note')}</span>
    </div>
  {/if}
</div>

<style>
  .pallet {
    width: 100%;
    height: 100%;
    overflow: auto;
    padding: 56px 16px 16px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 14px;
    background: var(--pf-canvas);
    border-radius: var(--pf-radius);
  }
  .controls {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 10px;
    align-items: end;
  }
  .controls label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--pf-muted);
  }
  .controls .check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
    color: var(--pf-text);
    font-size: 13px;
  }
  select,
  input:not([type='checkbox']) {
    height: 34px;
    padding-inline: 10px;
    border: 1px solid var(--pf-border);
    border-radius: 8px;
    background: var(--pf-input);
    color: var(--pf-text);
    font: inherit;
    font-size: 13px;
  }
  .views {
    display: grid;
    grid-template-columns: 3fr 2fr;
    gap: 14px;
  }
  figure {
    margin: 0;
    padding: 10px;
    background: var(--pf-surface);
    border: 1px solid var(--pf-border);
    border-radius: 12px;
  }
  svg {
    width: 100%;
    max-height: 300px;
    display: block;
  }
  figcaption {
    margin-top: 6px;
    font-size: 12px;
    color: var(--pf-muted);
    text-align: center;
  }
  .board {
    fill: #b48a5a;
    opacity: 0.35;
    stroke: #8a6236;
    stroke-width: 3;
    vector-effect: non-scaling-stroke;
  }
  .box {
    fill: #d6b384;
    stroke: #5a3d20;
    stroke-width: 1.2px;
    vector-effect: non-scaling-stroke;
  }
  .box.turned {
    fill: #c49a66;
  }
  .figures {
    margin: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 10px 16px;
  }
  dt {
    font-size: 11px;
    color: var(--pf-muted);
  }
  dd {
    margin: 0;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .actions {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
  }
  .primary {
    height: 34px;
    padding-inline: 14px;
    border: 1px solid var(--pf-accent);
    border-radius: 9px;
    background: var(--pf-accent);
    color: var(--pf-accent-text);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .note {
    font-size: 12px;
    color: var(--pf-muted);
  }
  .error {
    margin: 0;
    padding: 10px 14px;
    border-radius: 10px;
    background: var(--pf-danger-soft);
    color: var(--pf-danger);
  }
  @container pf (max-width: 820px) {
    .views {
      grid-template-columns: 1fr;
    }
  }
</style>
