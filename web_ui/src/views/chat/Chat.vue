<template>
  <div class="flex flex-col h-full bg-gray-50 text-gray-800 font-sans">
    <!-- 聊天内容区域 -->
    <!-- 聊天内容区域 -->
    <el-scrollbar height="90vh" ref="scrollbarRef" always @scroll="handleScroll">
      <div class="flex-1 p-4 md:p-6 space-y-3" ref="innerRef">
        <!-- 这里建议放一个加载状态提示 -->
        <div v-if="isLoading" class="text-center py-2 text-gray-400 text-sm">Loading...</div>
        <div v-if="!hasMore" class="text-center py-2 text-gray-300 text-sm">聊天记录已全部加载</div>

        <div
          v-for="(message, index) in messages"
          :key="index"
          class="flex flex-col max-w-4xl mx-auto w-full"
        >
          <AgentResponse
            v-if="
              message.event.event_type == EventTypeEnum.AgentResponse && message.sections.length
            "
            :agent-info="agent.data"
            :event="message.event"
            :sections="message.sections"
          />
          <UserRequest
            v-if="message.event.event_type == EventTypeEnum.UserRequest"
            :agent-info="agent.data"
            :event="message.event"
          />
        </div>
      </div>
      <div style="height: 100px" />
    </el-scrollbar>
    <!-- 底部输入框 -->
    <footer class="bg-white border-t border-gray-200 p-4 shrink-0 sticky bottom-0">
      <div class="w-full mx-auto flex items-end gap-3 relative">
        <textarea
          v-model="inputRaw"
          @keydown.enter.prevent="handleEnter"
          rows="1"
          placeholder="Type your message..."
          class="flex-1 bg-gray-50 border border-gray-300 rounded-2xl px-4 py-3.5 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-none max-h-32 min-h-[52px]"
          :disabled="!agent.connected"
        ></textarea>
        <button
          @click="sendMessage"
          :disabled="!inputRaw.trim() || !agent.connected"
          class="bg-blue-600 hover:bg-blue-700 text-white rounded-xl px-6 py-3.5 font-medium transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center shrink-0"
        >
          Send
        </button>
      </div>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import * as module from '@/api/modules/'
import { EventTypeEnum } from '@/api/modules/enums.ts'

import { type AgentInfo, useAgentStore } from '@/stores/agent.ts'

import UserRequest from '@/views/chat/event/UserRequest.vue'
import AgentResponse from '@/views/chat/event/AgentResponse.vue'
import type { ScrollbarInstance } from 'element-plus'

const props = defineProps<{
  agent: AgentInfo
}>()

// --- 状态定义 ---
const inputRaw = ref('')

const innerRef = ref<HTMLElement | null>(null)
const scrollbarRef = ref<ScrollbarInstance>()
const messages = computed(() => props.agent.messages)
onMounted(() => {
  if (!props.agent.agent_id) return
  scrollToBottom()
})

// 回车发送 (Shift+Enter 换行)
const handleEnter = (e: KeyboardEvent) => {
  if (e.shiftKey) return
  sendMessage()
}

// 滚动到底部
const scrollToBottom = async () => {
  await nextTick()
  setTimeout(() => {
    if (innerRef.value) {
      scrollbarRef.value!.setScrollTop(innerRef.value!.scrollHeight)
    }
  }, 300)
}
const agentStore = useAgentStore()
const agent = agentStore.getAgent(props.agent.agent_id)
// 发送用户消息并触发后端的推流机制
const sendMessage = async () => {
  if (!inputRaw.value.trim()) return

  const userText = inputRaw.value.trim()
  inputRaw.value = ''

  try {
    // 调用该接口后，后端应该开始通过上面已经建立的 SSE 连接发送事件
      await module.agent.chat(props.agent.agent_id, userText)

    // 2. 设置生成状态并调用触发接口
    scrollToBottom()
  } catch (error) {
    console.error('发送消息失败:', error)
  }
}

// --- 状态定义 ---
const isLoading = ref(false)
const hasMore = ref(true)

// 处理滚动事件
const handleScroll = ({ scrollTop }: { scrollTop: number }) => {
  // 当距离顶部小于 50px 时尝试加载更多 (预留一点距离体验更好)
  if (scrollTop < 50 && !isLoading.value && hasMore.value) {
    loadMoreMessages()
  }
}

async function loadMoreMessages() {
  if (isLoading.value || !hasMore.value) return

  isLoading.value = true

  // 1. 记录加载前的滚动高度
  const oldScrollHeight = innerRef.value?.scrollHeight || 0

  try {
    // 2. 调用 Store 的加载方法
    // 假设 agentStore.loadMoreEvents 返回 boolean 表示是否还有更多
    hasMore.value = await agentStore.loadMoreEvents(agent.agent_id)
    // 3. 关键：修正滚动条位置
    // 等待 DOM 更新（新消息渲染完成）
    await nextTick()

    if (innerRef.value && scrollbarRef.value) {
      const newScrollHeight = innerRef.value.scrollHeight
      const scrollOffset = newScrollHeight - oldScrollHeight

      // 将滚动条恢复到之前的位置，避免跳动
      // 保持视觉上还在刚才看的那条消息处
      scrollbarRef.value.setScrollTop(scrollOffset)
    }
  } catch (error) {
    console.error('Failed to load more messages:', error)
  } finally {
    isLoading.value = false
  }
}

defineExpose({
  scrollToBottom,
})
</script>

<style>
::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background-color: rgba(156, 163, 175, 0.5);
  border-radius: 20px;
}
</style>
