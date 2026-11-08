<template>
  <!-- 最外层布局：可通过 class 透传进一步自定义外边距 -->
  <div class="flex items-start gap-3 w-full group animate-fade-in mb-6">

    <!-- 左侧：头像区 -->
    <div class="flex-shrink-0 mt-1">
      <slot name="avatar">
        <!-- 默认头像展示 -->
        <div
          :class="[
            'w-10 h-10 rounded-xl flex items-center justify-center shadow-lg border border-white/10 transition-transform group-hover:scale-105',
            avatarBgClass || 'bg-gradient-to-br from-indigo-500 to-purple-600 shadow-purple-500/20'
          ]"
        >
          <img v-if="avatar" :src="avatar" class="w-full h-full rounded-xl object-cover"  alt=""/>
          <el-icon v-else :size="20" color="#fff"><UserFilled /></el-icon>
        </div>
      </slot>
    </div>

    <!-- 右侧：内容区 -->
    <div class="flex flex-col gap-1.5 flex-1 min-w-0">

      <!-- 头部信息：名称 + 时间 -->
      <div class="flex items-center justify-between pr-1">
        <div class="flex items-center gap-2">
          <span class="text-sm font-bold text-slate-700 dark:text-slate-200 tracking-wide">
            {{ nickname }}
          </span>
          <!-- 预留：可以在名称后通过 slot 增加标签（如：Bot、Admin） -->
          <slot name="header-extra"></slot>
        </div>

        <span class="text-[11px] font-mono text-slate-400 dark:text-slate-500 bg-slate-100/50 dark:bg-slate-800/50 px-2 py-0.5 rounded border border-slate-200/50 dark:border-slate-700/50">
          {{ time }}
        </span>
      </div>

      <!-- 气泡主体 -->
      <div
        :class="[
          'relative px-4 py-3 rounded-2xl shadow-md border transition-all duration-300',
          'bg-white dark:bg-[#1e1e2d]', // 背景色
          'border-slate-200 dark:border-slate-700/50', // 边框
          'text-slate-800 dark:text-slate-100', // 文字颜色
          'hover:shadow-lg hover:border-indigo-400/30 dark:hover:border-purple-500/30'
        ]"
      >
        <!-- 默认插槽：消息内容 -->
        <div class="text-[15px] leading-relaxed whitespace-pre-wrap break-words font-medium">
          <slot></slot>
        </div>
        <!-- 底部扩展槽：用于放置点赞、复制按钮等 -->
        <div class="mt-2 flex justify-end gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
          <slot name="footer"></slot>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { UserFilled } from '@element-plus/icons-vue'

/**
 * Props 定义
 */
interface Props {
  avatar?: string        // 头像图片地址
  nickname: string       // 昵称
  time: string           // 时间显示文本
  avatarBgClass?: string // 自定义头像背景颜色 (Tailwind Class)
}

defineProps<Props>()

</script>

<style scoped>
.animate-fade-in {
  animation: fadeIn 0.3s ease-out forwards;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>
