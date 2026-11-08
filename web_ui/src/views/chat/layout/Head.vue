<template>
  <header
    class="h-16 relative border-b border-slate-200 bg-white dark:bg-[#0f111a] dark:border-slate-800 flex items-center justify-between px-4 transition-colors duration-300"
  >
    <!-- 左侧 -->
    <div class="flex flex-row items-center gap-2.5 mr-auto">
      <!-- 1. Agent 选择器 -->
      <el-select
        v-model="agentStore.last_agent_id"
        placeholder="请选择 Agent"
        style="min-width: 150px"
        @change="handleAgentChange"
        :key="agentStore.agent_list.length"
      >
        <template #label>
          <div class="flex flex-row">
            <el-avatar :size="24" :src="currentAgent.avatar" shape="square" />
            <span class="ml-2">{{ currentAgent.name }}</span>
          </div>
        </template>
        <el-option v-for="agent in agentStore.agent_list" :key="agent.id" :value="agent.id">
          <el-avatar :size="24" :src="agent.avatar" shape="square" />
          <span class="ml-2">{{ agent.name }}</span>
        </el-option>
      </el-select>
      <div class="flex font-bold text-[18px] w-20">控制台</div>
    </div>
    <!--    中间-->
    <div class="absolute inset-x-0 flex justify-center">
      <div class="flex flex-row items-center gap-3">
        <!-- 2. 状态 & 计时器 -->
        <div
          class="flex items-center h-8 rounded-md border border-slate-200 bg-slate-50 dark:border-[#2b303b] dark:bg-[#1e222b] transition-colors overflow-hidden"
        >
          <!-- 运行状态 -->
          <div class="flex items-center gap-2 pl-3 pr-4 h-full">
            <!-- 自定义状态环 SVG -->
            <svg viewBox="0 0 16 16" fill="none" class="w-3.5 h-3.5" :class="statusColor">
              <circle cx="8" cy="8" r="5" stroke="currentColor" stroke-width="2.5" />
              <circle cx="8" cy="8" r="2" fill="currentColor" />
            </svg>
            <span class="text-[13px] font-medium" :class="statusColor">
              {{ statusText }}
            </span>
          </div>
          <!-- 分隔线 -->
          <div class="w-[1px] h-4 bg-slate-200 dark:bg-[#2b303b]"></div>
          <!-- 计时器 -->
          <div
            class="flex items-center gap-1.5 pl-3 pr-4 h-full text-slate-600 dark:text-slate-300"
          >
            <SvgIcon name="clock" class="text-sm opacity-80" />
            <span class="text-[13px] font-mono tracking-wide tabular-nums mt-[1px]">
              {{ duration }}
            </span>
          </div>
        </div>

        <!-- 3. 暂停/继续 按钮 -->
        <button
          @click="togglePause"
          class="flex items-center gap-2 h-8 px-3 rounded-md border border-slate-200 bg-slate-50 hover:bg-slate-100 dark:border-[#2b303b] dark:bg-[#1e222b] dark:hover:bg-[#252a35] transition-all"
        >
          <svg-icon name="alert" class="w-3.5 h-3.5 text-orange-500" />
          <span class="text-[13px] font-medium text-slate-700 dark:text-slate-200 mt-[1px]">
            打断
          </span>
        </button>

        <!-- 4. 停止按钮 -->
        <button
          @click="stopTask"
          class="flex items-center gap-2 h-8 px-3 rounded-md border border-slate-200 bg-slate-50 hover:bg-red-50 hover:border-red-200 dark:border-[#2b303b] dark:bg-[#1e222b] dark:hover:bg-[#322024] dark:hover:border-red-900/50 transition-all group"
        >
          <!-- 圆角正方形停止 SVG -->
          <svg-icon
            name="cube"
            class="w-[11px] h-[11px] text-[#f44336] group-hover:scale-110 transition-transform"
          />
          <span
            class="text-[13px] font-medium text-slate-700 dark:text-slate-200 group-hover:text-[#f44336] transition-colors mt-[1px]"
          >
            停止
          </span>
        </button>
      </div>
    </div>

    <!-- 右侧：设置 & 主题 (保留原样) -->
    <div class="flex items-center gap-3">
      <button
        @click="toggleTheme"
        class="p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors text-slate-500 dark:text-slate-400"
      >
        <el-icon size="20">
          <Moon v-if="isDark" />
          <Sunny v-else />
        </el-icon>
      </button>

      <div class="w-[1px] h-5 bg-slate-200 dark:bg-slate-700 mx-1"></div>

      <button
        @click="showSettings = true"
        class="flex items-center gap-2 px-3 py-1.5 rounded-md bg-primary-600 hover:bg-primary-500 transition-all shadow-md shadow-primary-500/20"
      >
        <el-icon size="16" class="animate-spin-slow">
          <Setting />
        </el-icon>
        <span class="text-sm font-medium">Settings</span>
      </button>
    </div>
  </header>
  <!-- Agent 设置弹窗 -->
  <el-dialog v-model="showSettings" title="Agent Configuration" width="500px">
    <!-- 弹窗内容保留 -->
    <div class="space-y-4">
      <div class="flex items-center gap-4 p-4 bg-slate-50 dark:bg-slate-800 rounded-xl">
        <el-avatar :size="60" :src="currentAgent.avatar" />
        <div>
          <h3 class="font-bold text-lg text-slate-800 dark:text-slate-100">
            {{ currentAgent.name }}
          </h3>
          <p class="text-sm text-slate-500">ID: {{ currentAgent.id }}</p>
        </div>
      </div>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { Moon, Setting, Sunny } from '@element-plus/icons-vue'
