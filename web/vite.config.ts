import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    port: 5173,
    proxy: {
      // dev: forward API + SSE to the Rust server
      '/api': { target: 'http://127.0.0.1:8787', changeOrigin: true },
    },
  },
});
