export interface Menu {
  name: string
  path: string
  title: string
  icon: string
  component: any
  children?: Menu[]
}

export const menus: Menu[] = [
  {
    name: '对话',
    path: '/',
    title: '对话',
    icon: 'talk',
    component: () => import('../views/chat/index.vue')
  },
  {
    name: '智能体',
    path: '/agent',
    title: '智能体',
    icon: 'bot',
    component: () => import('../views/agent/index.vue')
  },
  {
    name: '模型',
    path: '/model',
    title: '模型',
    icon: 'model',
    component: () => import('../views/model/index.vue')
  },

  {
    name: '工具',
    path: '/tool',
    title: '工具',
    icon: 'tool',
    component: undefined,
  },
  {
    name: '组件',
    path: '/component',
    title: '组件',
    icon: 'module',
    component: () => import('../views/component/index.vue')
  },
  {
    name: '设置',
    path: '/setting',
    title: '设置',
    icon: 'setting',
    component: undefined,
  }
]
