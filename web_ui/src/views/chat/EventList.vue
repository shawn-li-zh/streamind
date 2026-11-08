<template>
  <!-- 主容器：控制主题的 class -->
  <div class="bg-gray-50 dark:bg-[#0f111a] transition-colors duration-300">
    <!-- 卡片容器 -->
    <div
      class="flex flex-col bg-white dark:bg-[#161b26] shadow-sm border border-gray-200 dark:border-gray-800 h-full"
    >
      <!-- 头部 Header -->
      <div class="p-3">
        <div class="flex justify-between items-center mb-3">
          <h2 class="text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400">
            事件消息队列
          </h2>

          <div class="flex gap-2">
            <!-- 刷新按钮 -->
            <el-button type="primary" :loading="loading" @click="handleRefresh" plain size="small">
              <el-icon class="mr-1">
                <Refresh />
              </el-icon>
              刷新
            </el-button>
          </div>
        </div>

        <!-- 状态筛选 Tabs -->
        <div class="flex flex-wrap gap-2 mb-6">
          <button
            v-for="tab in filterTabs"
            :key="tab.key"
            @click="activeTab = tab.key"
            :class="[
              'px-3 py-1 rounded-lg flex items-center gap-1 transition-all border',
              activeTab === tab.key
                ? 'bg-blue-50 border-blue-200 text-blue-600 dark:bg-blue-900/30 dark:border-blue-800 dark:text-blue-400'
                : 'bg-gray-100 border-transparent text-gray-500 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-400',
            ]"
          >
            <span class="text-sm font-medium">{{ tab.label }}</span>
            <span
              class="text-xs px-1 py-0.5 rounded-full bg-white/50 dark:bg-black/20"
              :class="tab.countColor"
              >{{ tab.count }}</span
            >
          </button>
        </div>
      </div>

      <!-- 表格区域 -->
      <div class="flex-1 overflow-hidden px-2">
        <el-table
          :data="eventItems"
          v-loading="loading"
          style="width: 100%"
          height="300px"
          size="small"
          :header-cell-style="{ background: 'transparent', color: '#888', fontWeight: '400' }"
          class="custom-table"
        >
          <!-- 事件名称列 -->
          <el-table-column label="事件" min-width="130">
            <template #default="{ row }">
              <div class="flex items-center gap-3">
                <!-- 图标占位 -->
                <div
                  class="w-8 h-8 rounded-lg bg-gray-100 dark:bg-gray-800 flex items-center justify-center text-gray-500"
                >
                  <svg-icon
                    :name="typeIcons[row.event_type as EventTypeEnum]"
                    class="text-[14px]"
                  />
                </div>
                <div class="flex flex-col">
                  <span class="font-bold text-gray-700 dark:text-gray-200">{{
                    typeLabels[row.event_type as EventTypeEnum]
                  }}</span>
                  <span class="text-xs text-gray-400">{{ row.description }}</span>
                </div>
              </div>
            </template>
          </el-table-column>
          <!-- 状态列 -->
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <span :class="getStatusClass(row.status)">
                {{ statusLabels[row.status as EventStatusEnum] }}
              </span>
            </template>
          </el-table-column>

          <!-- 时间列 -->
          <el-table-column label="时间" width="150" align="right">
            <template #default="{ row }">
              <span class="text-[10px] text-gray-400 font-mono">{{ row.time }}</span>
            </template>
          </el-table-column>
        </el-table>
      </div>

      <!-- 分页 -->
      <div class="p-2 flex justify-center">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :total="activeTab ? statusCount[activeTab as EventStatusEnum] || 0 : totalCount"
          @change="list"
          layout="prev, pager, next"
          background
          size="small"
          class="dark-pagination"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Refresh } from '@element-plus/icons-vue'
import * as modules from '@/api/modules'
import { type EventItem } from '@/api/modules/event.ts'
import { EventStatusEnum, EventTypeEnum } from '@/api/modules/enums.ts'
import SvgIcon from '@/components/SvgIcon.vue'
import type { AgentInfo } from '@/stores/agent.ts'
import { useDebounce } from '@/hooks/limiting.ts'

const props = defineProps<{
  agent: AgentInfo
}>()

// 状态管理

const loading = ref(false)
const activeTab = ref<EventStatusEnum>()
const currentPage = ref(1)
const pageSize = ref(6)

// 模拟数据
const totalCount = ref(0)
const statusCount = ref<Record<EventStatusEnum, number>>({} as any)
const eventItems = ref<EventItem[]>([])
// onMounted(async () => {
//   await handleRefresh()
// })
watch(
  () => props.agent.connected,
  v => {
    if (v) handleRefresh()
  },
)

