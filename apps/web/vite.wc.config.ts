import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const isCustomElement = (filename: string) => /[\\/]src[\\/]wc[\\/]/.test(filename);

// <packforge-designer> custom element: one self-contained ES module (WASM and CSS inlined)
// so it can be dropped into any page, Laravel Blade view or framework app.
export default defineConfig({
  plugins: [
    svelte({
      compilerOptions: { css: 'injected' },
      dynamicCompileOptions: ({ filename }) =>
        isCustomElement(filename) ? { customElement: true } : undefined,
    }),
  ],
  build: {
    outDir: 'dist/wc',
    target: 'es2022',
    assetsInlineLimit: 50 * 1024 * 1024,
    emptyOutDir: true,
    minify: true,
    lib: {
      entry: 'src/wc/index.ts',
      formats: ['es'],
      fileName: () => 'packforge-designer.js',
    },
  },
});
