<template>
  <div class="flex h-screen w-full overflow-hidden bg-gray-100">
    <!-- 左侧菜单 -->
    <Sidebar />

    <!-- 右侧内容区 -->
    <main class="flex-1 flex flex-col min-w-0 overflow-hidden">
      <div class="flex-1 overflow-y-auto p-3">
        <!-- 核心变化：使用 transition 增加页面切换丝滑感 -->
        <router-view v-slot="{ Component }">
          <transition name="fade-transform" mode="out-in">
            <component :is="Component" class="bg-white min-h-full rounded-lg shadow-sm" />
          </transition>
        </router-view>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import Sidebar from './Sidebar.vue'
</script>

<style scoped>
/* 简单的切换动画 */
.fade-transform-enter-active,
.fade-transform-leave-active {
  transition: all 0.2s;
}

.fade-transform-enter-from {
  opacity: 0;
  transform: translateX(-20px);
}

.fade-transform-leave-to {
  opacity: 0;
  transform: translateX(20px);
}
</style>
