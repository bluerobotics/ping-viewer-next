import { URL, fileURLToPath } from 'node:url';
import Vue from '@vitejs/plugin-vue';
import autoprefixer from 'autoprefixer';
import tailwindcss from 'tailwindcss';
import AutoImport from 'unplugin-auto-import/vite';
import Fonts from 'unplugin-fonts/vite';
import Components from 'unplugin-vue-components/vite';
import VueRouter from 'unplugin-vue-router/vite';
import { defineConfig } from 'vite';
import topLevelAwait from 'vite-plugin-top-level-await';
import Layouts from 'vite-plugin-vue-layouts';
import Vuetify, { transformAssetUrls } from 'vite-plugin-vuetify';
import wasm from 'vite-plugin-wasm';

const removeMdiPreload = {
  name: 'remove-eot-preload',
  order: 'post',
  transformIndexHtml: {
    order: 'post',
    handler(html) {
      return html.replace(/<link[^>]*?materialdesignicons[^>]*?>/g, '');
    },
  },
};

export default defineConfig({
  plugins: [
    wasm(),
    topLevelAwait(),
    VueRouter(),
    Layouts(),
    Vue({
      template: { transformAssetUrls },
    }),
    Vuetify({
      autoImport: true,
      styles: {
        configFile: 'src/styles/settings.scss',
      },
    }),
    Components(),
    Fonts({
      google: {
        families: [
          {
            name: 'Roboto',
            styles: 'wght@100;300;400;500;700;900',
          },
        ],
      },
    }),
    AutoImport({
      imports: ['vue', 'vue-router'],
      eslintrc: {
        enabled: true,
      },
      vueTemplate: true,
    }),
    removeMdiPreload,
  ],
  define: { 'process.env': {} },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@components': fileURLToPath(new URL('./src/components', import.meta.url)),
    },
    extensions: ['.js', '.json', '.jsx', '.mjs', '.ts', '.tsx', '.vue'],
  },
  server: {
    port: 3000,
    cors: {
      origin: '*',
    },
  },
  css: {
    postcss: {
      plugins: [tailwindcss, autoprefixer],
    },
  },
  optimizeDeps: {
    exclude: ['@syntect/wasm', '@foxglove/wasm-zstd', '@foxglove/wasm-lz4', '@foxglove/wasm-bz2'],
  },
  build: {
    target: 'esnext',
    rollupOptions: {
      external: [
        '@foxglove/wasm-lz4',
        '@foxglove/wasm-zstd',
        '@foxglove/wasm-bz2',
        '@syntect/wasm',
      ],
      output: {
        assetFileNames: 'assets/[name][extname]',
        globals: {
          '@foxglove/wasm-lz4': 'WasmLz4',
          '@foxglove/wasm-zstd': 'WasmZstd',
          '@foxglove/wasm-bz2': 'WasmBz2',
          '@syntect/wasm': 'SyntectWasm',
        },
      },
    },
  },
});
