<template>
  <!-- 顶层路由出口，根据路径决定渲染 Login.vue 还是 MainLayout -->
  <router-view />
</template>

<script setup lang="ts"></script>

<style>
/* 确保基础样式重置 */
body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial;
}

#app {
  height: 100vh;
  width: 100vw;
  display: flex;
}
</style>
