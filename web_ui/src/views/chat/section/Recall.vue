<template>
  <ReactionCard
    title="回忆"
    :content="section.content"
    :elapsed="section.elapsed"
    :running="!section.elapsed"
    content-class="bg-slate-900 text-green-400"
  >
    <template #logo>
      <SvgIcon name="tool" class="text-orange-500" />
    </template>
    <template #title="{ title }">
      <div class="flex flex-row space-x-2">
        <!-- 标题 -->
        <span class="text-[15px] font-semibold text-gray-800">{{ title }}</span>
      </div>
    </template>
    <template #content="{ content }">
      <PlayContent :content="content!" class="bg-slate-900 text-green-400 text-[11px] w-auto" />
    </template>
  </ReactionCard>
</template>

<script setup lang="ts">
import * as module from '@/api/modules'
import ReactionCard from '@/views/chat/components/ReactionCard.vue'
import SvgIcon from '@/components/SvgIcon.vue'
import PlayContent from '@/components/PlayContent.vue'

const props = defineProps<{
  section: module.section.Entity
}>()
</script>

<style scoped>
.rotate-180 {
  transform: rotate(180deg);
}

pre {
  font-family: 'Fira Code', 'Cascadia Code', Monaco, Consolas, monospace;
}
</style>
