import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  build: {
    lib: {
      entry: {
        index: resolve(__dirname, 'src/index.ts'),
        'browser/lifestats': resolve(__dirname, 'src/browser.ts'),
      },
      formats: ['es', 'cjs'],
    },
    sourcemap: true,
    minify: false, // Keep readable for debugging
  },
});
