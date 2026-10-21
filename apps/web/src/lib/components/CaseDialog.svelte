<script lang="ts">
  import { core, CoreError } from '../core';
  import type { CaseCandidate, PalletPreset, Project } from '../types';

  interface Props {
    open: boolean;
    project: Project;
    locale: string;
    t: (key: string, vars?: Record<string, string | number>) => string;
    onclose: () => void;
    onuse: (inside: [number, number, number]) => void;
  }

  let { open, project, locale, t, onclose, onuse }: Props = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  let pl = $state(100);
  let pw = $state(80);
  let ph = $state(60);
  let count = $state(12);
  let clearance = $state(4);
  let upright = $state(false);
  let rankPallet = $state(false);
  let pallet = $state<PalletPreset>('eur');
  let price = $state<number | null>(null);

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });

  const results = $derived.by((): { ok: CaseCandidate[] } | { error: string } | null => {
    if (!open) return null;
    const snap = $state.snapshot(project) as Project;
    try {
      return {
        ok: core.caseOptimize({
          product: [pl, pw, ph],
          count,
          component_id: snap.component_id,
          board: snap.board,
          clearance,
          allow_rotation: !upright,
          max_results: 8,
          pallet: rankPallet ? { pallet } : null,
          board_price_per_m2: price,
        }),
      };
    } catch (e) {
      return { error: e instanceof CoreError ? e.message : String(e) };
    }
  });

  const fmt = (v: number, d = 0) => v.toLocaleString(locale, { maximumFractionDigits: d });
  const num = (v: string, fallback: number) => {
    const n = Number(v.replace(',', '.'));
    return Number.isFinite(n) && n > 0 ? n : fallback;
  };
</script>

<dialog bind:this={dialog} onclose={onclose} aria-labelledby="pf-case-title">
  <form method="dialog" onsubmit={(e) => e.preventDefault()}>
    <header>
      <h2 id="pf-case-title">{t('case.title')}</h2>
      <button type="button" class="x" aria-label={t('export.cancel')} onclick={onclose}>×</button>
    </header>
    <p class="intro">{t('case.intro')}</p>

    <div class="grid">
      <label>{t('case.product_l')}<input inputmode="decimal" value={pl} oninput={(e) => (pl = num(e.currentTarget.value, pl))} /></label>
      <label>{t('case.product_w')}<input inputmode="decimal" value={pw} oninput={(e) => (pw = num(e.currentTarget.value, pw))} /></label>
      <label>{t('case.product_h')}<input inputmode="decimal" value={ph} oninput={(e) => (ph = num(e.currentTarget.value, ph))} /></label>
      <label>{t('case.count')}<input inputmode="numeric" value={count} oninput={(e) => (count = Math.round(num(e.currentTarget.value, count)))} /></label>
      <label>{t('case.clearance')}<input inputmode="decimal" value={clearance} oninput={(e) => (clearance = num(e.currentTarget.value, clearance))} /></label>
      <label>{t('case.price')}<input inputmode="decimal" placeholder="—" value={price ?? ''} oninput={(e) => (price = e.currentTarget.value.trim() ? num(e.currentTarget.value, 0) : null)} /></label>
    </div>
    <div class="row">
      <label class="check"><input type="checkbox" bind:checked={upright} /> {t('case.upright')}</label>
      <label class="check"><input type="checkbox" bind:checked={rankPallet} /> {t('case.rank_pallet')}</label>
      {#if rankPallet}
        <select bind:value={pallet} aria-label={t('pallet.type')}>
          {#each ['eur', 'eur-half', 'iso-1200x1000', 'gma', 'asia'] as p (p)}
            <option value={p}>{t(`pallet.preset.${p}`)}</option>
          {/each}
        </select>
      {/if}
    </div>

    {#if results && 'error' in results}
      <p class="error" role="alert">{results.error}</p>
    {:else if results}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>{t('case.layout')}</th>
              <th>{t('case.inside')}</th>
              <th>{t('case.board')}</th>
              {#if price !== null}<th>{t('case.cost')}</th>{/if}
              {#if rankPallet}<th>{t('case.per_pallet')}</th>{/if}
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each results.ok as c, i (i)}
              <tr class:best={i === 0}>
                <td>{c.arrangement.join(' × ')}</td>
                <td>{c.inside.map((v) => fmt(v)).join(' × ')}</td>
                <td>{fmt(c.blank_area_m2, 3)} m²</td>
                {#if price !== null}<td>{fmt(c.board_cost_per_case ?? 0, 3)}</td>{/if}
                {#if rankPallet}<td>{fmt(c.products_per_pallet ?? 0)}</td>{/if}
                <td><button type="button" class="use" onclick={() => onuse(c.inside)}>{t('case.use')}</button></td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </form>
</dialog>

<style>
  dialog {
    width: min(720px, calc(100vw - 32px));
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
    gap: 14px;
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
  .intro {
    margin: 0;
    font-size: 13px;
    color: var(--pf-muted);
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
  .grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }
  .grid label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--pf-muted);
  }
  input:not([type='checkbox']),
  select {
    height: 34px;
    padding-inline: 10px;
    border: 1px solid var(--pf-border);
    border-radius: 8px;
    background: var(--pf-input);
    color: var(--pf-text);
    font: inherit;
    font-size: 13px;
  }
  .row {
    display: flex;
    gap: 16px;
    align-items: center;
    flex-wrap: wrap;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .table-wrap {
    overflow: auto;
    max-height: 340px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  th,
  td {
    padding: 8px 10px;
    text-align: start;
    border-bottom: 1px solid var(--pf-border);
    white-space: nowrap;
  }
  th {
    font-size: 11px;
    color: var(--pf-muted);
    font-weight: 600;
    position: sticky;
    top: 0;
    background: var(--pf-surface);
  }
  tr.best td {
    font-weight: 600;
    background: var(--pf-accent-soft);
  }
  .use {
    height: 28px;
    padding-inline: 12px;
    border: 1px solid var(--pf-accent);
    border-radius: 7px;
    background: transparent;
    color: var(--pf-accent);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .error {
    margin: 0;
    padding: 10px 14px;
    border-radius: 10px;
    background: var(--pf-danger-soft);
    color: var(--pf-danger);
  }
  @media (max-width: 560px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
