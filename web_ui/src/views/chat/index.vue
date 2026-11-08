<template>
  <div class="flex flex-col size-full">
    <Head :agent="agentInfo" class="w-full" />
    <div class="flex flex-row w-full flex-1" style="height: 90vh" v-loading="!agentInfo.connected">
      <!--      思考过程 函数调用-->
      <Process :agent="agentInfo" class="w-3/12" ref="processRef" />
      <!--      对话-->
      <Chat :agent="agentInfo" class="w-5/12 h-full" ref="chatRef" />
      <div class="flex flex-col w-4/12">
        <Stack
          :agent="agentInfo.data"
          :state_stack="agentInfo.state_stack"
          class="h-[40%] flex-none"
        />
        <EventList :agent="agentInfo" class="h-[60%]" />
      </div>
    </div>
    <Bottom :agent="agentInfo.data" class="h-5" />
  </div>
</template>
<script setup lang="ts">
import { useAgentStore } from '@/stores/agent.ts'
import { nextTick, onMounted, onUnmounted, provide, ref } from 'vue'
import Head from '@/views/chat/layout/Head.vue'
import Bottom from '@/views/chat/layout/Bottom.vue'
import Process from '@/views/chat/Process.vue'
import Chat from '@/views/chat/Chat.vue'
import requestHttp from '@/api'

import type {
  EnvEventPayload,
  SectionEventPayload,
  SsePayload,
  ToolCallEventPayload,
} from '@/views/chat/type.ts'
import { AgentStatusEnum, EventTypeEnum, ToolCallStatus } from '@/api/modules/enums.ts'
import { dayjs } from 'element-plus'
import * as module from '@/api/modules'
import type { ToolCallDetail } from '@/api/modules/event.ts'
import Stack from '@/views/chat/Stack.vue'
import EventList from '@/views/chat/EventList.vue'
import type { SectionName } from '@/api/modules/section.ts'

const processRef = ref()
const chatRef = ref()
const isLoading = ref(true)
const agentStore = useAgentStore()
const agentInfo = agentStore.getAgent(agentStore.last_agent_id || '1')

let abortController: AbortController = new AbortController()
let isDestroyed = false // 标记组件是否已销毁
let retryCount = 0 // 记录重连次数（可选：用于实现指数退避）

onMounted(async () => {
  provide('agentInfo', agentInfo)
  isLoading.value = false
  // document.addEventListener('visibilitychange', handleVisibilityChange);
  connectSse() // 开始连接
  setInterval(custom, 300)
})

// 组件卸载时，停止重连并断开连接
onUnmounted(() => {
  isDestroyed = true
  abortController.abort()
})

function sleep(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms))
}

const messageQueue: SsePayload[] = []
const connectSse = async () => {
  // 1. 如果组件已经销毁，直接退出
  if (isDestroyed) return

  try {
    console.log(`正在尝试连接 SSE... (重连次数: ${retryCount})`)

    // 每次连接前创建新的 AbortController
    abortController = new AbortController()

    await requestHttp.sseConnect(
      `/api/agent/connect/${agentInfo.agent_id}`,
      async data => {
        try {
          const payload: SsePayload = JSON.parse(data)
          messageQueue.push(payload)
        } catch (e) {
          console.error('解析 SSE JSON 失败:', data, e)
        }
      },
      {
        abortController: abortController,
        onConnect: () => {
          console.log('SSE 连接成功')
          agentInfo.connected = true
          retryCount = 0 // 连接成功，重置重连次数
        },
      },
    )

    // 如果 sseConnect 正常结束（比如服务器关闭了连接），也会走到这里
    console.warn('SSE 连接已断开 (正常结束)')
  } catch (e: any) {
    // 只有在非主动取消的情况下才打印错误
    if (e.name !== 'AbortError') {
      console.error('SSE 发生错误:', e)
    }
  } finally {
    agentInfo.connected = false
    // 2. 自动重连逻辑
    if (!isDestroyed) {
      retryCount++
      // 策略：基础等待 1.5s，随次数增加，最长等待 10s
      const delay = Math.min(500 + retryCount * 500, 10000)
      console.log(`${delay / 1000}s 后尝试重连...`)

      await sleep(delay)
      connectSse() // 递归调用
    }
  }
}
let isProcessing = false

async function custom() {
  if (isProcessing || messageQueue.length === 0) return
  isProcessing = true
  while (messageQueue.length > 0) {
    // 如果队列堆积太长（比如后台回来后有几百条），可以考虑批量处理或只取最新状态
    const currentTask = messageQueue.shift()
    if (currentTask) {
      if (currentTask.event == 'section' && currentTask.type == 'chunk') {
        let first = messageQueue[0]
        while (first?.event == 'section' && first?.type == 'chunk') {
          const chunk = messageQueue.shift()!
          currentTask.data.chunk += (chunk.data as any).chunk
          first = messageQueue[0]
        }
      }
      console.log(currentTask)
      await handleSseEvent(currentTask)
    }

    // 给主线程喘息的机会，防止卡死 UI
    // 如果队列依然很长，每处理一个任务释放一次执行权
    if (messageQueue.length > 5) {
      await new Promise(resolve => requestAnimationFrame(resolve))
    }
  }
  isProcessing = false
}

// ================= 核心：状态机处理 =================
// const sse: SsePayload[] = []
const handleSseEvent = async (payload: SsePayload) => {
  // sse.push(payload)
  // if (payload.type !== 'chunk') {
  //   console.log(sse)
  // }

  switch (payload.event) {
    case 'section': {
      await handleSectionEvent(payload)
      break
    }
    case 'env': {
      await handleEnvEvent(payload)
      break
    }
    case 'tool_call': {
      await handleToolCallEvent(payload)
      break
    }
  }
}