import { dayjs, ElMessage } from 'element-plus'
import { type AgentInfo, useAgentStore } from '@/stores/agent.ts'

import * as module from '@/api/modules/'
import SvgIcon from '@/components/SvgIcon.vue'
import { useTimer } from '@/hooks/timer.ts'
import { AgentStatusEnum } from '@/api/modules/enums.ts'

const agentStore = useAgentStore()
const props = defineProps<{
  agent: AgentInfo
}>()
const currentAgent = computed<module.agent.Entity>(() => props.agent.data)

// 状态: running(运行中) | paused(暂停) | stopped(已停止)
const status = computed(() => currentAgent.value.status || AgentStatusEnum.Idle)
const showSettings = ref(false)
const isDark = ref(false) // 默认白天主题

// --- 计时器逻辑 ---
const currentTime = dayjs()
const seconds_offset = computed(() => {
  const startTime = dayjs(props.agent.data.startup_time)
  return currentTime.diff(startTime, 'second')
})
const { duration, pause } = useTimer(
  () => props.agent.data.status == AgentStatusEnum.Paused,
  ms => {
    const sec = seconds_offset.value + ((ms / 1000) | 0)
    const h = Math.floor(sec / 3600)
      .toString()
      .padStart(2, '0')
    const m = Math.floor((sec % 3600) / 60)
      .toString()
      .padStart(2, '0')
    const s = (sec % 60).toString().padStart(2, '0')
    return `${h}:${m}:${s}`
  },
)
const stopTimer = () => {
  pause()
}

// --- 计算属性：状态文本 & 颜色 ---
const statusText = computed(() => {
  if (status.value === AgentStatusEnum.Idle) return '空闲'
  if (status.value === AgentStatusEnum.Sleep) return '睡眠中'
  if (status.value === AgentStatusEnum.Processing) return '运行中'
  return '已停止'
})

const statusColor = computed(() => {
  if (status.value === AgentStatusEnum.Idle) return 'text-amber-500'
  if (status.value === AgentStatusEnum.Sleep) return 'text-amber-500'
  if (status.value === AgentStatusEnum.Processing) return 'text-teal-500' // 青色匹配截图
  return 'text-slate-400'
})

// --- 方法 ---
const handleAgentChange = (agent: module.agent.Entity) => {
  agentStore.last_agent_id = agent.id
}

const togglePause = async () => {
  await module.agent.user_interrupt(currentAgent.value.id).then(() => {})
}

const stopTask = () => {
  if (status.value === AgentStatusEnum.Paused) return
  stopTimer()
  ElMessage.error('系统已停止运行')
}

const toggleTheme = () => {
  isDark.value = !isDark.value
  if (isDark.value) {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }
}
</script>

<style scoped>
.animate-spin-slow {
  animation: spin 6s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

/* 覆盖Element Plus默认颜色，使其适配 Tailwind */
:deep(.el-button--primary) {
  --el-button-bg-color: #2563eb;
  --el-button-border-color: #2563eb;
  --el-button-hover-bg-color: #3b82f6;
  --el-button-hover-border-color: #3b82f6;
}
</style>
