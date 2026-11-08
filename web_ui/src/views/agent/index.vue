<template>
  <div class="h-full bg-gray-50 p-6 flex flex-col">
    <!-- 头部 -->
    <div class="flex justify-between items-start mb-6">
      <div>
        <h1 class="flex text-xl font-bold text-gray-800 items-center">
          <SvgIcon name="bot" class="text-blue-400 text-xl" />
          智能体列表
        </h1>
        <p class="text-sm text-gray-500 mt-1">查看和管理可用的模型，获取调用地址和使用统计信息。</p>
      </div>
      <el-button type="primary" :icon="Plus" class="!rounded-lg" @click="edit({})"
        >添加智能体
      </el-button>
    </div>
    <Table @update="raw => edit(raw)" />
    <!-- 添加弹窗 -->
    <Editor v-model="current" ref="editorRef" :key="current.id" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { Plus } from '@element-plus/icons-vue'
import * as module from '@/api/modules'
import Table from './Table.vue'
import Editor from './Editor.vue'
import SvgIcon from '@/components/SvgIcon.vue'

const modelList = ref<module.agent.Entity[]>([])
onMounted(() => {
  module.agent.tree().then(res => {
    modelList.value = res.data
  })
})
const editorRef = ref<any>(null)

const current = ref<module.agent.Entity>({} as any)

function edit(obj: any) {
  current.value = obj
  editorRef.value.show = true
}
</script>

<style>
/* 可以在此处微调 Element Plus 样式 */
.el-table {
  --el-table-header-bg-color: #f9fafb;
}
</style>
