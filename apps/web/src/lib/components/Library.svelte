<script lang="ts">
  import { core } from '../core';
  import { dielineBox, pathData } from '../geometry';
  import type { ComponentSummary } from '../types';

  interface Props {
    components: ComponentSummary[];
    selected: string;
    t: (key: string, vars?: Record<string, string | number>, fallback?: string) => string;
    onselect: (id: string) => void;
  }

  let { components, selected, t, onselect }: Props = $props();
  let query = $state('');

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return components.filter((c) =>
      [c.id, c.name, c.standard ?? '', t(`component.${c.id}`, undefined, c.name)]
        .join(' ')
        .toLowerCase()
        .includes(q),
    );
  });

  const byCategory = $derived.by(() => {
    const map = new Map<string, ComponentSummary[]>();
    for (const c of filtered) map.set(c.category, [...(map.get(c.category) ?? []), c]);
    return [...map.entries()];
  });

  // Thumbnails rendered once per component with default parameters.
  const thumbs = new Map<string, { d: string; vb: string } | null>();
  function thumb(id: string) {
    if (!thumbs.has(id)) {
      try {
        const r = core.render({ component_id: id, params: {}, board: 'C-flute', units: 'mm' });
        const b = dielineBox(r.dieline);
        const d = r.dieline.entities
          .filter((e) => e.line_type === 'cut' || e.line_type === 'crease')
          .map((e) => pathData(e.path))
          .join('');
        thumbs.set(
          id,
          b ? { d, vb: `${b.minX} ${-b.maxY} ${b.maxX - b.minX} ${b.maxY - b.minY}` } : null,
        );
      } catch {
        thumbs.set(id, null);
      }
    }
    return thumbs.get(id) ?? null;
  }
</script>

<div class="library">
  <input
    class="search"
    type="search"
    placeholder={t('library.search')}
    aria-label={t('library.search')}
    bind:value={query}
  />
  {#each byCategory as [category, list] (category)}
    <h3>{t(`category.${category}`, undefined, category)}</h3>
    <ul>
      {#each list as c (c.id)}
        {@const th = thumb(c.id)}
        <li>
          <button
            type="button"
            class:active={c.id === selected}
            aria-pressed={c.id === selected}
            onclick={() => onselect(c.id)}
          >
            <span class="thumb" aria-hidden="true">
              {#if th}
                <svg viewBox={th.vb} preserveAspectRatio="xMidYMid meet">
                  <g transform="scale(1,-1)"><path d={th.d} /></g>
                </svg>
              {/if}
            </span>
            <span class="text">
              <span class="name">{t(`component.${c.id}`, undefined, c.name)}</span>
              <span class="meta">
                {c.standard ?? c.id}
                <span class="tier {c.tier}">{t(`tier.${c.tier}`)}</span>
              </span>
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="empty">{t('library.empty')}</p>
  {/each}
</div>

<style>
  .library {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .search {
    height: 34px;
    padding-inline: 10px;
    border: 1px solid var(--pf-border);
    border-radius: 8px;
    background: var(--pf-input);
    color: var(--pf-text);
    font: inherit;
    font-size: 13px;
  }
  .search:focus {
    outline: none;
    border-color: var(--pf-accent);
    box-shadow: 0 0 0 3px var(--pf-accent-soft);
  }
  h3 {
    margin: 8px 0 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--pf-muted);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  li button {
    width: 100%;
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 8px;
    border: 1px solid transparent;
    border-radius: 10px;
    background: transparent;
    color: var(--pf-text);
    text-align: start;
    font: inherit;
    cursor: pointer;
  }
  li button:hover {
    background: var(--pf-hover);
  }
  li button.active {
    background: var(--pf-accent-soft);
    border-color: var(--pf-accent);
  }
  .thumb {
    flex: none;
    width: 64px;
    height: 40px;
    display: grid;
    place-items: center;
    background: var(--pf-canvas);
    border-radius: 6px;
    padding: 3px;
    box-sizing: border-box;
  }
  .thumb svg {
    width: 100%;
    height: 100%;
  }
  .thumb path {
    fill: none;
    stroke: var(--pf-text);
    stroke-width: 1px;
    vector-effect: non-scaling-stroke;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .name {
    font-size: 13px;
    font-weight: 600;
  }
  .meta {
    font-size: 12px;
    color: var(--pf-muted);
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .tier {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 99px;
    background: var(--pf-hover);
  }
  .tier.premium {
    background: var(--pf-premium-soft);
    color: var(--pf-premium);
  }
  .empty {
    font-size: 13px;
    color: var(--pf-muted);
  }
</style>
