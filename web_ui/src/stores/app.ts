import { defineStore } from 'pinia'
import piniaPersistConfig from '@/stores/helper/persist.ts'

const id = 'app'

export interface AppState {
  token: string
  theme: string
}

export const useAppStore = defineStore(id, {
  state: (): AppState => ({
    token: '',
    theme: '',
  }),
  getters: {},
  actions: {
    // 主题切换逻辑
    toggleTheme(theme: 'dark' | 'light') {
      const isDarkTheme = theme === 'dark'
      if (isDarkTheme) {
        document.documentElement.classList.add('dark')
      } else {
        document.documentElement.classList.remove('dark')
      }
      this.theme = theme
    },
  },
  persist: piniaPersistConfig(id, ['token', 'theme']),
})
