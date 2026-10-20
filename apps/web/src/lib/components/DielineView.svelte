<script lang="ts">
  import { dielineBox, pathData, type Box } from '../geometry';
  import type { LineType, RenderResult, Units } from '../types';
  import { formatLength } from '../units';

  interface Props {
    result: RenderResult | null;
    hidden: Set<LineType>;
    units: Units;
    locale: string;
    /** Changes whenever the view should re-fit automatically (e.g. new component). */
    fitKey: string;
    labels: { fit: string; zoomIn: string; zoomOut: string };
  }

  let { result, hidden, units, locale, fitKey, labels }: Props = $props();

  let svg: SVGSVGElement | undefined = $state();
  let vb = $state({ x: 0, y: 0, w: 100, h: 100 });
  let userMoved = $state(false);
  let lastFitKey = '';

  const box: Box | null = $derived(result ? dielineBox(result.dieline) : null);

  const groups = $derived.by(() => {
    if (!result) return [];
    const byType = new Map<LineType, string[]>();
    for (const e of result.dieline.entities) {
      if (hidden.has(e.line_type)) continue;
      const list = byType.get(e.line_type) ?? [];
      list.push(pathData(e.path));
      byType.set(e.line_type, list);
    }
    return [...byType.entries()].map(([type, paths]) => ({ type, d: paths.join('') }));
  });

  function fit() {
    if (!box) return;
    const w = box.maxX - box.minX;
    const h = box.maxY - box.minY;
    const pad = Math.max(w, h) * 0.06 + 10;
    // Extra room on the right and bottom for the dimension lines and labels.
    const dimRoom = Math.max(w, h) * 0.12;
    // SVG space is the model flipped vertically: y_svg = -y_model.
    vb = {
      x: box.minX - pad,
      y: -box.maxY - pad,
      w: w + 2 * pad + dimRoom,
      h: h + 2 * pad + dimRoom * 0.6,
    };
    userMoved = false;
  }

  $effect(() => {
    // Auto-fit on new component/board, and on parameter changes until the user pans/zooms.
    void box;
    if (fitKey !== lastFitKey || !userMoved) {
      lastFitKey = fitKey;
      fit();
    }
  });

  function toSvg(clientX: number, clientY: number) {
    const m = svg?.getScreenCTM();
    if (!svg || !m) return { x: 0, y: 0 };
    const p = new DOMPoint(clientX, clientY).matrixTransform(m.inverse());
    return { x: p.x, y: p.y };
  }

  function zoomAt(factor: number, cx?: number, cy?: number) {
    const c =
      cx !== undefined && cy !== undefined
        ? toSvg(cx, cy)
        : { x: vb.x + vb.w / 2, y: vb.y + vb.h / 2 };
    const w = Math.min(Math.max(vb.w * factor, 5), 1e6);
    const f = w / vb.w;
    vb = { x: c.x - (c.x - vb.x) * f, y: c.y - (c.y - vb.y) * f, w, h: vb.h * f };
    userMoved = true;
  }

  function onwheel(e: WheelEvent) {
    e.preventDefault();
    zoomAt(Math.exp(e.deltaY * 0.0015), e.clientX, e.clientY);
  }

  let drag: { x: number; y: number; id: number } | null = null;

  function onpointerdown(e: PointerEvent) {
    if (e.button !== 0) return;
    const p = toSvg(e.clientX, e.clientY);
    drag = { ...p, id: e.pointerId };
    svg?.setPointerCapture(e.pointerId);
  }

  function onpointermove(e: PointerEvent) {
    if (!drag || drag.id !== e.pointerId) return;
    const p = toSvg(e.clientX, e.clientY);
    vb = { ...vb, x: vb.x + drag.x - p.x, y: vb.y + drag.y - p.y };
    userMoved = true;
  }

  function onpointerup(e: PointerEvent) {
    if (drag?.id === e.pointerId) drag = null;
  }

  function onkeydown(e: KeyboardEvent) {
    const step = vb.w * 0.08;
    const actions: Record<string, () => void> = {
      '+': () => zoomAt(0.8),
      '=': () => zoomAt(0.8),
      '-': () => zoomAt(1.25),
      '0': fit,
      ArrowLeft: () => (vb = { ...vb, x: vb.x - step }),
      ArrowRight: () => (vb = { ...vb, x: vb.x + step }),
      ArrowUp: () => (vb = { ...vb, y: vb.y - step }),
      ArrowDown: () => (vb = { ...vb, y: vb.y + step }),
    };
    const act = actions[e.key];
    if (act) {
      e.preventDefault();
      act();
      if (e.key !== '0') userMoved = true;
    }
  }

  // Overall dimension lines, drawn in SVG space (outside the flipped group so text is upright).
  const dims = $derived.by(() => {
    if (!box) return null;
    const gap = Math.max(box.maxX - box.minX, box.maxY - box.minY) * 0.035 + 4;
    const font = Math.max(vb.w, vb.h) * 0.022;
    return {
      font,
      w: {
        y: -box.minY + gap,
        x1: box.minX,
        x2: box.maxX,
        label: formatLength(box.maxX - box.minX, units, locale),
      },
      h: {
        x: box.maxX + gap,
        y1: -box.maxY,
        y2: -box.minY,
        label: formatLength(box.maxY - box.minY, units, locale),
      },
    };
  });
