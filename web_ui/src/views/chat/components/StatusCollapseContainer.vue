<template>
  <div class="mb-4 border border-slate-200 dark:border-slate-700 rounded-xl overflow-hidden bg-white/50 dark:bg-slate-900/50 backdrop-blur-sm shadow-sm transition-all duration-300">

    <!-- 上部分：头部信息 -->
    <div class="flex items-center justify-between px-4 py-3 bg-slate-50/80 dark:bg-slate-800/80 border-b border-slate-200 dark:border-slate-700">
      <div class="flex items-center gap-3">
        <!-- 动态图标 -->
        <div :class="statusConfig.color">
          <el-icon :size="20" :class="statusConfig.animation">
            <component :is="statusConfig.icon" />
          </el-icon>
        </div>

        <!-- 标题 -->
        <h3 class="text-sm font-bold text-slate-700 dark:text-slate-200 tracking-tight uppercase">
          {{ title }}
        </h3>
      </div>

      <div class="flex items-center gap-4">
        <!-- 计时器 -->
        <div class="flex items-center gap-2 font-mono text-xs font-bold px-2 py-1 bg-slate-200/50 dark:bg-slate-700/50 rounded border border-slate-300 dark:border-slate-600">
          <span class="text-[10px] text-slate-400">DURATION</span>
          <span :class="props.time ? 'text-slate-500' : 'text-blue-500 animate-pulse'">
            {{ displayTime }}
          </span>
        </div>

        <!-- 折叠控制按钮 -->
        <button
          @click="isExpanded = !isExpanded"
          class="p-1 hover:bg-slate-200 dark:hover:bg-slate-700 rounded transition-colors text-slate-500"
        >
          <el-icon :class="['transition-transform duration-300', isExpanded ? 'rotate-180' : '']">
            <ArrowDown />
          </el-icon>
        </button>
      </div>
    </div>

    <!-- 下部分：可折叠区域 -->
    <div
      class="transition-all duration-300 ease-in-out"
      :style="{ maxHeight: isExpanded ? '1000px' : '60px' }"
    >
      <div class="p-4 relative">
        <!-- 折叠时的遮罩渐变 (只在折叠时显示) -->
        <div
          v-if="!isExpanded"
          class="absolute inset-x-0 bottom-0 h-8 bg-gradient-to-t from-white/90 dark:from-slate-900/90 to-transparent pointer-events-none"
        ></div>

        <!-- 内容槽位 -->
        <div :class="['text-sm leading-relaxed text-slate-600 dark:text-slate-300', !isExpanded ? 'line-clamp-2' : '']">
          <slot></slot>
        </div>
      </div>
    </div>

    <!-- 底部状态条 (科幻装饰) -->
    <div class="h-[2px] w-full bg-slate-100 dark:bg-slate-800">
      <div
        :class="['h-full transition-all duration-1000', statusConfig.barColor]"
        :style="{ width: props.time ? '100%' : '30%' }"
      ></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { Loading, CircleCheck, Warning, InfoFilled, ArrowDown } from '@element-plus/icons-vue'

interface Props {
  title: string
  status: 'processing' | 'success' | 'error' | 'info'
  time?: string | number | null // 如果为空则开始计时，如果有值则显示该值并停止计时
}

const props = withDefaults(defineProps<Props>(), {
  status: 'info',
  time: null
})

const isExpanded = ref(false)
const elapsedSeconds = ref(0)
let timerInterval: number | null = null

// --- 计时器逻辑 ---
const startTimer = () => {
  if (timerInterval) return
  timerInterval = window.setInterval(() => {
    elapsedSeconds.value++
  }, 1000)
}

const stopTimer = () => {
  if (timerInterval) {
    clearInterval(timerInterval)
    timerInterval = null
  }
}

const formatSeconds = (s: number) => {
  const m = Math.floor(s / 60).toString().padStart(2, '0')
  const sec = (s % 60).toString().padStart(2, '0')
  return `${m}:${sec}s`
}

const displayTime = computed(() => {
  // 如果外部传入了 time (比如后端返回的最终耗时)，直接显示
  if (props.time) {
    return typeof props.time === 'number' ? formatSeconds(props.time) : props.time
  }
  // 否则显示实时计时的秒数
  return formatSeconds(elapsedSeconds.value)
})

// 监听 props.time，一旦有值就停止计时
watch(() => props.time, (newVal) => {
  if (newVal) {
    stopTimer()
  } else {
    startTimer()
  }
}, { immediate: true })

// --- 状态配置 ---
const statusConfig = computed(() => {
  const configs = {
    processing: {
      icon: Loading,
      color: 'text-blue-500',
      animation: 'animate-spin',
      barColor: 'bg-blue-500 shadow-[0_0_8px_#3b82f6]'
    },
    success: {
      icon: CircleCheck,
      color: 'text-green-500',
      animation: '',
      barColor: 'bg-green-500'
    },
    error: {
      icon: Warning,
      color: 'text-red-500',
      animation: 'animate-pulse',
      barColor: 'bg-red-500'
    },
    info: {
      icon: InfoFilled,
      color: 'text-slate-400',
      animation: '',
      barColor: 'bg-slate-400'
    }
  }
  return configs[props.status] || configs.info
})

onUnmounted(() => {
  stopTimer()
})
</script>

<style scoped>
/* 自定义行数限制（针对老旧浏览器兼容） */
.line-clamp-2 {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow: hidden;
}
</style>