async function counts() {
  const { data } = await modules.event.count_by_status(props.agent.agent_id)
  totalCount.value = 0
  statusCount.value = {} as any
  for (const datum of data) {
    statusCount.value[datum.status] = datum.count
    totalCount.value += datum.count
  }
}

async function list() {
  const { data } = await modules.event.list_items(
    props.agent.agent_id,
    currentPage.value,
    pageSize.value,
    activeTab.value as any,
  )
  eventItems.value = data
}

watch(
  () => activeTab.value,
  () => {
    currentPage.value = 1
    return list()
  },
)
const fn = useDebounce(async () => {
  await counts()
  await list()
}, 500)
watch(
  () => props.agent.step,
  (n, o) => {
    if (n > o) {
      fn()
    }
  },
)
const statusLabels: Record<EventStatusEnum, string> = {
  [EventStatusEnum.Pending]: '待处理',
  [EventStatusEnum.Processing]: '处理中',
  [EventStatusEnum.Done]: '已完成',
  [EventStatusEnum.Unloaded]: '已卸载',
}
const typeLabels: Record<EventTypeEnum, string> = {
  [EventTypeEnum.AgentResponse]: '响应',
  [EventTypeEnum.ToolResponse]: '工具结果',
  [EventTypeEnum.Error]: '系统错误',
  [EventTypeEnum.UserRequest]: '用户请求',
  [EventTypeEnum.Interrupt]: '输出中断',
  [EventTypeEnum.Interaction]: 'UI交互',
  [EventTypeEnum.Timer]: '定时任务',
  [EventTypeEnum.Heartbeat]: '心跳',
  [EventTypeEnum.Reflection]: '条件反射',
}
const typeIcons: Record<EventTypeEnum, string> = {
  [EventTypeEnum.AgentResponse]: '',
  [EventTypeEnum.ToolResponse]: 'tool',
  [EventTypeEnum.Error]: 'error',
  [EventTypeEnum.UserRequest]: 'user',
  [EventTypeEnum.Interrupt]: 'interrupt',
  [EventTypeEnum.Interaction]: 'click',
  [EventTypeEnum.Timer]: 'clock',
  [EventTypeEnum.Heartbeat]: 'heartbeat',
  [EventTypeEnum.Reflection]: 'alert',
}
// 统计信息
const filterTabs = computed(() => [
  { key: undefined, label: '全部', count: totalCount.value, countColor: 'text-blue-500' },
  {
    key: EventStatusEnum.Pending,
    label: '待处理',
    count: statusCount.value[EventStatusEnum.Pending] || 0,
    countColor: 'text-orange-500',
  },
  {
    key: EventStatusEnum.Processing,
    label: '处理中',
    count: statusCount.value[EventStatusEnum.Processing] || 0,
    countColor: 'text-blue-400',
  },
  {
    key: EventStatusEnum.Done,
    label: '已完成',
    count: statusCount.value[EventStatusEnum.Done] || 0,
    countColor: 'text-green-500',
  },
  {
    key: EventStatusEnum.Unloaded,
    label: '已卸载',
    count: statusCount.value[EventStatusEnum.Unloaded] || 0,
    countColor: 'text-gray-300',
  },
])

// 样式处理函数
const getStatusClass = (status: EventStatusEnum) => {
  const base = 'px-2 py-1 rounded text-xs font-medium '
  switch (status) {
    case EventStatusEnum.Pending:
      return base + 'bg-orange-100 text-orange-600 dark:bg-orange-900/30 dark:text-orange-400'
    case EventStatusEnum.Processing:
      return base + 'bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400'
    case EventStatusEnum.Done:
      return base + 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400'
    default:
      return base + 'bg-gray-100 text-gray-600'
  }
}

const handleRefresh = async () => {
  loading.value = true
  await counts()
  await list()
  loading.value = false
}
</script>

<style>
/* Element Plus 暗色模式适配 */
.dark .el-table {
  --el-table-bg-color: transparent;
  --el-table-tr-bg-color: transparent;
  --el-table-header-bg-color: transparent;
  --el-table-border-color: #1f2937;
  --el-table-text-color: #d1d5db;
  --el-table-row-hover-bg-color: #1f2937;
}

.dark .el-pagination {
  --el-pagination-bg-color: transparent;
  --el-pagination-button-bg-color: #1f2937;
  --el-pagination-button-color: #9ca3af;
}

/* 去除表格边框线，模仿图片简约感 */
.custom-table .el-table__inner-wrapper::before {
  display: none;
}

.el-table td.el-table__cell,
.el-table th.el-table__cell.is-leaf {
  border-bottom: 1px solid rgba(128, 128, 128, 0.1) !important;
}
</style>
