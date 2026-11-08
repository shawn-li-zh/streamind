<template>
  <div :class="payload.class" :style="payload.style">
    <template v-if="!isLoading">
      <!--      组件-->
      <component
        :key="payload.path"
        :is="RemoteComp"
        :attrs="payload.attrs || {}"
        :value="valueRef"
        @updateValue="updateValue"
      />
    </template>
    <template v-else>
      <div>组件加载中...</div>
    </template>
  </div>
</template>
<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, provide, ref } from 'vue'
import * as modules from '@/api/modules/index.ts'
import { agent } from '@/api/modules/index.ts'
import requestHttp from '@/api'
import type { RenderPayload } from '@/api/modules/section.ts'

const props = defineProps<{
  agentInfo: agent.Entity
  section: modules.section.Entity
  preview?: boolean
}>()

interface StreamindRuntime {
  sendEvent: (message: string) => Promise<void>
  getLocalImageUrl: (path: string) => string
}

provide<StreamindRuntime>('runtime', {
  sendEvent: async (message: string) => {
    if (props.preview) return
    const { data } = await modules.agent.ui_interaction(props.agentInfo.id, {
      section_id: props.section.id,
      message,
    })
  },
  getLocalImageUrl: (path: string) => {
    if (path.startsWith('tool/')) {
      path = props.agentInfo.workspace + '/' + path
    }
    return requestHttp.getUri(`/api/local-image`, { path })
  },
})
const isLoading = ref(false)
const payload = computed<RenderPayload>(() => props.section.payload)
const valueRef = ref<any>(props.section.value)
const url = requestHttp.getUri(`/public/components/${payload.value.path}`)
const RemoteComp = defineAsyncComponent(() => import(url))
onMounted(() => {
  isLoading.value = false
})

function updateValue(value: any) {
  if (props.preview) return
  modules.section.update_value(props.section.id, value).then(() => {
    valueRef.value = value
  })
}

// //监听状态
// watch(
//   () => values.value,
//   nv => {
//     renderState.set_state_by_id(props.section.id, nv).then(() => {
//       state.value = nv
//     })
//   },
//   { deep: true },
// )
</script>
