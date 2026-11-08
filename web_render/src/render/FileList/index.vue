<template>
  <div class="w-full max-w-2xl bg-white border border-slate-200 rounded-xl shadow-sm overflow-hidden font-sans">
    <!-- 头部 -->
    <div class="px-4 py-3 border-b border-slate-100 bg-slate-50/50 flex justify-between items-center">
      <div class="flex items-center gap-2">
        <el-icon class="text-slate-500">
          <Files/>
        </el-icon>
        <span class="text-sm font-bold text-slate-700">附件清单</span>
      </div>
      <span class="text-[11px] text-slate-400 font-medium">{{ attrs.files.length }} 个文件</span>
    </div>

    <!-- 文件列表 -->
    <div class="divide-y divide-slate-100">
      <div
          v-for="(file, index) in attrs.files"
          :key="index"
          class="group flex items-center gap-4 px-4 py-3 hover:bg-blue-50/30 transition-colors"
      >
        <!-- 左侧：图标 -->
        <div
            class="flex-shrink-0 w-10 h-10 rounded-lg bg-slate-100 flex items-center justify-center group-hover:bg-blue-100 transition-colors">
          <el-icon :size="20" :class="getFileColor(file.name)">
            <component :is="getFileIcon(file.name)"/>
          </el-icon>
        </div>

        <!-- 中间：信息区 -->
        <div class="flex-1 min-w-0">
          <div class="flex items-baseline gap-2">
            <h4 class="text-sm font-semibold text-slate-800 truncate" :title="file.name">
              {{ file.name }}
            </h4>
          </div>

          <!-- 描述 (如果有) -->
          <p v-if="file.description" class="text-xs text-slate-400 truncate mt-0.5">
            {{ file.description }}
          </p>

          <!-- 元数据：时间 & 大小 -->
          <div class="flex items-center gap-3 mt-1">
            <span class="flex items-center gap-1 text-[11px] text-slate-400">
              <el-icon><Calendar/></el-icon>
              {{ file.time || '刚刚' }}
            </span>
            <span class="flex items-center gap-1 text-[11px] text-slate-400">
              <el-icon><ScaleToOriginal/></el-icon>
              {{ file.size || '未知大小' }}
            </span>
          </div>
        </div>

        <!-- 右侧：操作区 -->
        <div class="flex-shrink-0">
          <el-tooltip content="点击下载" placement="top">
            <el-button
                type="primary"
                circle
                plain
                size="small"
                class="!border-slate-200 group-hover:!border-blue-300"
                @click="handleDownload(file)"
            >
              <el-icon>
                <Download/>
              </el-icon>
            </el-button>
          </el-tooltip>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import {
  Calendar,
  Collection,
  Document,
  Download,
  Files,
  Picture,
  ScaleToOriginal,
  VideoPlay
} from '@element-plus/icons-vue'
import {ElButton, ElIcon, ElTooltip} from "element-plus"

interface File {
  url: string;
  name: string;
  description?: string; // 描述
  time?: string;        // 时间
  size?: string;        // 大小
}

interface Attrs {
  files: File[];
}

const props = defineProps<{
  attrs: Attrs
  value?: any
}>();

/**
 * 处理下载逻辑
 */
const handleDownload = (file: File) => {
  const link = document.createElement('a');
  link.href = file.url;
  link.download = file.name;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
};

/**
 * 根据文件名后缀返回图标
 */
const getFileIcon = (fileName: string) => {
  const ext = fileName.split('.').pop()?.toLowerCase();
  if (['jpg', 'jpeg', 'png', 'gif', 'svg'].includes(ext!)) return Picture;
  if (['mp4', 'mov', 'avi'].includes(ext!)) return VideoPlay;
  if (['zip', 'rar', '7z'].includes(ext!)) return Collection;
  return Document;
};

/**
 * 根据文件名后缀返回颜色类名
 */
const getFileColor = (fileName: string) => {
  const ext = fileName.split('.').pop()?.toLowerCase();
  if (['jpg', 'jpeg', 'png'].includes(ext!)) return 'text-orange-500';
  if (['mp4', 'mov'].includes(ext!)) return 'text-purple-500';
  if (['pdf'].includes(ext!)) return 'text-red-500';
  if (['zip', 'rar'].includes(ext!)) return 'text-amber-600';
  return 'text-blue-500';
};
</script>

<style scoped>
/* 自定义按钮悬停阴影 */
.el-button.is-circle:hover {
  box-shadow: 0 2px 8px -2px rgba(59, 130, 246, 0.3);
}

/* 进场动画 */
.divide-y > div {
  animation: slideIn 0.3s ease-out backwards;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateX(-10px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}
</style>
