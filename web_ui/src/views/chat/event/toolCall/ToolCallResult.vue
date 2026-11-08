<template>
  <div
    class="p-4 flex flex-col space-y-2 bg-gray-50/50 rounded-2xl border border-gray-100 max-w-3xl w-8/12"
  >
    <!-- 头部统计 -->
    <div class="flex items-center justify-between px-1 mb-1">
      <div class="flex items-center gap-2">
        <div class="flex -space-x-2">
          <div
            v-for="i in Math.min(toolCallIds.length, 3)"
            :key="i"
            class="w-5 h-5 rounded-full bg-blue-500 border-2 border-white flex items-center justify-center text-[10px] text-white font-bold"
          >
            T
          </div>
        </div>
        <span class="text-xs font-bold text-gray-500 uppercase tracking-widest">
          Tool Engine
          <span class="ml-1 text-gray-300 font-normal">({{ toolCallIds.length }} calls)</span>
        </span>
      </div>
      <div v-if="isAnyActive" class="flex items-center gap-1.5">
        <span class="relative flex h-2 w-2">
          <span
            class="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75"
          ></span>
          <span class="relative inline-flex rounded-full h-2 w-2 bg-blue-500"></span>
        </span>
        <span class="text-[10px] text-blue-600 font-medium uppercase animate-pulse">Running</span>
      </div>
    </div>

    <!-- 列表区域 -->
    <div class="space-y-0 relative p-1 border-1 border-gray-100 m-1">
      <template v-for="id in toolCallIds" :key="id">
        <ToolCallItem :detail="id_map_tool_call_item[id]!.detail" :ref="el => setRef(id, el)" />
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import type { ToolCallEventPayload } from '@/views/chat/type.ts'
import ToolCallItem from './ToolCallItem.vue'
import { agent, event } from '@/api/modules/index.ts'
import { ToolCallStatus } from '@/api/modules/enums.ts'

const props = defineProps<{
  agentInfo: agent.Entity
  data: event.Entity
}>()

// 存储结构
interface ToolCallStorage {
  detail: event.ToolCallDetail
  ref: any
}

if (props.data.event_type != 'ToolCallResult') {
  throw new Error('Invalid event type')
}
const id_map_tool_call_item = ref<Record<string, ToolCallStorage>>({})
const toolCallIds = ref<string[]>([])
const details = ref<event.ToolCallDetail[]>(props.data.data || [])
const isAnyActive = computed(() => {
  // 简单判断是否有工具正在运行
  return toolCallIds.value.some(id => {
    const item = id_map_tool_call_item.value[id]
    // 这里如果想更精确，可以在 item 中存一个 status
    return item?.ref?.status === 'active'
  })
})

onMounted(() => {
  if (!props.data?.data) return

  // 初始化历史数据
  details.value.forEach(item => {
    id_map_tool_call_item.value[item.id] = {
      detail: item,
      ref: null,
    }
    toolCallIds.value.push(item.id)
  })
})

function setRef(id: string, el: any) {
  if (id_map_tool_call_item.value[id]) {
    id_map_tool_call_item.value[id].ref = el
  }
}

async function onEvent(payload: ToolCallEventPayload) {
  switch (payload.type) {
    case 'start': {
      toolCallIds.value = []
      const items = payload.data
      items.forEach(item => {
        id_map_tool_call_item.value[item.id] = {
          detail: {
            ...item,
            result: '',
            status: ToolCallStatus.Undo,
            time: '0',
          },
          ref: null,
        }
        toolCallIds.value.push(item.id)
      })
      await nextTick()
      break
    }
    case 'end':
      break
    default: {
      const id = payload.data.id
      const item = id_map_tool_call_item.value[id]
      if (item) {
        // 确保 DOM 已渲染且 ref 已挂载
        if (item.ref) {
          item.ref.onEvent(payload)
        } else {
          // 如果 ref 还没准备好，等待下一个周期
          await nextTick(() => item.ref?.onEvent(payload))
        }
      }
    }
  }
}

defineExpose({ onEvent })
</script>

<style scoped>
/* 自定义列表左侧的连接线样式 */
.border-l-2 {
  transition: border-color 0.3s ease;
}
</style>
