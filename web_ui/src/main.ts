import './exportDeps.ts'
import './assets/main.css'
// import './assets/iconfont/iconfont.css'
import './assets/iconfont/iconfont.js'
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import TDesign from 'tdesign-vue-next'
import TDesignChat from '@tdesign-vue-next/chat' // 引入 Chat 组件
import 'tdesign-vue-next/es/style/index.css' // 引入少量全局样式变量
import '@tdesign-vue-next/chat/es/style/index.css'

import App from './App.vue'
import router from './router'

export const app = createApp(App)

app.use(createPinia().use(piniaPluginPersistedstate))
app.use(router)
app.use(ElementPlus)
app.use(TDesign).use(TDesignChat)
app.mount('#app')

// setTimeout(() => {
//   // const ele=document.querySelector("#app > div > main > div > div > div.flex.flex-row.w-full > div.flex.flex-col.h-full.bg-gray-50.text-gray-800.font-sans.w-5\\/12 > footer")
//   // const ele = document.querySelector('#app > div > main > div > div > header > div:nth-child(2)')
//   const ele = document.body
//   const a = get_readable_element(ele as any)
//   console.log(a.node)
// }, 3000)
