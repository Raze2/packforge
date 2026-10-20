<script lang="ts">
  import type { Board, BoardRef, ComponentInfo, ParamDef, ParamValue, Units } from '../types';
  import { fromDisplay, inputValue, stepFor } from '../units';

  interface Props {
    info: ComponentInfo;
    params: Record<string, ParamValue>;
    board: BoardRef;
    boards: Board[];
    units: Units;
    t: (key: string, vars?: Record<string, string | number>, fallback?: string) => string;
    onparam: (id: string, value: ParamValue) => void;
    onboard: (board: BoardRef) => void;
    onreset: () => void;
  }

  let { info, params, board, boards, units, t, onparam, onboard, onreset }: Props = $props();

  const groups = $derived.by(() => {
    const map = new Map<string, ParamDef[]>();
    for (const p of info.params) {
      const g = p.group ?? 'Other';
      map.set(g, [...(map.get(g) ?? []), p]);
    }
    return [...map.entries()];
  });

  // Text being edited per field, so partially typed numbers are not overwritten.
  let drafts: Record<string, string> = $state({});
  let errors: Record<string, string> = $state({});

  const isLength = (p: ParamDef) => p.type === 'length';
  const label = (p: ParamDef) => t(`param.${info.id}.${p.id}`, undefined, p.label);
  const current = (p: ParamDef): ParamValue => params[p.id] ?? p.default;

  function shown(p: ParamDef): number {
    const v = Number(current(p));
    return isLength(p) ? inputValue(v, units) : v;
  }

  function bound(p: ParamDef, which: 'min' | 'max'): number | undefined {
    const v = p[which];
    if (v === undefined) return undefined;
    return isLength(p) ? inputValue(v, units) : v;
  }

  function commitNumber(p: ParamDef, text: string) {
    drafts[p.id] = text;
    const n = Number(text.replace(',', '.'));
    if (text.trim() === '' || !Number.isFinite(n)) {
      errors[p.id] = t('error.number');
      return;
    }
    const mm = isLength(p) ? fromDisplay(n, units) : n;
    if (p.min !== undefined && mm < p.min - 1e-9) {
      errors[p.id] = t('error.min', { value: bound(p, 'min') ?? p.min });
      return;
    }
    if (p.max !== undefined && mm > p.max + 1e-9) {
      errors[p.id] = t('error.max', { value: bound(p, 'max') ?? p.max });
      return;
    }
    delete errors[p.id];
    onparam(p.id, mm);
  }

  function endEdit(p: ParamDef) {
    delete drafts[p.id];
    delete errors[p.id];
  }

  const boardId = $derived(typeof board === 'string' ? board : 'custom');
  const caliper = $derived(
    typeof board === 'string'
      ? (boards.find((b) => b.id === board)?.caliper_mm ?? 0)
      : board.caliper_mm,
  );

  function selectBoard(id: string) {
    if (id === 'custom') {
      onboard({ id: 'custom', name: t('params.custom_board'), caliper_mm: caliper || 3 });
    } else {
      onboard(id);
    }
  }

  function setCaliper(text: string) {
    const n = Number(text.replace(',', '.'));
    if (Number.isFinite(n) && n > 0 && n <= 30) {
      onboard({ id: 'custom', name: t('params.custom_board'), caliper_mm: n });
    }
  }
</script>

