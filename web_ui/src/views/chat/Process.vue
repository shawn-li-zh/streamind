<template>
  <div
    class="flex flex-col h-full bg-slate-50/50 dark:bg-slate-900/50 border border-slate-200 dark:border-slate-800 overflow-hidden"
  >
    <!-- 头部：活动状态 -->
    <div
      class="px-4 py-3 border-b border-slate-200 dark:border-slate-800 bg-white/50 dark:bg-slate-900/50 flex items-center justify-between"
    >
      <div class="flex items-center gap-2">
        <h2 class="text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400">
          思维/动作链
        </h2>
      </div>
    </div>

    <!-- 主体：活动流 -->
    <main ref="scrollContainer" class="flex-1 overflow-y-auto p-4 space-y-4 custom-scrollbar">
      <div v-if="reactions.length > 0" class="space-y-4 relative">
        <template v-for="(reaction, index) in reactions" :key="index">
          <template v-if="reaction.sections.length">
            <div class="text-[11px] text-gray-400 text-center">{{ reaction.event.time }}</div>
            <template v-for="(section, si) in reaction.sections" :key="si">
              <!-- 动态组件渲染 -->
              <component :is="sectionComponentMap[section.name]" :section="section" />
            </template>
          </template>
        </template>
      </div>

      <!-- 空状态 -->
      <div v-else class="h-full flex flex-col items-center justify-center text-slate-400 space-y-3">
        <div class="p-4 rounded-full bg-slate-100 dark:bg-slate-800/50">
          <el-icon size="32" class="opacity-20">
            <Monitor />
          </el-icon>
        </div>
        <p class="text-xs font-mono tracking-widest uppercase opacity-50 text-center">
          Waiting for system pulse...<br />
          <span class="text-[10px]">No active processes detected</span>
        </p>
      </div>
    </main>
  </div>
</template>

<script setup lang="ts">
import { sectionComponentMap } from '@/views/chat/index.ts'
import { computed, nextTick, onMounted, ref } from 'vue'
import { type AgentInfo } from '@/stores/agent.ts'
import { Monitor } from '@element-plus/icons-vue'

const props = defineProps<{
  agent: AgentInfo
}>()
// const agentStore = useAgentStore()
// const agentInfo = agentStore.getAgent(props.agent.id)
const reactions = computed(() => props.agent.reactions)
const scrollContainer = ref<HTMLElement | null>(null)
onMounted(() => {
  scrollToBottom()
})

async function scrollToBottom() {
  await nextTick()

  setTimeout(() => {
    if (scrollContainer.value) {
      scrollContainer.value.scrollTo({
        top: scrollContainer.value.scrollHeight,
        behavior: 'smooth',
      })
    }
  }, 300)
}

defineExpose({
  scrollToBottom,
})
</script>

<style scoped>
/* 自定义滚动条，增强科技感 */
.custom-scrollbar::-webkit-scrollbar {
  width: 4px;
}

.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}

.custom-scrollbar::-webkit-scrollbar-thumb {
  @apply bg-slate-300 dark:bg-slate-700 rounded-full;
}

.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  @apply bg-white;
}

/* 进场动画 */
@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateX(-10px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}

.group {
  animation: slideIn 0.3s ease-out forwards;
}
</style>
