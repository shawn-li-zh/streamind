import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import vueDevTools from 'vite-plugin-vue-devtools'
import { resolve } from 'path'

const srcPath = resolve(__dirname, 'src')
// const commonDeps = ['vue', 'element-plus', '@element-plus/icons-vue']
const commonDeps = []
// https://vite.dev/config/
export default defineConfig({
  plugins: [vue(), vueJsx(), vueDevTools()],
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:3000', // 你的 Rust 后端地址
        changeOrigin: true, // 允许跨域
        // fetch(...args) {
        //   console.log(args[0] + '')
        //   return fetch(...args)
        // },
      },
      '/public': {
        target: 'http://localhost:3000', // 你的 Rust 后端地址
        changeOrigin: true, // 允许跨域
      },
    },
  },
  optimizeDeps: {
    // 1. 强制 Vite 不要预构建这两个包
    exclude: commonDeps,
  },
  resolve: {
    external: commonDeps,
    alias: {
      '@': srcPath,
      // "dayjs": "http://localhost:3000/public/dep/dayjs/index.js",
      // vue: 'https://unpkg.com/vue@3/dist/vue.esm-browser.js',
      // 'element-plus': 'https://unpkg.com/element-plus/dist/index.full.mjs',
      // '@element-plus/icons-vue': 'https://cdn.jsdelivr.net/npm/@element-plus/icons-vue@2.3.2/+esm',
      // '@/render/Grant': 'http://localhost:3000/public/components/Grant/index.js',
      // '@/render/Grant@1.0.0': 'http://localhost:3000/public/components/Grant/index.js',
      // '@/render/GrantList': 'http://localhost:3000/public/components/GrantList/index.js',
      // '@/render/GrantList@1.0.0': 'http://localhost:3000/public/components/GrantList/index.js',
      // '@element-plus/icons-vue': 'http://localhost:3000/public/dep/@element-plus/icons-vue_2.3.2.js',
      // '@element-plus/icons-vue@2.3.2':
      //   'http://localhost:3000/public/dep/@element-plus/icons-vue_2.3.2.js',
      // 'element-plus': 'http://localhost:3000/public/dep/element-plus_2.2.2.js',
      // 'element-plus@2.2.2': 'http://localhost:3000/public/dep/element-plus_2.2.2.js',
      // vue: 'http://localhost:3000/public/dep/vue_3.2.25.js',
      // 'vue@3.2.25': 'http://localhost:3000/public/dep/vue_3.2.25.js',
    },
  },
  build: {
    rollupOptions: {
      external: commonDeps,
    },
  },
})
