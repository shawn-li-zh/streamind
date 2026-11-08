<!-- components/ScorePanel.vue -->

<script setup lang="ts">
import type { Entity } from '@/views/emergence/core/domain.ts'
import { onMounted, ref } from 'vue'

defineProps<{
  entities: Entity<any>[]
}>()

const tick = ref(0)

onMounted(() => {
  setInterval(() => {
    tick.value++
  }, 500)
})
</script>

<template>
  <div class="w-80 border-l p-4 overflow-auto bg-slate-50">
    <h2 class="text-lg font-semibold mb-4">Entity Scores</h2>

    <div
      v-for="entity in entities"
      :key="entity.race_name + entity.id + tick"
      class="mb-3 p-3 rounded bg-white shadow-sm"
    >
      <div class="flex justify-between">
        <span>
          {{ entity.race_name + '-' + entity.id }}
        </span>

        <span class="font-mono">
          {{ entity.score.toFixed(2) }}
        </span>
      </div>

      <div class="text-xs text-slate-500 mt-1">
        reward:
        {{ entity.cur_exp.reward.toFixed(2) }}
      </div>
    </div>
  </div>
</template>
