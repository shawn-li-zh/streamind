<template>
  <div
    :class="['w-6 h-6 rounded flex items-center justify-center text-xs', config.bg, config.color]"
  >
    {{ config.label }}
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ type: string }>()

const modalityMap: Record<string, any> = {
  text: { short: '文', label: '文本', bg: 'bg-blue-50', color: 'text-blue-600' },
  image: { short: '图', label: '图像', bg: 'bg-green-50', color: 'text-green-600' },
  audio: { short: '音', label: '音频', bg: 'bg-purple-50', color: 'text-purple-600' },
  video: { short: '视', label: '视频', bg: 'bg-orange-50', color: 'text-orange-600' },
  vector: { short: '向', label: '向量', bg: 'bg-cyan-50', color: 'text-cyan-600' },
}

const config = computed(() => modalityMap[props.type] || modalityMap['text'])
</script>
