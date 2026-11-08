<template>
  <div class="h-full bg-gray-50 p-6 flex flex-col">
    <!-- 头部 -->
    <div class="flex justify-between items-start mb-6">
      <div>
        <h1 class="text-xl font-bold text-gray-800">模型列表</h1>
        <p class="text-sm text-gray-500 mt-1">查看和管理可用的模型，获取调用地址和使用统计信息。</p>
      </div>
      <el-button type="primary" :icon="Plus" class="!rounded-lg" @click="edit({ id: 0 })"
        >添加模型
      </el-button>
    </div>

    <!-- 统计卡片 -->
    <ModelStats :data="[]" />
    <ModelTable @update="raw => edit(raw)" />
    <!-- 添加模型弹窗 -->
    <Editor v-model="currentModel" ref="editorRef" :key="currentModel.id" />
  </div>
</template>

<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { Plus } from '@element-plus/icons-vue'
import * as module from '@/api/modules'
import ModelStats from './Head.vue'
import ModelTable from './Table.vue'
import Editor from '@/views/model/Editor.vue'

const editorRef = ref<any>(null)
const currentModel = ref<module.model.Entity>({} as any)

async function edit(obj: any) {
  await nextTick()
  currentModel.value = obj
  editorRef.value.show = true
}
</script>

<style>
/* 可以在此处微调 Element Plus 样式 */
.el-table {
  --el-table-header-bg-color: #f9fafb;
}
</style>
