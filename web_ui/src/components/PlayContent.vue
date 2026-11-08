<template>
  <div
    ref="containerRef"
    class="relative flex items-center rounded-lg cursor-pointer overflow-hidden transition-all duration-300 select-none"
    :style="{ backgroundColor: bgColor, color: textColor }"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
    @click="$emit('click')"
  >
    <!-- 文本包装层 -->
    <div class="flex-1 overflow-hidden h-full flex items-center px-2 py-1.5">
      <!-- 情况 A：鼠标悬停且内容溢出 -> 开启滚动 -->
      <div v-if="isHovered && isOverflow" class="flex whitespace-nowrap animate-marquee">
        <span ref="textRef" class="pr-8">{{ content }}</span>
        <!-- 镜像文本实现无缝衔接 -->
        <span class="pr-8">{{ content }}</span>
      </div>
      <!-- 情况 B：正常状态或内容未溢出 -> 截断显示 -->
      <div v-else class="w-full truncate" :title="content">
        {{ content }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'

const props = defineProps<{
  content: string
  textColor?: string
  bgColor?: string
  speed?: number
}>()

defineEmits(['click'])

const containerRef = ref(null)
const textRef = ref(null)
const isHovered = ref(false)
const isOverflow = ref(false)

// 检测是否溢出的方法
const checkOverflow = () => {
  if (containerRef.value) {
    // 创建一个临时 span 来测量文字实际宽度，或者在特定条件下比对
    const containerWidth = containerRef.value.offsetWidth

    // 创建临时元素辅助测量（因为 truncate 状态下拿不到完整宽度）
    const tempSpan = document.createElement('span')
    tempSpan.style.visibility = 'hidden'
    tempSpan.style.position = 'absolute'
    tempSpan.style.whiteSpace = 'nowrap'
    tempSpan.style.fontSize = '14px' // 对应 text-sm
    tempSpan.innerText = props.content
    document.body.appendChild(tempSpan)

    isOverflow.value = tempSpan.offsetWidth > containerWidth

    document.body.removeChild(tempSpan)
  }
}

let resizeObserver = null

onMounted(() => {
  checkOverflow()
  // 监听容器大小变化
  resizeObserver = new ResizeObserver(checkOverflow)
  resizeObserver.observe(containerRef.value)
})

onUnmounted(() => {
  if (resizeObserver) resizeObserver.disconnect()
})

// 内容改变时重新检测
watch(
  () => props.content,
  () => {
    nextTick(checkOverflow)
  },
)

// 计算动画时长（可选：根据长度动态计算速度）
const animationDuration = Math.max(props.content.length * 0.2, 10) + 's'
</script>

<style scoped>
/* 滚动动画：从 0 位移到 -50%（刚好是一组文字的距离） */
.animate-marquee {
  display: inline-flex;
  animation: marquee v-bind(animationDuration) linear infinite;
}

@keyframes marquee {
  0% {
    transform: translateX(0);
  }
  100% {
    transform: translateX(-50%);
  }
}

/* 兼容性优化 */
.truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