</script>

<div class="view">
  <!-- A pan/zoom canvas is a custom widget (role="application"): it needs focus and pointer/keyboard handlers. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <svg
    bind:this={svg}
    class="stage"
    viewBox="{vb.x} {vb.y} {vb.w} {vb.h}"
    preserveAspectRatio="xMidYMid meet"
    role="application"
    aria-label="Die-line viewer. Drag to pan, scroll or +/- to zoom, 0 to fit."
    tabindex="0"
    {onwheel}
    {onpointerdown}
    {onpointermove}
    {onpointerup}
    onpointercancel={onpointerup}
    {onkeydown}
  >
    <g transform="scale(1,-1)">
      {#each groups as g (g.type)}
        <path class="line {g.type}" d={g.d} />
      {/each}
    </g>
    {#if dims}
      <g class="dims" font-size={dims.font}>
        <line x1={dims.w.x1} y1={dims.w.y} x2={dims.w.x2} y2={dims.w.y} />
        <line x1={dims.w.x1} y1={dims.w.y - dims.font * 0.6} x2={dims.w.x1} y2={dims.w.y + dims.font * 0.6} />
        <line x1={dims.w.x2} y1={dims.w.y - dims.font * 0.6} x2={dims.w.x2} y2={dims.w.y + dims.font * 0.6} />
        <text x={(dims.w.x1 + dims.w.x2) / 2} y={dims.w.y + dims.font * 1.4} text-anchor="middle">
          {dims.w.label}
        </text>
        <line x1={dims.h.x} y1={dims.h.y1} x2={dims.h.x} y2={dims.h.y2} />
        <line x1={dims.h.x - dims.font * 0.6} y1={dims.h.y1} x2={dims.h.x + dims.font * 0.6} y2={dims.h.y1} />
        <line x1={dims.h.x - dims.font * 0.6} y1={dims.h.y2} x2={dims.h.x + dims.font * 0.6} y2={dims.h.y2} />
        <text
          x={dims.h.x + dims.font * 0.8}
          y={(dims.h.y1 + dims.h.y2) / 2}
          dominant-baseline="middle"
        >
          {dims.h.label}
        </text>
      </g>
    {/if}
  </svg>

  <div class="zoom">
    <button type="button" onclick={() => zoomAt(0.8)} aria-label={labels.zoomIn} title={labels.zoomIn}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg>
    </button>
    <button type="button" onclick={() => zoomAt(1.25)} aria-label={labels.zoomOut} title={labels.zoomOut}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14" /></svg>
    </button>
    <button type="button" onclick={fit} aria-label={labels.fit} title={labels.fit}>
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" />
      </svg>
    </button>
  </div>
</div>

<style>
  .view {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 280px;
    background-color: var(--pf-canvas);
    background-image: radial-gradient(var(--pf-grid) 1px, transparent 1px);
    background-size: 20px 20px;
    border-radius: var(--pf-radius);
    overflow: hidden;
  }
  svg.stage {
    display: block;
    width: 100%;
    height: 100%;
    cursor: grab;
    touch-action: none;
    outline: none;
  }
  svg.stage:active {
    cursor: grabbing;
  }
  svg.stage:focus-visible {
    box-shadow: inset 0 0 0 2px var(--pf-accent);
  }
  .line {
    fill: none;
    stroke-width: 1.4px;
    vector-effect: non-scaling-stroke;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .cut {
    stroke: var(--pf-line-cut);
  }
  .crease {
    stroke: var(--pf-line-crease);
    stroke-dasharray: 6 3;
  }
  .reverse-crease {
    stroke: var(--pf-line-reverse);
    stroke-dasharray: 6 2 1 2;
  }
  .perforation {
    stroke: var(--pf-line-perf);
    stroke-dasharray: 2 2;
  }
  .half-cut {
    stroke: var(--pf-line-half);
    stroke-dasharray: 8 2;
  }
  .bleed {
    stroke: var(--pf-line-bleed);
    stroke-dasharray: 1 2;
  }
  .dimension,
  .annotation {
    stroke: var(--pf-muted);
  }
  .dims line {
    stroke: var(--pf-muted);
    stroke-width: 1px;
    vector-effect: non-scaling-stroke;
  }
  .dims {
    /* Drawing coordinates are always left-to-right, also in RTL layouts. */
    direction: ltr;
    unicode-bidi: isolate;
  }
  .dims text {
    fill: var(--pf-muted);
    font-family: var(--pf-font);
  }
  .zoom {
    position: absolute;
    inset-block-end: 12px;
    inset-inline-end: 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    background: var(--pf-surface);
    border: 1px solid var(--pf-border);
    border-radius: 10px;
    padding: 4px;
    box-shadow: var(--pf-shadow);
  }
  .zoom button {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--pf-text);
    cursor: pointer;
  }
  .zoom button:hover {
    background: var(--pf-hover);
  }
  .zoom svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
