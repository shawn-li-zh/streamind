<template>
  <div
    class="w-full max-w-md bg-white border border-gray-100 rounded-xl px-2 py-2 shadow-sm hover:shadow-md transition-shadow duration-300 select-none"
  >
    <!-- 顶部：标题与状态区 -->
    <div class="flex items-center justify-between">
      <div class="flex items-center space-x-2">
        <!-- Logo 插槽与环绕动画 -->
        <slot name="logo">
          <!-- 外层容器：预留光圈空间 -->
          <div class="relative w-9 h-9 flex items-center justify-center">
            <!-- 计时器运行时的环绕光圈动画 -->
            <div
              v-if="isRunning"
              class="absolute inset-0 rounded-xl bg-gradient-to-tr from-[#3B66F5] via-transparent to-transparent animate-spin shadow-sm"
              style="padding: 2px"
            >
              <!-- 内部遮罩，形成边框发光效果 -->
              <div class="w-full h-full bg-white rounded-[10px]"></div>
            </div>

            <!-- Logo 本体 -->
            <div
              class="absolute w-8 h-8 flex items-center justify-center rounded-lg overflow-hidden z-10"
              :class="!logo ? 'bg-[#3B66F5] text-white' : 'bg-transparent border border-gray-100'"
            >
              <img v-if="logo" :src="logo" alt="logo" class="w-full h-full object-cover" />
              <svg
                v-else
                class="w-5 h-5"
                viewBox="0 0 24 24"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
              >
                <path d="M12 2L4 20H8.5L12 11L15.5 20H20L12 2Z" fill="currentColor" />
              </svg>
            </div>
          </div>
        </slot>
        <slot name="title" :title="title">
          <!-- 标题 -->
          <span class="text-[15px] font-semibold text-gray-800">{{ title }}</span>
        </slot>
      </div>

      <!-- 右侧耗时与箭头 -->
      <div class="flex items-center space-x-2 text-gray-400 cursor-pointer" @click="openDialog">
        <span
          class="font-mono text-xs font-medium px-2 py-0.5 rounded transition-colors duration-300"
          :class="
            isRunning
              ? 'text-[#3B66F5] bg-blue-50'
              : 'text-slate-500 bg-slate-100 dark:bg-slate-800'
          "
        >
          <!-- 动态显示时间或传入的值 -->
          {{ duration }}
        </span>
        <el-icon :size="16">
          <View />
        </el-icon>
      </div>
    </div>

    <!-- 下方：文本展示区 (运行追踪 / 结束轮播) -->
    <slot name="content" :content="content">
      <PlayContent
        v-if="content"
        :content="content"
        :class="contentClass || 'bg-[#F4F7FF] text-[#3B66F5]'"
        class="text-[11px]"
      />
    </slot>

    <!-- 弹窗：详情展示区 -->
    <el-dialog v-model="dialogVisible" :title="title + ' - 详情'" width="40%" destroy-on-close body-class="dialog-body">
      <slot name="details">
        <div
          class="p-4 bg-gray-50 rounded-lg text-gray-600 text-sm whitespace-pre-wrap max-h-96 overflow-y-auto font-mono"
        >
          {{ content }}
        </div>
      </slot>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="dialogVisible = false">关闭</el-button>
          <el-button type="primary" @click="dialogVisible = false">确认</el-button>
        </span>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { View } from '@element-plus/icons-vue'
import PlayContent from '@/components/PlayContent.vue'
import { useTimer } from '@/hooks/timer.ts'

interface Props {
  logo?: string
  title?: string
  elapsed?: string
  content?: string
  contentClass?: string
  running: boolean
}

const props = defineProps<Props>()

// --- 弹窗逻辑 ---
const dialogVisible = ref(false)
const openDialog = () => {
  dialogVisible.value = true
}

// --- 计时器与动画逻辑 ---
const isRunning = computed(() => props.running) // 记录是否正在计时/加载
// 格式化时间为 mm:ss
const formatTime = (ms: number) => {
  if (props.elapsed) {
    ms = countMs(props.elapsed)
  }
  const totalSeconds = Math.floor(ms / 1000)
  const m = Math.floor(totalSeconds / 60)
    .toString()
    .padStart(2, '0')
  const s = (totalSeconds % 60).toString().padStart(2, '0')
  // console.log('elapsed', ms, props.elapsed, props.title, isRunning.value, `${m}:${s}`)
  return `${m}:${s}`
}
const { duration } = useTimer(() => {
  return !isRunning.value
}, formatTime)

function countMs(elapsed: string) {
  const items = elapsed.split(' ')
  let ms = 0
  for (const item of items) {
    //分离时间和单位
    const [time, unit] = item.split(/([a-z]+)/i)
    switch (unit) {
      case 'ms':
        ms += parseInt(time || '0')
        break
      case 's':
        ms += parseInt(time || '0') * 1000
        break
      case 'm':
        ms += parseInt(time || '0') * 60 * 1000
        break
    }
  }
  return ms
}
</script>

<style scoped>
/* 左侧渐隐遮罩效果 (追踪最新内容时使用) */
.text-fade-left {
  -webkit-mask-image: linear-gradient(to right, transparent 0%, black 15%, black 100%);
  mask-image: linear-gradient(to right, transparent 0%, black 15%, black 100%);
}

/* 无缝轮播动画 */
.animate-marquee {
  /* 调整 15s 可以改变轮播速度 */
  animation: marquee 10s linear infinite;
}

/* 由于复制了一份文本，平移 -50% 刚好能实现无缝衔接 */
@keyframes marquee {
  0% {
    transform: translateX(0);
  }
  100% {
    transform: translateX(-50%);
  }
}
.dialog-body{
  height: 70vh;
}
</style>
