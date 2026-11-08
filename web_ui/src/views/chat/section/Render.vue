<template>
  <!--
    1. 外层容器：
    - w-fit max-w-full: 宽度自适应内部组件，但不超过父容器
    - ring-1: 替代 border，提供更细腻的 1px 物理边框感
    - group: 用于子元素的 hover 联动交互
    - shadow-sm hover:shadow-md: 悬浮时的呼吸感交互
  -->
  <div
    class="relative flex flex-col min-w-[280px] max-w-full bg-white rounded-xl ring-1 ring-slate-200/60 shadow-sm hover:shadow-md hover:ring-slate-300/80 transition-all duration-300 overflow-hidden"
  >
    <!-- 2. 标题栏：内嵌感、小灰字、窗口指示点 -->
<!--    <div-->
<!--      v-if="payload.title"-->
<!--      class="flex items-center gap-2 px-3 py-2 bg-slate-50/50 border-b border-slate-100/80"-->
<!--    >-->
<!--      &lt;!&ndash; 装饰性状态点（增强窗口感，hover时微亮） &ndash;&gt;-->
<!--      <div-->
<!--        class="w-1.5 h-1.5 rounded-full bg-slate-300 group-hover:bg-indigo-400 transition-colors duration-300"-->
<!--      ></div>-->

<!--      &lt;!&ndash; 小灰字标题 &ndash;&gt;-->
<!--      <h3 class="text-xs font-medium text-slate-400 tracking-wide truncate select-none">-->
<!--        {{ payload.title }}-->
<!--      </h3>-->
<!--    </div>-->

    <!-- 3. 内容区域 -->
    <div>
      <Transition name="fade" mode="out-in">
        <!-- 加载中状态：骨架屏 -->
        <div v-if="isLoading" class="flex flex-col space-y-3 w-full min-w-[240px]">
          <div class="h-3 bg-slate-100 rounded-full animate-pulse w-2/3"></div>
          <div
            class="h-16 bg-slate-50 rounded-lg animate-pulse w-full border border-slate-100"
          ></div>
        </div>
        <!-- 渲染组件 -->
        <div v-else class="relative">
          <RemoteComponent
            :agent-info="props.agent"
            :section="props.section"
            :component-path="payload.path"
          />
        </div>
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import RemoteComponent from '../components/RemoteComponent.vue'
import * as module from '@/api/modules'
import type { RenderPayload } from '@/api/modules/section.ts'

const props = defineProps<{
  agent: module.agent.Entity
  section: Extract<module.section.Entity, { payload: RenderPayload }>
}>()
const payload = computed<RenderPayload>(() => {
  return props.section.payload || {}
})
// const payload: Payload = props.section.payload
const isLoading = computed(() => {
  return !payload.value.path
})
</script>

<style scoped>
/* 柔和的淡入淡出过渡，避免突兀的闪烁 */
.fade-enter-active,
.fade-leave-active {
  transition:
    opacity 0.25s ease,
    transform 0.25s ease;
}

.fade-enter-from {
  opacity: 0;
  transform: translateY(4px); /* 增加一点微小的下沉动画，强化物理空间感 */
}

.fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

/*
  注意：去掉了原先的 div { box-shadow: ... }
  因为在 Vue <style scoped> 中如果直接写 html 标签，
  会污染内部 RemoteComponent 里面所有的 div，导致排版灾难。
  现在的阴影通过 Tailwind 的 shadow-sm / hover:shadow-md 实现。
*/
</style>
