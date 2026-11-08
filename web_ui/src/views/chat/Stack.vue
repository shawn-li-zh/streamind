<template>
  <div
    class="flex flex-col bg-slate-50/50 dark:bg-slate-900/50 border-l border-slate-200 dark:border-slate-800 backdrop-blur-sm"
  >
    <!-- 头部：标题与统计 -->
    <div
      class="px-4 py-3 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between bg-white/50 dark:bg-slate-800/50"
    >
      <div class="flex items-center gap-2">
        <h2 class="text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400">
          记忆锚点
        </h2>
      </div>
    </div>

    <!-- 列表区 -->
    <el-scrollbar v-loading="loading" element-loading-background="rgba(0,0,0,0.05)">
      <div class="p-4 space-y-4">
        <div
          v-for="(item, index) in state_stack"
          :key="item.id"
          class="group relative flex gap-4 transition-all duration-300"
        >
          <!-- 时间线/连接线 -->
          <div class="flex flex-col items-center">
            <div
              class="w-8 h-8 rounded-lg bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 flex items-center justify-center text-xs font-mono font-bold text-primary-600 shadow-sm group-hover:border-primary-500 transition-colors"
            >
              {{ item.pos.toString().padStart(2, '0') }}
            </div>
            <div
              v-if="index !== state_stack.length - 1"
              class="w-px h-full bg-slate-200 dark:bg-slate-800 my-1"
            ></div>
          </div>

          <!-- 卡片内容 -->
          <div class="flex-1 min-w-0">
            <div
              class="bg-white dark:bg-slate-800/80 border border-slate-200 dark:border-slate-700 rounded-xl overflow-hidden hover:shadow-lg hover:shadow-primary-500/5 transition-all duration-300"
            >
              <!-- 卡片头部 -->
              <div
                class="p-3 cursor-pointer flex items-center justify-between"
                @click="toggleExpand(item.id)"
              >
                <div class="flex-1 min-w-0 select-none">
                  <h3 class="text-sm font-bold text-slate-700 dark:text-slate-200 truncate">
                    {{ item.title }}
                  </h3>
                  <p class="text-[10px] text-slate-400 font-mono uppercase mt-0.5">
                    {{ item.time }}
                  </p>
                </div>
                <div class="flex items-center gap-2">
                  <el-button link :icon="View" @click.stop="openPreview(item)"></el-button>
                  <el-icon
                    class="text-slate-400 transition-transform duration-300"
                    :class="{ 'rotate-180': expandedKeys.includes(item.id) }"
                  >
                    <ArrowDown />
                  </el-icon>
                </div>
              </div>

              <!-- 折叠详细内容 (Inline) -->
              <div
                v-show="expandedKeys.includes(item.id)"
                class="px-3 pb-3 border-t border-slate-100 dark:border-slate-700/50 animate-fade-in"
              >
                <div
                  class="mt-2 text-xs text-slate-600 font-mono dark:text-slate-400 leading-relaxed bg-slate-50 dark:bg-slate-900/50 p-2 rounded-lg border-l-2 border-slate-200 dark:border-slate-700 whitespace-pre-wrap"
                >
                  {{ item.content || 'No content provided.' }}
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 空状态 -->
        <el-empty
          v-if="!loading && state_stack.length === 0"
          :image-size="60"
          description="No State Data"
        />
      </div>
    </el-scrollbar>

    <!-- 详情弹窗预览 -->
    <el-dialog
      v-model="previewVisible"
      :title="`State Detail: ${previewData?.title}`"
      width="60%"
      destroy-on-close
      append-to-body
    >
      <div class="space-y-4">
        <div class="flex gap-4">
          <el-tag>POS: {{ previewData?.pos }}</el-tag>
          <el-tag type="info">{{ previewData?.time }}</el-tag>
        </div>
        <div
          class="p-4 bg-slate-900 text-green-400 font-mono text-sm rounded-lg border border-green-900/30 min-h-[200px] whitespace-pre-wrap"
        >
          <span class="opacity-50 mr-2">$ cat content.log</span><br />
          {{ previewData?.content }}
        </div>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { ArrowDown, View } from '@element-plus/icons-vue'

import * as module from '@/api/modules'

const props = defineProps<{
  agent: module.agent.Entity
  state_stack: module.state.Entity[]
}>()

const loading = ref(false)
const expandedKeys = ref<string[]>([])

// --- 交互逻辑 ---
const toggleExpand = (id: string) => {
  const index = expandedKeys.value.indexOf(id)
  if (index > -1) {
    expandedKeys.value.splice(index, 1)
  } else {
    expandedKeys.value.push(id)
  }
}

// 弹窗预览逻辑
const previewVisible = ref(false)
const previewData = ref<module.state.Entity | null>(null)

const openPreview = (item: module.state.Entity) => {
  previewData.value = item
  previewVisible.value = true
}
</script>

<style scoped>
.animate-fade-in {
  animation: fadeIn 0.3s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 弹窗科幻风格定制 */
:deep(.sci-fi-dialog) {
  border-radius: 16px;
  background: rgba(255, 255, 255, 0.9);
  backdrop-filter: blur(10px);
}

:deep(.dark .sci-fi-dialog) {
  background: rgba(15, 23, 42, 0.9);
  border: 1px solid rgba(51, 65, 85, 1);
}

:deep(.el-dialog__header) {
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
  margin-right: 0;
}

:deep(.dark .el-dialog__header) {
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}
</style>
