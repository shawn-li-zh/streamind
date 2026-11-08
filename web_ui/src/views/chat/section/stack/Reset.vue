<template>
  <ReactionCard
    title="栈顶重置"
    :content="section.content"
    :elapsed="section.elapsed"
    :running="!section.elapsed"
    content-class="bg-red-50 text-red-400"
  >
    <template #logo>
      <SvgIcon name="stack" class="text-red-400" />
    </template>
  </ReactionCard>
</template>

<script setup lang="ts">
import * as module from '@/api/modules'
import ReactionCard from '@/views/chat/components/ReactionCard.vue'
import SvgIcon from '@/components/SvgIcon.vue'

const props = defineProps<{
  section: module.section.Entity
}>()
</script>

<style scoped>
@keyframes blink {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0;
  }
}

/* 自定义滚动条风格 */
::-webkit-scrollbar {
  width: 4px;
}

::-webkit-scrollbar-thumb {
  background: #e2e8f0;
  border-radius: 10px;
}

.rotate-180 {
  transform: rotate(180deg);
}

/* 更好的字体渲染 */
.font-serif {
  font-family: ui-serif, Georgia, Cambria, 'Times New Roman', Times, serif;
}
</style>
