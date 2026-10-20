<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { core } from '../core';
  import { BoxRenderer, type Camera, type Palette } from '../gl';
  import type { Folded, Project, Vec3 } from '../types';

  interface Props {
    project: Project;
    corrugated: boolean;
    dark: boolean;
    labels: { fold: string; play: string; flatten: string; reset: string; unsupported: string };
  }

  let { project, corrugated, dark, labels }: Props = $props();

  let canvas: HTMLCanvasElement | undefined = $state();
  let renderer: BoxRenderer | null = null;
  let unsupported = $state(false);
  let progress = $state(1);
  let playing = $state(false);
  const DEFAULT_CAM: Camera = { yaw: 0.65, pitch: 0.42, zoom: 1 };
  let cam: Camera = $state({ ...DEFAULT_CAM });

  // Frame the larger of the flat and erected shapes so nothing leaves the view while folding.
  let frame = $state<{ target: Vec3; radius: number } | null>(null);
  let folded: Folded | null = null;

  function extent(f: Folded) {
    return {
      c: { x: (f.min.x + f.max.x) / 2, y: (f.min.y + f.max.y) / 2, z: (f.min.z + f.max.z) / 2 },
      r: Math.hypot(f.max.x - f.min.x, f.max.y - f.min.y, f.max.z - f.min.z) / 2,
    };
  }

  $effect(() => {
    // Re-frame whenever the design changes.
    const snap = $state.snapshot(project) as Project;
    try {
      const erected = extent(core.fold(snap, 1));
      const flat = extent(core.fold(snap, 0));
      frame = { target: erected.c, radius: Math.max(erected.r, flat.r * 0.75) };
    } catch {
      frame = null;
    }
  });

  $effect(() => {
    const snap = $state.snapshot(project) as Project;
    try {
      folded = core.fold(snap, progress);
      renderer?.setGeometry(folded);
    } catch {
      folded = null;
    }
    void cam.yaw;
    void cam.pitch;
    void cam.zoom;
    void frame;
    void dark;
    void corrugated;
    requestDraw();
  });

  const palette = (): Palette => ({
    outside: corrugated ? [0.78, 0.62, 0.42] : [0.96, 0.95, 0.92],
    inside: corrugated ? [0.62, 0.47, 0.3] : [0.82, 0.8, 0.76],
    edge: dark ? [0.1, 0.1, 0.12] : [0.25, 0.2, 0.15],
    background: dark ? [0.07, 0.08, 0.1] : [0.98, 0.98, 0.99],
  });

  let pending = 0;
  function requestDraw() {
    if (pending || !renderer) return;
    pending = requestAnimationFrame(() => {
      pending = 0;
      if (renderer && frame && folded) renderer.draw(frame.target, frame.radius, cam, palette());
    });
  }

  let resize: ResizeObserver | null = null;
  onMount(() => {
    if (!canvas) return;
    try {
      renderer = new BoxRenderer(canvas);
    } catch {
      unsupported = true;
      return;
    }
    if (folded) renderer.setGeometry(folded);
    resize = new ResizeObserver(() => requestDraw());
    resize.observe(canvas);
    requestDraw();
  });

  onDestroy(() => {
    resize?.disconnect();
    if (pending) cancelAnimationFrame(pending);
    if (anim) cancelAnimationFrame(anim);
  });

  // ── Interaction ─────────────────────────────────────────────────────────
  let drag: { x: number; y: number; id: number } | null = null;
  function onpointerdown(e: PointerEvent) {
    drag = { x: e.clientX, y: e.clientY, id: e.pointerId };
    canvas?.setPointerCapture(e.pointerId);
  }
  function onpointermove(e: PointerEvent) {
    if (!drag || drag.id !== e.pointerId) return;
    cam.yaw -= (e.clientX - drag.x) * 0.008;
    cam.pitch = Math.max(-1.5, Math.min(1.5, cam.pitch + (e.clientY - drag.y) * 0.008));
    drag = { ...drag, x: e.clientX, y: e.clientY };
  }
  function onpointerup() {
    drag = null;
  }
  function onwheel(e: WheelEvent) {
    e.preventDefault();
    cam.zoom = Math.max(0.3, Math.min(4, cam.zoom * Math.exp(e.deltaY * 0.0015)));
  }
  function onkeydown(e: KeyboardEvent) {
    const step = 0.1;
    if (e.key === 'ArrowLeft') cam.yaw += step;
    else if (e.key === 'ArrowRight') cam.yaw -= step;
    else if (e.key === 'ArrowUp') cam.pitch = Math.min(1.5, cam.pitch + step);
    else if (e.key === 'ArrowDown') cam.pitch = Math.max(-1.5, cam.pitch - step);
    else if (e.key === '0') resetView();
    else return;
    e.preventDefault();
  }
  function resetView() {
    cam = { ...DEFAULT_CAM };
  }

  // ── Fold animation ──────────────────────────────────────────────────────
  let anim = 0;
  function animateTo(goal: number) {
    if (anim) cancelAnimationFrame(anim);
    const start = progress;
    const t0 = performance.now();
    const duration = 900 * Math.abs(goal - start) + 1;
    playing = true;
    const step = (now: number) => {
      const k = Math.min(1, (now - t0) / duration);
      const ease = k < 0.5 ? 2 * k * k : 1 - (-2 * k + 2) ** 2 / 2;
      progress = start + (goal - start) * ease;
      if (k < 1) anim = requestAnimationFrame(step);
      else {
        anim = 0;
        playing = false;
      }
    };
    anim = requestAnimationFrame(step);
  }
