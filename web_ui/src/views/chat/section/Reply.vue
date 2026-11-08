<template>
  <div class="bg-white px-5 py-4">
    <t-chat-markdown :content="section.content" />
    <span
      v-if="!isClosed"
      class="inline-block w-1.5 h-4 ml-1 align-baseline bg-black animate-[pulse_1s_ease-in-out_infinite]"
    ></span>
  </div>
</template>
<script setup lang="ts">
import { computed } from 'vue'
import * as module from '@/api/modules'
import { ChatMarkdown as TChatMarkdown } from '@tdesign-vue-next/chat'

const props = defineProps<{
  section: module.section.Entity
}>()
const isClosed = computed(() => props.section.id)
</script>

<style scoped></style>
