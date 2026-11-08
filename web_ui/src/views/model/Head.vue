<template>
  <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
    <div v-for="item in stats" :key="item.label" class="bg-white p-5 rounded-xl border border-gray-100 shadow-sm flex items-center">
      <div :class="[item.bgClass, 'p-3 rounded-lg mr-4']">
        <component :is="item.icon" class="w-6 h-6" :class="item.iconClass" />
      </div>
      <div>
        <p class="text-sm text-gray-500 mb-1">{{ item.label }}</p>
        <p class="text-2xl font-bold text-gray-800">{{ item.value }}</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Box, Coin, Lightning, PieChart } from '@element-plus/icons-vue'
import * as module from '@/api/modules'

const props = defineProps<{ data: module.model.Entity[] }>()

const stats = computed(() => {
  const totalModels = props.data.length
  const totalTokens = props.data.reduce((acc, cur) => acc + cur.total_tokens, 0)
  const consumedTokens = props.data.reduce((acc, cur) => acc + cur.consumed_tokens, 0)
  const ratio = totalTokens > 0 ? ((consumedTokens / totalTokens) * 100).toFixed(2) : '0.00'

  return [
    { label: '模型总数', value: totalModels, icon: Box, bgClass: 'bg-blue-50', iconClass: 'text-blue-500' },
    { label: '总 Token 数', value: totalTokens.toLocaleString(), icon: Coin, bgClass: 'bg-cyan-50', iconClass: 'text-cyan-500' },
    { label: '已消耗 Token 数', value: consumedTokens.toLocaleString(), icon: Lightning, bgClass: 'bg-yellow-50', iconClass: 'text-yellow-500' },
    { label: '消耗占比', value: `${ratio}%`, icon: PieChart, bgClass: 'bg-indigo-50', iconClass: 'text-indigo-500' },
  ]
})
</script>
