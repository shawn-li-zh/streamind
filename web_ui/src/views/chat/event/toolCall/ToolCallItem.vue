<template>
  <div class="my-3 max-w-full">
    <!-- 容器 -->
    <div
      class="group rounded-xl border transition-all duration-300 overflow-hidden shadow-sm"
      :class="statusClasses[status]"
      :key="version"
    >
      <!-- 头部区域 -->
      <div
        class="flex items-center justify-between p-3 cursor-pointer select-none"
        @click="isExpanded = !isExpanded"
      >
        <div class="flex items-center gap-3 flex-1 min-w-0">
          <!-- 状态图标 -->
          <div class="relative flex items-center justify-center w-6 h-6">
            <el-icon v-if="status === 'active'" class="is-loading text-blue-500 animate-spin">
              <Loading />
            </el-icon>
            <el-icon v-else-if="status === 'success'" class="text-emerald-500 font-bold">
              <CircleCheck />
            </el-icon>
            <el-icon v-else-if="status === 'failure'" class="text-rose-500">
              <CircleClose />
            </el-icon>
          </div>

          <!-- 标题信息 -->
          <div class="flex flex-col flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <span class="text-sm font-semibold tracking-tight text-gray-800">
                {{ status === 'active' ? '正在执行工具...' : '工具执行完毕' }}
              </span>
              <!-- 计时标签 -->
              <span
                class="text-[10px] px-1.5 py-0.5 rounded-full font-mono font-medium"
                :class="
                  status === 'active'
                    ? 'bg-blue-100 text-blue-600 animate-pulse'
                    : 'bg-gray-100 text-gray-500'
                "
              >
                {{ formattedTime }}
              </span>
            </div>
            <div class="text-[11px] text-gray-400 font-mono flex gap-2">
              <span>ID: {{ detail.id || '' }}</span>
              <span v-if="status !== 'active'" class="text-gray-300">|</span>
              <span v-if="status === 'success'" class="text-emerald-600/70">Success</span>
              <span v-else-if="status === 'failure'" class="text-rose-600/70">Failed</span>
            </div>
          </div>
        </div>

        <!-- 折叠箭头 -->
        <div class="flex items-center ml-2">
          <div class="p-1 rounded-md hover:bg-black/5 transition-colors">
            <el-icon
              class="transition-transform duration-300"
              :class="{ 'rotate-180': isExpanded }"
            >
              <ArrowDown />
            </el-icon>
          </div>
        </div>
      </div>

      <!-- 详情展开区 -->
      <transition
        @enter="el => (el.style.height = el.scrollHeight + 'px')"
        @before-leave="el => (el.style.height = el.scrollHeight + 'px')"
        @leave="el => (el.style.height = '0')"
      >
        <div
          v-if="isExpanded"
          class="overflow-hidden transition-[height] duration-300 ease-in-out border-t border-inherit bg-white/30"
        >
          <div class="p-3">
            <div class="flex items-center justify-between mb-2">
              <span class="text-[11px] uppercase tracking-wider text-gray-400 font-bold"
                >Output Result</span
              >
              <button @click.stop="copyResult" class="text-[11px] text-blue-600 hover:underline">
                Copy
              </button>
            </div>
            <div
              class="text-xs font-mono bg-gray-900 text-gray-100 rounded-lg p-3 overflow-x-auto max-h-[300px] shadow-inner"
            >
              <template v-if="detail.content">
                <pre class="whitespace-pre-wrap break-all leading-relaxed">{{
                  formatResult(detail.content)
                }}</pre>
              </template>
              <template v-else>
                <div class="flex items-center gap-2 text-gray-500 italic">
                  <div class="w-1 h-1 bg-gray-500 rounded-full animate-bounce"></div>
                  等待数据流回传...
                </div>
              </template>
            </div>
          </div>
        </div>
      </transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onUnmounted, ref } from 'vue'
import { ArrowDown, CircleCheck, CircleClose, Loading } from '@element-plus/icons-vue'
import * as module from '@/api/modules'
import { ElMessage } from 'element-plus'

import { event } from '@/api/modules/index.ts'
import type { ToolCallEventPayload } from '@/views/chat/type.ts'
import { ToolCallStatus } from '@/api/modules/enums.ts'

const props = defineProps<{
  section:module.section.Entity
}>()

const detail = ref<event.ToolCallDetail>(props.section.payload || ({} as any))
const status = computed(() => detail.value.status || ToolCallStatus.Undo)
const version = ref(0)
const isExpanded = ref(false)

// 计时器相关
const startTime = ref<number>(performance.now())
const duration = ref<number>(0)
let timerInterval: any = null

const formattedTime = computed(() => {
  return (duration.value / 1000).toFixed(2) + 's'
})

function startTimer() {
  if (timerInterval) return
  startTime.value = performance.now()
  timerInterval = setInterval(() => {
    duration.value = performance.now() - startTime.value
  }, 50)
}

function stopTimer() {
  if (timerInterval) {
    clearInterval(timerInterval)
    timerInterval = null
    duration.value = performance.now() - startTime.value
  }
}

// 样式映射
const statusClasses: Record<string, string> = {
  active: 'bg-blue-50/30 border-blue-100 shadow-blue-50',
  success: 'bg-emerald-50/20 border-emerald-100 shadow-emerald-50',
  failure: 'bg-rose-50/20 border-rose-100 shadow-rose-50',
}

function onEvent(payload: ToolCallEventPayload) {
  version.value++
  switch (payload.type) {
    case 'active':
      startTimer()
      detail.value.status = ToolCallStatus.Active
      break
    case 'success':
      stopTimer()
      detail.value.status = ToolCallStatus.Success
      detail.value.content = payload.data.content
      detail.value.time = payload.data.time
      detail.value.args = payload.data.args
      break
    case 'failure':
      stopTimer()
      detail.value.status = ToolCallStatus.Failure
      detail.value.content = payload.data.content || '未知错误'
      detail.value.time = payload.data.time
      detail.value.args = payload.data.args
      break
  }
}

function formatResult(val: any) {
  if (typeof val !== 'string') return JSON.stringify(val, null, 2)
  try {
    const obj = JSON.parse(val)
    return JSON.stringify(obj, null, 2)
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
  } catch (_: any) {
    return val
  }
}

function copyResult() {
  navigator.clipboard.writeText(detail.value.content || '')
  ElMessage.success('已复制到剪贴板')
}

onUnmounted(() => stopTimer())

defineExpose({ onEvent })
</script>

<style scoped>
.rotate-180 {
  transform: rotate(180deg);
}

pre {
  font-family: 'Fira Code', 'Cascadia Code', Monaco, Consolas, monospace;
}
</style>
