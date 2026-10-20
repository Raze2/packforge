<script lang="ts">
  import Designer, { type DesignerApi } from './lib/Designer.svelte';
  import { connectDesktop, desktopPlatform, isDesktop } from './desktop';

  const desktop = isDesktop();
  let api: DesignerApi | null = null;
</script>

<Designer
  persist
  platform={desktop ? desktopPlatform : undefined}
  bindApi={(a) => (api = a)}
  onready={() => {
    if (desktop && api) connectDesktop(api).catch((e: unknown) => api?.notify(String(e)));
  }}
/>