</script>

<div class="fold">
  {#if unsupported}
    <p class="msg">{labels.unsupported}</p>
  {:else}
    <canvas
      bind:this={canvas}
      aria-label="3D preview. Drag to rotate, scroll to zoom, 0 to reset."
      tabindex="0"
      {onpointerdown}
      {onpointermove}
      {onpointerup}
      onpointercancel={onpointerup}
      {onwheel}
      {onkeydown}
      ondblclick={resetView}
    ></canvas>
  {/if}
  <div class="controls">
    <button type="button" class="pill" onclick={() => animateTo(progress < 0.99 ? 1 : 0)} disabled={playing}>
      {progress < 0.99 ? labels.play : labels.flatten}
    </button>
    <label>
      <span>{labels.fold}</span>
      <input
        type="range"
        min="0"
        max="100"
        step="1"
        value={Math.round(progress * 100)}
        oninput={(e) => (progress = Number(e.currentTarget.value) / 100)}
      />
      <output>{Math.round(progress * 100)}%</output>
    </label>
    <button type="button" class="pill ghost" onclick={resetView}>{labels.reset}</button>
  </div>
</div>

<style>
  .fold {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 280px;
    border-radius: var(--pf-radius);
    overflow: hidden;
    background: var(--pf-canvas);
  }
  canvas {
    display: block;
    width: 100%;
    height: 100%;
    cursor: grab;
    touch-action: none;
    outline: none;
  }
  canvas:active {
    cursor: grabbing;
  }
  canvas:focus-visible {
    box-shadow: inset 0 0 0 2px var(--pf-accent);
  }
  .msg {
    margin: 0;
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--pf-muted);
  }
  .controls {
    position: absolute;
    inset-inline: 12px;
    inset-block-end: 12px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: var(--pf-surface);
    border: 1px solid var(--pf-border);
    border-radius: 12px;
    box-shadow: var(--pf-shadow);
    max-width: 560px;
  }
  label {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--pf-muted);
    min-width: 180px;
  }
  input[type='range'] {
    flex: 1;
    accent-color: var(--pf-accent);
  }
  output {
    font-variant-numeric: tabular-nums;
    min-width: 36px;
    text-align: end;
    color: var(--pf-text);
  }
  .pill {
    height: 30px;
    padding-inline: 12px;
    border: 1px solid var(--pf-accent);
    border-radius: 8px;
    background: var(--pf-accent);
    color: var(--pf-accent-text);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .pill.ghost {
    background: transparent;
    color: var(--pf-accent);
  }
  .pill:disabled {
    opacity: 0.6;
  }
</style>
