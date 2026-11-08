<template>
  <!-- 最外层容器：靠左对齐 -->
  <div class="flex items-start gap-3 w-full max-w-[90%] mb-6 group animate-fade-in">
    <ChatMessageItem nickname="用户" :time="event.time">
      <template #avatar>
        <div
          :class="[
            'w-10 h-10 rounded-xl flex items-center justify-center shadow-lg border border-white/10 transition-transform group-hover:scale-105',
            'bg-gradient-to-br from-indigo-500 to-purple-600 shadow-purple-500/20',
          ]"
        >
          <SvgIcon name="user" class="text-white text-xl" />
        </div>
      </template>
      <p class="text-[15px] leading-relaxed whitespace-pre-wrap break-words font-medium">
        {{ event.data.content }}
      </p>
    </ChatMessageItem>
  </div>
</template>

<script setup lang="ts">
import { agent } from '@/api/modules/index.ts'
import * as modules from '@/api/modules'

import ChatMessageItem from '@/views/chat/components/ChatMessageItem.vue'
import SvgIcon from '@/components/SvgIcon.vue'
import { EventTypeEnum } from '@/api/modules/enums.ts'

const props = defineProps<{
  agentInfo: agent.Entity
  event: Extract<modules.event.Entity, { event_type: EventTypeEnum.UserRequest }>
}>()

// 校验事件类型
if (props.event.event_type !== EventTypeEnum.UserRequest) {
  // 注意：在组件渲染逻辑中直接 throw 可能会导致整个页面崩溃，建议使用逻辑判断
  console.error('Invalid event type for UserMessage component')
}
</script>

<style scoped>
/* 简单的入场动画 */
.animate-fade-in {
  animation: fadeIn 0.3s ease-out forwards;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 针对长文本的滚动条优化 (如果内容极长) */
.whitespace-pre-wrap {
  word-break: break-word;
  overflow-wrap: break-word;
}
</style>
