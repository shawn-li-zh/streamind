import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'
import { menus } from '@/router/memu.ts'
import { useAppStore } from '@/stores/app.ts'
import { ElMessage } from 'element-plus'

const menuRoutes: RouteRecordRaw[] = menus.map(menu => {
  return {
    path: menu.path,
    name: menu.name,
    component: menu.component,
    meta: {
      title: menu.title,
      icon: menu.icon,
    },
  }
})
const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      name: '登录',
      path: '/login',
      component: () => import('../views/login/index.vue'),
      meta: {
        title: '登录',
      },
    },
    {
      path: '/emergence',
      component: () => import('../views/emergence/index.vue'),
    },
    {
      path: '/',
      component: () => import('../layout/MainLayout.vue'),
      meta: { requiresAuth: true }, // 标记整个 Layout 及其子路由都需要鉴权
      // 这里的 children 才是你真正放在 MainLayout 右侧业务区域的内容
      children: menuRoutes,
    },
  ],
})

// 路由全局前置守卫：鉴权拦截
router.beforeEach((to, from, next) => {
  const appStore = useAppStore()
  // 获取我们在 Login.vue 中存入的登录码
  const token = appStore.token
  if (to.meta.requiresAuth && !token) {
    // 1. 如果去需要权限的页面，但没登录 -> 踢回登录页
    ElMessage.error('请先登录！')
    next('/login')
  } else if (to.path === '/login' && token) {
    // 2. 如果已经登录了，还想去登录页 -> 踢回首页
    next('/')
  } else {
    // 3. 正常放行
    next()
  }
})

export default router
