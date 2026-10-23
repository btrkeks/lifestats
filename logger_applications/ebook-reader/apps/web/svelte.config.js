/* global process */
import adapter from '@sveltejs/adapter-static';
import preprocess from 'svelte-preprocess';

// jidoujisho's embedded profile: the host ships every asset, so no worker is registered, and
// the lever passes a generation derived from commit and source so rebuilds are byte-identical.
const generation = process.env.TTU_BUNDLE_GENERATION;

/** @type {import('@sveltejs/kit').Config} */
const config = {
  compilerOptions: {
    immutable: true
  },

  // Consult https://github.com/sveltejs/svelte-preprocess
  // for more information about preprocessors
  preprocess: [
    preprocess({
      postcss: true
    })
  ],

  kit: {
    serviceWorker: { register: process.env.VITE_SERVICE_WORKER !== 'off' },
    ...(generation && { version: { name: generation } }),
    adapter: adapter({
      fallback: '404.html'
    })
  }
};

export default config;
