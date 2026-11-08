<template>
  <ReactionCard
    :title="info.title"
    :content="info.content"
    :elapsed="detail.elapsed"
    :running="detail.status === 'active' || detail.status === 'undo'"
    content-class="bg-slate-900 text-green-400"
  >
    <template #logo>
      <SvgIcon name="tool" class="text-orange-500" />
    </template>
    <template #title="{ title }">
      <div class="flex flex-row space-x-2">
        <!-- 标题 -->
        <span class="text-[15px] font-semibold text-gray-800">{{ title }}</span>
        <div
          class="font-mono text-[11px] font-medium px-1 py-0.5 rounded transition-colors"
          :class="statusCss.tag"
        >
          {{ detail.status }}
        </div>
      </div>
    </template>
    <template #content="{ content }">
      <PlayContent :content="content!" class="bg-slate-900 text-green-400 text-[11px] w-auto" />
    </template>

    <template #details>
      <div class="px-4 pb-4 space-y-3 overflow-hidden">
        <!-- Arguments -->
        <div class="space-y-1">
          <span class="text-[10px] font-bold text-slate-400 uppercase ml-1">Args</span>
          <!-- 同样为 Args 增加了 max-h 限制，防止参数过多撑爆页面 -->
          <pre
            class="text-xs p-3 rounded-lg bg-slate-100 dark:bg-slate-800 text-blue-600 dark:text-blue-400 overflow-auto max-h-[150px] border border-slate-200 dark:border-slate-700"
          ><code>{{ JSON.stringify(detail.args, null, 2) }}</code></pre>
        </div>

        <!-- Result / Content (Streaming) -->
        <div v-if="detail.values?.length" class="space-y-1">
          <div class="flex items-center justify-between">
            <span class="text-[10px] font-bold text-slate-400 uppercase ml-1">Output</span>
            <el-button link type="primary" class="!text-[10px]" @click.stop="copyResult()">
              Copy
            </el-button>
          </div>

          <!-- 核心修改部分 -->
          <div
            class="relative rounded-lg bg-slate-900 p-3 font-mono text-xs text-green-400 shadow-inner
             overflow-auto max-h-[300px] scrollbar-thin scrollbar-thumb-slate-700"
          >
            <!-- 使用 whitespace-pre-wrap 允许长行换行，或者保持原样配合 overflow-auto -->
            <pre class="whitespace-pre-wrap break-all"><code>{{ output || 'Executing remote procedure...' }}</code><span v-if="detail.status === 'active'" class="animate-pulse ml-1">_</span></pre>
          </div>
        </div>
      </div>
    </template>
  </ReactionCard>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import { ElMessage } from 'element-plus'

import { event } from '@/api/modules/index.ts'
import { ToolCallStatus } from '@/api/modules/enums.ts'
import * as module from '@/api/modules'
import ReactionCard from '@/views/chat/components/ReactionCard.vue'
import SvgIcon from '@/components/SvgIcon.vue'
import { getToolConfig } from '@/views/chat/section/tool.ts'
import PlayContent from '@/components/PlayContent.vue'

const props = defineProps<{
  section: module.section.Entity
}>()

const detail = computed<event.ToolCallDetail>(() => {
  return props.section.payload || ({} as any)
})

const info = computed(() => {
  const { package: packageName, tool } = detail.value
  const config = getToolConfig(packageName, tool)
  const content = config.explain(detail.value.args)
  return { content, title: config.title }
})
const statusCss = computed(() => {
  return statusConfig[detail.value.status] || statusConfig[ToolCallStatus.Undo]
})
const statusConfig: any = {
  [ToolCallStatus.Undo]: {
    container: 'border-slate-200 bg-slate-50 dark:border-slate-700 dark:bg-slate-800/30',
    iconBg: 'bg-slate-400',
    tag: 'bg-slate-100 text-slate-500 border-slate-200',
  },
  [ToolCallStatus.Active]: {
    container:
      'border-blue-200 bg-blue-50/30 dark:border-blue-900/50 dark:bg-blue-900/10 shadow-[0_0_15px_rgba(59,130,246,0.1)]',
    iconBg: 'bg-blue-500 animate-pulse',
    tag: 'bg-blue-100 text-blue-600 border-blue-200 animate-pulse',
  },
  [ToolCallStatus.Success]: {
    container:
      'border-emerald-200 bg-emerald-50/20 dark:border-emerald-900/40 dark:bg-emerald-900/5',
    iconBg: 'bg-emerald-500',
    tag: 'text-emerald-600 border-emerald-200',
  },
  [ToolCallStatus.Failure]: {
    container: 'border-red-200 bg-red-50/20 dark:border-red-900/40 dark:bg-red-900/5',
    iconBg: 'bg-red-500',
    tag: 'text-red-600 border-red-200',
  },
  [ToolCallStatus.Timeout]: {
    container: 'border-amber-200 bg-amber-50/20 dark:border-amber-900/40 dark:bg-amber-900/5',
    iconBg: 'bg-amber-500',
    tag: 'text-amber-600 border-amber-200',
  },
}

function copyResult() {
  navigator.clipboard.writeText(output.value || '')
  ElMessage.success('已复制到剪贴板')
}

const output = computed(() => {
  return detail.value.values
    .map(item => {
      if (item.type == 'raw') return item.payload
      if (item.type == 'json') return JSON.stringify(item.payload, null, 2)
      if (item.type == 'image') return item.payload.path
    })
    .join('\n\n')
})
</script>

<style scoped>
.rotate-180 {
  transform: rotate(180deg);
}

pre {
  font-family: 'Fira Code', 'Cascadia Code', Monaco, Consolas, monospace;
}
</style>
