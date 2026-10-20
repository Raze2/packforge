import type { DieLine, LineType, Path } from './types';

export interface Box {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

const TOOLING = new Set<LineType>(['cut', 'crease', 'reverse-crease', 'perforation', 'half-cut']);

/**
 * SVG path data in model coordinates (y up). Draw inside a `scale(1,-1)` group: in model space a
 * counter-clockwise arc (positive sweep) is the positive-angle direction, i.e. sweep-flag 1.
 */
export function pathData(p: Path): string {
  let d = '';
  let cx: number | null = null;
  let cy: number | null = null;
  for (const s of p.segments) {
    const [sx, sy] =
      s.kind === 'line'
        ? [s.from.x, s.from.y]
        : [s.center.x + s.radius * Math.cos(s.start), s.center.y + s.radius * Math.sin(s.start)];
    if (cx === null || cy === null || Math.hypot(sx - cx, sy - cy) > 1e-6) d += `M${sx} ${sy}`;
    if (s.kind === 'line') {
      d += `L${s.to.x} ${s.to.y}`;
      [cx, cy] = [s.to.x, s.to.y];
    } else {
      const parts = Math.abs(s.sweep) > 1.9 * Math.PI ? 2 : 1;
      for (let i = 1; i <= parts; i++) {
        const a = s.start + (s.sweep * i) / parts;
        const ex = s.center.x + s.radius * Math.cos(a);
        const ey = s.center.y + s.radius * Math.sin(a);
        const large = Math.abs(s.sweep / parts) > Math.PI ? 1 : 0;
        const flag = s.sweep > 0 ? 1 : 0;
        d += `A${s.radius} ${s.radius} 0 ${large} ${flag} ${ex} ${ey}`;
        [cx, cy] = [ex, ey];
      }
    }
  }
  return p.closed ? d + 'Z' : d;
}

/** Bounds of tooling lines (the blank), approximating arcs by their full circle box. */
export function dielineBox(d: DieLine): Box | null {
  let b: Box | null = null;
  const add = (x: number, y: number) => {
    if (!b) b = { minX: x, minY: y, maxX: x, maxY: y };
    else {
      b.minX = Math.min(b.minX, x);
      b.minY = Math.min(b.minY, y);
      b.maxX = Math.max(b.maxX, x);
      b.maxY = Math.max(b.maxY, y);
    }
  };
  for (const e of d.entities) {
    if (!TOOLING.has(e.line_type)) continue;
    for (const s of e.path.segments) {
      if (s.kind === 'line') {
        add(s.from.x, s.from.y);
        add(s.to.x, s.to.y);
      } else {
        add(s.center.x - s.radius, s.center.y - s.radius);
        add(s.center.x + s.radius, s.center.y + s.radius);
      }
    }
  }
  return b;
}
