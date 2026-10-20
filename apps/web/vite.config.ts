import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const isCustomElement = (filename: string) => /[\\/]src[\\/]wc[\\/]/.test(filename);

// Standalone designer app (index.html) and the embed demo page.
export default defineConfig({
  plugins: [
    svelte({
      dynamicCompileOptions: ({ filename }) =>
        isCustomElement(filename) ? { customElement: true } : undefined,
    }),
  ],
  build: {
    outDir: 'dist/app',
    target: 'es2022',
    rollupOptions: {
      input: { index: 'index.html', embed: 'embed-demo.html' },
    },
  },
});
