<template>
  <div
      class="p-4 w-full max-w-2xl bg-white rounded-2xl shadow-sm border border-slate-100 flex flex-col space-y-3 font-sans">
    <!-- 头部：动态标题与计数 -->
    <div class="flex items-center justify-between px-0.5">
      <div class="flex items-center gap-2">
        <div class="w-1 h-4 bg-blue-500 rounded-full"></div>
        <span class="text-sm font-bold text-slate-700 tracking-tight">
          {{ attrs.title || '图片预览' }}
        </span>
      </div>
      <span class="text-[11px] font-medium px-2 py-0.5 bg-blue-50 text-blue-600 rounded-md border border-blue-100">
        {{ attrs.images?.length || 0 }} P
      </span>
    </div>

    <!-- 图片展示区 -->
    <div class="flex flex-wrap gap-3">
      <div
          v-for="(item, i) in attrs.images"
          :key="i"
          class="relative group overflow-hidden rounded-xl border border-slate-200 cursor-zoom-in transition-all duration-300 hover:border-blue-300 hover:shadow-lg hover:shadow-blue-100"
          :style="imageBoxStyle"
      >
        <el-image
            class="w-full h-full transform transition-transform duration-700 group-hover:scale-110"
            :src="getUrl(item.path)"
            :preview-src-list="previewList"
            :initial-index="i"
            :alt="item.name"
            fit="cover"
            loading="lazy"
            preview-teleported
        >
          <!-- 加载中 -->
          <template #placeholder>
            <div class="flex items-center justify-center w-full h-full bg-slate-50">
              <div class="w-5 h-5 border-2 border-blue-100 border-t-blue-500 rounded-full animate-spin"></div>
            </div>
          </template>
          <!-- 加载失败 -->
          <template #error>
            <div class="flex flex-col items-center justify-center w-full h-full bg-slate-50 text-slate-300">
              <el-icon :size="24">
                <Picture/>
              </el-icon>
              <span class="text-[10px] mt-1">加载失败</span>
            </div>
          </template>
        </el-image>

        <!-- 悬停遮罩层 -->
        <div
            class="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-300 pointer-events-none">
          <div class="absolute inset-0 flex items-center justify-center">
            <el-icon class="text-white drop-shadow-md" :size="24">
              <ZoomIn/>
            </el-icon>
          </div>
          <!-- 图片名称备注 (如果有) -->
          <div v-if="item.name" class="absolute bottom-0 left-0 right-0 p-2">
            <p class="text-white text-[13px] truncate text-center w-full drop-shadow-sm">
              {{ item.name }}
            </p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import {computed, inject} from 'vue'
import {ElIcon, ElImage} from "element-plus"
import {Picture, ZoomIn} from '@element-plus/icons-vue'
import type {StreamindRuntime} from "@/streamind.ts";

const rt = inject<StreamindRuntime>("runtime")!;

interface Image {
  //url或者本地文件路径
  path: string;
  name?: string;
}

interface Attrs {
  title?: string;
  images: Image[];
  width?: number;
  height?: number;
}

const props = defineProps<{
  attrs: Attrs
  value?: any
}>();

// 提取预览图链接数组
const previewList = computed(() => {
  return props.attrs.images?.map(img => {
    return getUrl(img.path)
  }) || [];
});

function getUrl(path: string) {
  if (!path) return ''
  if (!path.startsWith("http")) {
    return rt.getLocalImageUrl(path)
  }
  return path
}

// 计算图片格子尺寸
const imageBoxStyle = computed(() => ({
  width: (props.attrs.width || 100) + "px",
  height: (props.attrs.height || 100) + "px",
}));
</script>

<style scoped>
/* 深度美化预览组件 */
:deep(.el-image-viewer__wrapper) {
  z-index: 9999;
}

:deep(.el-image-viewer__canvas) {
  cursor: grab;
}

:deep(.el-image-viewer__canvas:active) {
  cursor: grabbing;
}

/* 入场动画：错落有致的淡入 */
.flex-wrap > div {
  animation: slideUp 0.5s ease-out backwards;
}

@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 为循环生成的图片设置延迟动画 */
.flex-wrap > div:nth-child(1) {
  animation-delay: 0.1s;
}

.flex-wrap > div:nth-child(2) {
  animation-delay: 0.15s;
}

.flex-wrap > div:nth-child(3) {
  animation-delay: 0.2s;
}

.flex-wrap > div:nth-child(4) {
  animation-delay: 0.25s;
}

.flex-wrap > div:nth-child(n+5) {
  animation-delay: 0.3s;
}
</style>
