<svelte:options
  customElement={{
    tag: 'packforge-designer',
    shadow: 'none',
    props: {
      component: { attribute: 'component', type: 'String' },
      params: { attribute: 'params', type: 'Object' },
      board: { attribute: 'board', type: 'String' },
      units: { attribute: 'units', type: 'String' },
      locale: { attribute: 'locale', type: 'String' },
      theme: { attribute: 'theme', type: 'String' },
      mode: { attribute: 'mode', type: 'String' },
      persist: { attribute: 'persist', type: 'Boolean' },
      exportEndpoint: { attribute: 'export-endpoint', type: 'String' },
      csrfToken: { attribute: 'csrf-token', type: 'String' },
    },
  }}
/>

<script lang="ts">
  // <packforge-designer> — embeddable designer for any page (Laravel Blade, React, Vue, HTML).
  // Events (bubbling, composed): pf:ready, pf:change, pf:save, pf:export, pf:error.
  // pf:save and pf:export are cancelable: call preventDefault() to skip the browser download.
  import { onMount } from 'svelte';
  import Designer, { type DesignerApi, type ExportDetail } from '../lib/Designer.svelte';
  import { isLocale } from '../lib/i18n';
  import { download } from '../lib/storage';
  import type { BoardRef, ExportFormat, ExportOptions, ParamValue, Project, Units } from '../lib/types';

  interface Props {
    component?: string;
    params?: Record<string, ParamValue>;
    board?: string;
    units?: string;
    locale?: string;
    theme?: string;
    mode?: string;
    persist?: boolean;
    exportEndpoint?: string;
    /** Sent as X-CSRF-TOKEN with exports (Laravel). Falls back to <meta name="csrf-token">. */
    csrfToken?: string;
  }

  let {
    component = 'fefco-0201',
    params = {},
    board = 'C-flute',
    units = 'mm',
    locale = 'en',
    theme = 'auto',
    mode = 'full',
    persist = false,
    exportEndpoint,
    csrfToken,
  }: Props = $props();

  // With shadow: 'none' the component renders directly inside <packforge-designer>.
  let anchor: HTMLSpanElement | undefined = $state();
  const host = (): HTMLElement | null => anchor?.parentElement ?? null;
  let api: DesignerApi | null = null;

  const safeUnits = (u: string): Units => (u === 'cm' || u === 'in' ? u : 'mm');
  const safeTheme = (t: string) => (t === 'light' || t === 'dark' ? t : 'auto');

  /** `board` is a preset id, or a custom board as JSON: {"id":"x","caliper_mm":3.2}. */
  function parseBoard(b: string): BoardRef {
    if (!b.trim().startsWith('{')) return b;
    try {
      const v = JSON.parse(b) as { id?: unknown; name?: unknown; caliper_mm?: unknown };
      if (typeof v.caliper_mm === 'number' && v.caliper_mm > 0) {
        return {
          id: typeof v.id === 'string' ? v.id : 'custom',
          name: typeof v.name === 'string' ? v.name : 'Custom',
          caliper_mm: v.caliper_mm,
        };
      }
    } catch {
      // fall through to the default preset
    }
    return 'C-flute';
  }

  function emit(name: string, detail: unknown, cancelable = false): boolean {
    const ev = new CustomEvent(name, { detail, bubbles: true, composed: true, cancelable });
    host()?.dispatchEvent(ev);
    return ev.defaultPrevented;
  }

  onMount(() => {
    const h = host();
    if (h && !h.style.display) h.style.display = 'block';
  });

  async function remoteExport(d: ExportDetail): Promise<void> {
    const headers: Record<string, string> = { 'Content-Type': 'application/json', Accept: '*/*' };
    const csrf =
      csrfToken ?? document.querySelector<HTMLMetaElement>('meta[name="csrf-token"]')?.content;
    if (csrf) headers['X-CSRF-TOKEN'] = csrf;
    const res = await fetch(exportEndpoint as string, {
      method: 'POST',
      headers,
      credentials: 'same-origin',
      body: JSON.stringify({ project: d.project, format: d.format, options: d.options }),
    });
    if (!res.ok) throw new Error(`export failed: ${res.status}`);
    download(new Uint8Array(await res.arrayBuffer()), d.filename, d.mime);
  }

  // Public element API (also available as element methods).
  export function getProject(): Project | null {
    return api?.getProject() ?? null;
  }
  export function setProject(p: Project): void {
    api?.setProject(p);
  }
  export function exportFile(format: ExportFormat, options?: ExportOptions): ExportDetail | null {
    return api?.exportFile(format, options) ?? null;
  }
</script>

<span bind:this={anchor} hidden></span>
<Designer
  {component}
  {params}
  board={parseBoard(board)}
  units={safeUnits(units)}
  locale={isLocale(locale) ? locale : 'en'}
  theme={safeTheme(theme)}
  mode={mode === 'configurator' ? 'configurator' : 'full'}
  {persist}
  bindApi={(a) => (api = a)}
  onready={() => emit('pf:ready', null)}
  onchange={(d) => emit('pf:change', d)}
  onerror={(e) => emit('pf:error', { code: e.code, message: e.message })}
  onsave={(p) => emit('pf:save', { project: p }, true)}
  onexport={(d) => {
    if (emit('pf:export', d, true)) return true;
    if (exportEndpoint) {
      remoteExport(d).catch((e: unknown) =>
        emit('pf:error', { code: 'export_failed', message: String(e) }),
      );
      return true;
    }
    return false;
  }}
/>