<form class="form" onsubmit={(e) => e.preventDefault()}>
  {#each groups as [group, defs] (group)}
    <fieldset>
      <legend>{t(`group.${group}`, undefined, group)}</legend>
      {#each defs as p (p.id)}
        {@const fieldId = `pf-${info.id}-${p.id}`}
        <div class="field" class:invalid={errors[p.id]}>
          <label for={fieldId}>{label(p)}</label>
          {#if p.type === 'enum'}
            <div class="segmented" role="radiogroup" aria-label={label(p)} id={fieldId}>
              {#each p.values ?? [] as v (v)}
                <button
                  type="button"
                  role="radio"
                  aria-checked={current(p) === v}
                  class:active={current(p) === v}
                  onclick={() => onparam(p.id, v)}
                >
                  {t(`enum.${v}`, undefined, v)}
                </button>
              {/each}
            </div>
          {:else if p.type === 'bool'}
            <input
              id={fieldId}
              type="checkbox"
              checked={current(p) === true}
              onchange={(e) => onparam(p.id, e.currentTarget.checked)}
            />
          {:else}
            <div class="number">
              <input
                id={fieldId}
                type="text"
                inputmode="decimal"
                autocomplete="off"
                value={drafts[p.id] ?? String(shown(p))}
                aria-invalid={errors[p.id] ? 'true' : undefined}
                aria-describedby={errors[p.id] ? `${fieldId}-err` : undefined}
                oninput={(e) => commitNumber(p, e.currentTarget.value)}
                onblur={() => endEdit(p)}
              />
              {#if isLength(p)}<span class="unit">{units}</span>{/if}
            </div>
            {#if p.min !== undefined && p.max !== undefined}
              <input
                class="slider"
                type="range"
                aria-label={label(p)}
                min={bound(p, 'min')}
                max={bound(p, 'max')}
                step={isLength(p) ? stepFor(units) : (p.step ?? 1)}
                value={shown(p)}
                oninput={(e) => {
                  endEdit(p);
                  commitNumber(p, e.currentTarget.value);
                }}
              />
            {/if}
            {#if errors[p.id]}
              <p class="error" id="{fieldId}-err" role="alert">{errors[p.id]}</p>
            {/if}
          {/if}
          {#if p.help}<p class="help">{p.help}</p>{/if}
        </div>
      {/each}
    </fieldset>
  {/each}

  <fieldset>
    <legend>{t('params.board')}</legend>
    <div class="field">
      <label for="pf-board">{t('params.board')}</label>
      <select id="pf-board" value={boardId} onchange={(e) => selectBoard(e.currentTarget.value)}>
        {#each boards as b (b.id)}
          <option value={b.id}>{b.name} · {b.caliper_mm} mm</option>
        {/each}
        <option value="custom">{t('params.custom_board')}</option>
      </select>
    </div>
    {#if boardId === 'custom'}
      <div class="field">
        <label for="pf-caliper">{t('params.caliper')}</label>
        <div class="number">
          <input
            id="pf-caliper"
            type="text"
            inputmode="decimal"
            value={String(caliper)}
            oninput={(e) => setCaliper(e.currentTarget.value)}
          />
          <span class="unit">mm</span>
        </div>
      </div>
    {/if}
  </fieldset>

  <button type="button" class="reset" onclick={onreset}>{t('params.reset')}</button>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  fieldset {
    border: 0;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  legend {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--pf-muted);
    margin-block-end: 10px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  label {
    font-size: 13px;
    font-weight: 500;
  }
  .number {
    position: relative;
    display: flex;
    align-items: center;
  }
  .number input,
  select {
    width: 100%;
    box-sizing: border-box;
    height: 36px;
    padding-inline: 10px 40px;
    border: 1px solid var(--pf-border);
    border-radius: 8px;
    background: var(--pf-input);
    color: var(--pf-text);
    font: inherit;
    font-variant-numeric: tabular-nums;
  }
  select {
    padding-inline-end: 10px;
  }
  .number input:focus,
  select:focus {
    outline: none;
    border-color: var(--pf-accent);
    box-shadow: 0 0 0 3px var(--pf-accent-soft);
  }
  .invalid .number input {
    border-color: var(--pf-danger);
  }
  .unit {
    position: absolute;
    inset-inline-end: 10px;
    font-size: 12px;
    color: var(--pf-muted);
    pointer-events: none;
  }
  .slider {
    width: 100%;
    accent-color: var(--pf-accent);
    margin: 0;
  }
  .segmented {
    display: flex;
    gap: 2px;
    padding: 2px;
    background: var(--pf-hover);
    border-radius: 9px;
  }
  .segmented button {
    flex: 1;
    height: 30px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--pf-text);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .segmented button.active {
    background: var(--pf-surface);
    box-shadow: var(--pf-shadow);
    font-weight: 600;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--pf-danger);
  }
  .help {
    margin: 0;
    font-size: 12px;
    color: var(--pf-muted);
  }
  .reset {
    align-self: flex-start;
    border: 0;
    background: none;
    color: var(--pf-accent);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    padding: 0;
  }
  .reset:hover {
    text-decoration: underline;
  }
</style>
