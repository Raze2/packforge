import type { Units } from './types';

const MM_PER: Record<Units, number> = { mm: 1, cm: 10, in: 25.4 };
const DIGITS: Record<Units, number> = { mm: 1, cm: 2, in: 3 };

export const UNITS: Units[] = ['mm', 'cm', 'in'];

export function toDisplay(mm: number, u: Units): number {
  return mm / MM_PER[u];
}

export function fromDisplay(v: number, u: Units): number {
  return v * MM_PER[u];
}

/** Rounded value for inputs (no trailing zeros). */
export function inputValue(mm: number, u: Units): number {
  const f = 10 ** DIGITS[u];
  return Math.round(toDisplay(mm, u) * f) / f;
}

export function formatLength(mm: number, u: Units, locale: string): string {
  return `${toDisplay(mm, u).toLocaleString(locale, {
    maximumFractionDigits: DIGITS[u],
  })} ${u}`;
}

/** Slider/input step in display units. */
export function stepFor(u: Units): number {
  return u === 'mm' ? 1 : u === 'cm' ? 0.1 : 0.05;
}
