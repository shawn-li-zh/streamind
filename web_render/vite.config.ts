// https://vite.dev/config/
// export default defineConfig({
//   plugins: [vue()],
// })

// vite.config.js
import {defineConfig} from 'vite';
import vue from '@vitejs/plugin-vue';
import {resolve} from 'path';
import * as console from "node:console";
import {viteExternalsPlugin} from "vite-plugin-externals";

const commonDeps = ['vue', 'pinia', 'vue-router', 'element-plus', '@element-plus/icons-vue', "axios"];
const srcPath = resolve(__dirname, 'src');

export default defineConfig(({mode}) => {
    console.log(mode)
    // 这里的环境变量由 build.js 传入
    const name = process.env.name;
    const path = process.env.path;
    if (!path) {
        throw new Error('❌ 请指定组件路径: --path=ComponentA');
    }
    return {
        plugins: [vue(), viteExternalsPlugin({
            "vue": 'SMD.Vue',
            'element-plus': 'SMD.ElementPlus',
            "@element-plus/icons-vue": 'SMD.ElementPlusIcons',

        }, {useWindow: false})],
        resolve: {
            alias: {
                // 开发时，让脚本能通过 @/render 找到源码
                "@": resolve(__dirname, 'src'),
            }
        },
        build: {
            lib: {
                // 入口动态指定
                entry: resolve(srcPath, `${path}/index.vue`),
                name: name,
                fileName: () => `index.js`,
                formats: ['es'], // 必须输出 esm 格式
            },
            rollupOptions: {
                // 关键：排除所有公共依赖
                external: (id) => {
                    console.log("Dep:" + id)
                    // 1. 基础依赖排除
                    if (commonDeps.includes(id)) return true;
                    // 2. 排除所有以 @/render/ 开头的路径
                    // 但要排除掉“当前组件自己”的路径（防止组件内部的子组件被排除）
                    if (id.startsWith('@/render/')) {
                        // 如果 id 是 '@/render/ComponentD'，但当前正在编 D，则不排除
                        // 如果 id 是 '@/render/ComponentA'，当前在编 D，则排除
                        return !id.startsWith(path);
                    }
                    return false;
                },
                globals: {
                    vue: 'Vue'
                }
            },
            emptyOutDir: false, // 增量编译不清空目录
            outDir: `dist/${name}` // 每个组件打包到独立文件夹
        }
    };
});