async function handleSectionEvent(payload: SectionEventPayload) {
  const now = dayjs().format('YYYY-MM-DD HH:mm:ss')
  const messages = agentInfo.messages
  const reactions = agentInfo.reactions
  const { reaction, message } = agentInfo.last_response_event
  let event_sections = undefined
  switch (payload.type) {
    case 'start':
      agentInfo.data.status = AgentStatusEnum.Processing
      const resp_event: module.event.Entity = {
        id: payload.bind_event_id,
        event_type: EventTypeEnum.AgentResponse,
        agent_id: agentInfo.agent_id,
        state_id: agentInfo.data.state_id,
        force: false,
        time: now,
        data: [],
      }
      agentInfo.last_response_event = {
        reaction: { event: { ...resp_event }, sections: [] },
        message: { event: resp_event, sections: [] },
      }
      agentInfo.last_response_event.reaction!.event.data =
        agentInfo.last_response_event.reaction?.sections!
      agentInfo.last_response_event.message!.event.data =
        agentInfo.last_response_event.message?.sections!
      //
      reactions.push(agentInfo.last_response_event.reaction!)
      messages.push(agentInfo.last_response_event.message!)
      break
    case 'open':
      const isReaction = isReactionSection(payload.data)
      event_sections = isReaction ? reaction : message
      if (event_sections) {
        const sec: module.section.Entity = {
          id: '',
          agent_id: agentInfo.agent_id,
          event_id: payload.bind_event_id,
          name: payload.data,
          content: '',
          elapsed: '',
          payload: 0,
        }
        event_sections.sections.push(sec)
        if (isReaction) {
          processRef.value?.scrollToBottom()
        }
      }
      break
    case 'chunk':
      event_sections = isReactionSection(payload.data.tag) ? reaction : message
      if (event_sections) {
        const last_section = event_sections.sections.at(-1)
        if (last_section) {
          last_section.content += payload.data.chunk
        }
        if (payload.data.tag == 'reply') {
          chatRef.value?.scrollToBottom()
        }
      }
      break
    case 'close':
      event_sections = isReactionSection(payload.data.name) ? reaction : message
      if (event_sections) {
        const last_section: module.section.Entity | undefined = event_sections.sections.at(-1)
        if (last_section) {
          last_section.id = payload.data.id
          last_section.content = payload.data.content
          last_section.elapsed = payload.data.elapsed
          if (payload.data.name == 'render') {
            const { data } = await module.section.get_by_id(last_section.id)
            last_section.payload = data.payload
            last_section.value = data.value
          }
          if (payload.data.name == 'push') {
            const { data } = await module.state.list(agentInfo.agent_id)
            agentInfo.state_stack = data
          }
        }
        chatRef.value?.scrollToBottom()
      }
      break
    case 'end':
      agentInfo.data.status = AgentStatusEnum.Idle
      agentInfo.step += 1
      break
  }
}

function isReactionSection(tag: SectionName) {
  // return tag === 'think' || tag === 'tool_call' || tag === 'push'
  return !(tag === 'reply' || tag === 'render')
}

function handleEnvEvent(payload: EnvEventPayload) {
  const eve: module.event.Entity = {
    id: payload.bind_event_id,
    agent_id: agentInfo.agent_id,
    event_type: EventTypeEnum.Error,
    force: false,
    time: dayjs().format('YYYY-MM-DD HH:mm:ss'),
    data: payload.data,
  } as any

  switch (payload.type) {
    case 'userRequest':
      eve.event_type = EventTypeEnum.UserRequest
      agentInfo.messages.push({ event: eve, sections: [] })
      chatRef.value?.scrollToBottom()
      break
    case 'timer':
      eve.event_type = EventTypeEnum.Timer
      break
    case 'error':
      eve.event_type = EventTypeEnum.Error
      break
  }
}

async function handleToolCallEvent(payload: ToolCallEventPayload) {
  const reaction = agentInfo.reactions.at(-1)
  if (!reaction) {
    console.warn('找不到对应的反应事件')
    return
  }
  if (payload.type == 'start') {
    for (const item of payload.data) {
      const section = reaction.sections.find(s => s.id == item.id)!
      const detail: ToolCallDetail = {
        id: item.id,
        call_id: '',
        package: item.package,
        tool: item.tool,
        values: [],
        args: item.args,
        elapsed: '',
        status: ToolCallStatus.Undo,
      }
      section.payload = detail
    }
    await nextTick()
    return
  }
  if (payload.type == 'active') {
    const section = reaction.sections.find(item => item.id == payload.data)
    if (section) {
      section.payload.status = ToolCallStatus.Active
    }
    return
  }
  if (payload.type == 'success') {
    const section = reaction.sections.find(item => item.id == payload.data.id)
    if (section) {
      section.payload.status = ToolCallStatus.Success
      section.payload.values = payload.data.values
      section.payload.elapsed = payload.data.elapsed
      await nextTick()
    }
  }
  if (payload.type == 'failure') {
    const section = reaction.sections.find(item => item.id == payload.data.id)
    if (section) {
      section.payload.status = ToolCallStatus.Failure
      section.payload.values = payload.data.values
      section.payload.elapsed = payload.data.elapsed
      await nextTick()
    }
  }
}

onUnmounted(() => {
  abortController.abort()
})
</script>

<style scoped></style>
