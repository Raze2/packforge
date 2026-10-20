// TypeScript mirrors of the JSON produced by pf-core (see crates/pf-model and pf-component).

export type LineType =
  | 'cut'
  | 'crease'
  | 'reverse-crease'
  | 'perforation'
  | 'half-cut'
  | 'bleed'
  | 'dimension'
  | 'annotation';

export const LINE_TYPES: LineType[] = [
  'cut',
  'crease',
  'reverse-crease',
  'perforation',
  'half-cut',
  'bleed',
  'dimension',
  'annotation',
];

export interface Point {
  x: number;
  y: number;
}

export type Segment =
  | { kind: 'line'; from: Point; to: Point }
  | { kind: 'arc'; center: Point; radius: number; start: number; sweep: number };

export interface Path {
  segments: Segment[];
  closed: boolean;
}

export interface Entity {
  line_type: LineType;
  path: Path;
}

export interface DieLine {
  entities: Entity[];
}

export interface Panel {
  id: string;
  origin: Point;
  width: number;
  height: number;
}

export interface Hinge {
  a: string;
  b: string;
  from: Point;
  to: Point;
  closed_angle: number;
}

export interface FoldModel {
  panels: Panel[];
  hinges: Hinge[];
  thickness: number;
}

export interface Board {
  id: string;
  name: string;
  caliper_mm: number;
  flute?: string;
}

export interface Stats {
  blank_width: number;
  blank_height: number;
  blank_area_m2: number;
  outline_area_m2: number;
  cut_length: number;
  crease_length: number;
}

export type Severity = 'info' | 'warning' | 'error';

export interface Diagnostic {
  severity: Severity;
  message: string;
}

export interface RenderResult {
  component_id: string;
  component_version: string;
  board: Board;
  dieline: DieLine;
  fold: FoldModel;
  stats: Stats;
  diagnostics: Diagnostic[];
}

export type ParamValue = number | boolean | string;
export type ParamType = 'length' | 'number' | 'angle' | 'bool' | 'enum';

export interface ParamDef {
  id: string;
  label: string;
  type: ParamType;
  default: ParamValue;
  min?: number;
  max?: number;
  step?: number;
  values?: string[];
  group?: string;
  help?: string;
}

export type Tier = 'free' | 'premium';

export interface ComponentSummary {
  id: string;
  version: string;
  name: string;
  standard: string | null;
  category: string;
  tier: Tier;
}

export interface ComponentInfo {
  id: string;
  version: string;
  name: string;
  standard?: string;
  category: string;
  tier: Tier;
  dimension_reference?: string;
  description?: string;
  /** Board preset suggested for this component (e.g. folding boxboard for cartons). */
  default_board?: string;
  params: ParamDef[];
}

export type Units = 'mm' | 'cm' | 'in';

/** Board is a preset id or a custom board. */
export type BoardRef = string | Board;

/** A saved design. Lengths in `params` are always millimetres. */
export interface Project {
  component_id: string;
  component_version?: string;
  params: Record<string, ParamValue>;
  board: BoardRef;
  units: Units;
}

export type ExportFormat = 'svg' | 'dxf';
