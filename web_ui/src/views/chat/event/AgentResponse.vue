<template>
  <!-- 最外层容器：靠左对齐 -->
  <div class="flex items-start gap-3 w-full max-w-[90%] mb-6 group animate-fade-in">
    <ChatMessageItem :nickname="agentInfo.name" :time="event.time">
      <template #avatar>
        <div
          :class="[
            'w-10 h-10 rounded-xl flex items-center justify-center shadow-lg border border-white/10 transition-transform group-hover:scale-105',
            'shadow-blue-500/20',
          ]"
        >
          <img :src="agentInfo.avatar" style="object-fit: cover" alt="" />
        </div>
      </template>
      <div class="flex flex-col gap-2">
        <!-- 遍历消息中的所有 Section (标签) -->
        <component
          v-for="(section, sIndex) in sections"
          :key="sIndex"
          :is="sectionComponentMap[section.name]"
          :section="section"
          :agent="agentInfo"
        />
      </div>
    </ChatMessageItem>
  </div>
</template>

<script setup lang="ts">
import * as modules from '@/api/modules'
import { sectionComponentMap } from '@/views/chat'
import ChatMessageItem from '@/views/chat/components/ChatMessageItem.vue'
import type { EventTypeEnum } from '@/api/modules/enums.ts'

const props = defineProps<{
  agentInfo: modules.agent.Entity
  event: Extract<modules.event.Entity, { event_type: EventTypeEnum.AgentResponse }>
  sections: modules.section.Entity[]
}>()
</script>

<style scoped></style>
