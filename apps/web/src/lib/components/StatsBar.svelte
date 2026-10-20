<script lang="ts">
  import type { Diagnostic, Stats, Units } from '../types';
  import { formatLength } from '../units';

  interface Props {
    stats: Stats | null;
    diagnostics: Diagnostic[];
    units: Units;
    locale: string;
    renderMs: number | null;
    t: (key: string) => string;
  }

  let { stats, diagnostics, units, locale, renderMs, t }: Props = $props();

  const area = (m2: number) => `${m2.toLocaleString(locale, { maximumFractionDigits: 3 })} m²`;
  const len = (mm: number) =>
    units === 'mm'
      ? `${(mm / 1000).toLocaleString(locale, { maximumFractionDigits: 2 })} m`
      : formatLength(mm, units, locale);
</script>

<div class="bar">
  {#if stats}
    <dl>
      <div>
        <dt>{t('stats.blank')}</dt>
        <dd>
          {formatLength(stats.blank_width, units, locale)} × {formatLength(
            stats.blank_height,
            units,
            locale,
          )}
        </dd>
      </div>
      <div>
        <dt>{t('stats.blank_area')}</dt>
        <dd>{area(stats.blank_area_m2)}</dd>
      </div>
      <div>
        <dt>{t('stats.outline_area')}</dt>
        <dd>{area(stats.outline_area_m2)}</dd>
      </div>
      <div>
        <dt>{t('stats.cut')}</dt>
        <dd>{len(stats.cut_length)}</dd>
      </div>
      <div>
        <dt>{t('stats.crease')}</dt>
        <dd>{len(stats.crease_length)}</dd>
      </div>
      {#if renderMs !== null}
        <div class="perf">
          <dt>{t('stats.render')}</dt>
          <dd>{renderMs.toLocaleString(locale, { maximumFractionDigits: 1 })} ms</dd>
        </div>
      {/if}
    </dl>
  {/if}
  {#if diagnostics.length}
    <ul class="diags" aria-live="polite">
      {#each diagnostics as d, i (i)}
        <li class={d.severity}>{d.message}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .bar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 24px;
    align-items: center;
    justify-content: space-between;
  }
  dl {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 24px;
    margin: 0;
  }
  dl div {
    display: flex;
    flex-direction: column;
  }
  dt {
    font-size: 11px;
    color: var(--pf-muted);
  }
  dd {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .perf dd {
    font-weight: 500;
    color: var(--pf-muted);
  }
  .diags {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .diags li {
    font-size: 12px;
    padding: 4px 10px;
    border-radius: 6px;
    background: var(--pf-hover);
  }
  .diags li.warning {
    background: var(--pf-warning-soft);
    color: var(--pf-warning);
  }
  .diags li.error {
    background: var(--pf-danger-soft);
    color: var(--pf-danger);
  }
</style>
