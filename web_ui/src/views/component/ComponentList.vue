<template>
  <div class="flex flex-col h-full">
    <!-- 搜索过滤区 -->
    <div
      class="bg-white p-3 rounded-xl border border-slate-100 shadow-sm flex justify-between items-center mb-4"
    >
      <div class="flex gap-3 flex-1">
        <el-input
          v-model="searchQuery"
          placeholder="搜索组件名称或用途"
          class="!w-80"
          :prefix-icon="Search"
          clearable
        />
      </div>
      <div class="flex items-center gap-3">
        <el-button
          type="primary"
          :icon="Plus"
          class="!rounded-lg shadow-sm"
          @click="editorRef!.show = true"
        >
          注册新组件
        </el-button>
        <el-button
          :icon="Refresh"
          circle
          class="!border-none hover:bg-slate-100"
          @click="loadData"
        />
      </div>
    </div>

    <!-- 表格内容区 -->
    <div class="flex-1 h-0">
      <Table :data="componentList" @edit="onEdit" />
    </div>

    <!-- 分页 -->
    <div class="flex justify-between items-center mt-4 text-gray-500">
      <span class="text-sm">共 {{ total }} 条</span>
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="size"
        background
        layout="prev, pager, next"
        :total="total"
      />
    </div>
    <ComponentEditor v-model="form" ref="editorRef" :key="form.id" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { Plus, Refresh, Search } from '@element-plus/icons-vue'
import Table from './Table.vue'
import * as module from '@/api/modules'
import ComponentEditor from '@/views/component/ComponentEditor.vue'

const searchQuery = ref()
const page = ref(1)
const size = ref(10)
const total = ref(0)
const componentList = ref<module.renderComponent.Entity[]>([
  // {
  //   id: '1',
  //   name: '代码解释器',
  //   path: 'code_interpreter',
  //   logo: 'icon-code',
  //   authorType: 'official',
  //   authorName: '系统官方',
  //   purpose: '执行 Python 代码并返回结果，支持数据分析、文件处理和可视化等任务。',
  // },
  // {
  //   id: '2',
  //   name: '网页搜索',
  //   path: 'web_search',
  //   logo: 'icon-search',
  //   authorType: 'official',
  //   authorName: '系统官方',
  //   purpose: '搜索互联网上的最新信息，获取实时数据和网页内容摘要。',
  // },
  // {
  //   id: '3',
  //   name: '数据库查询',
  //   path: 'db_query',
  //   logo: 'icon-database',
  //   authorType: 'agent',
  //   authorName: '数据分析助手',
  //   purpose: '连接并查询数据库，支持 SQL 执行、数据检索和结果分析。',
  // },
  // {
  //   id: '4',
  //   name: '文档摘要',
  //   path: 'doc_summarizer',
  //   logo: 'icon-file-text',
  //   authorType: 'self',
  //   authorName: '张三',
  //   purpose: '对长文档进行摘要提炼，生成简洁的内容概述和关键信息。',
  // },
  // {
  //   id: '5',
  //   name: '图像生成',
  //   path: 'image_generator',
  //   logo: 'icon-image',
  //   authorType: 'official',
  //   authorName: '系统官方',
  //   purpose: '根据文本描述生成高质量图像，支持多种风格和尺寸配置。',
  // },
  // {
  //   id: '6',
  //   name: '情感分析',
  //   path: 'sentiment_analysis',
  //   logo: 'icon-heart',
  //   authorType: 'others',
  //   authorName: '李四',
  //   purpose: '分析文本的情感倾向，识别积极、消极或中性情感。',
  // },
  // {
  //   id: '7',
  //   name: '表格数据提取',
  //   path: 'table_extractor',
  //   logo: 'icon-table',
  //   authorType: 'agent',
  //   authorName: '表格助手',
  //   purpose: '从文本或图片中提取表格数据，支持结构化输出。',
  // },
  // {
  //   id: '8',
  //   name: '多语言翻译',
  //   path: 'translator',
  //   logo: 'icon-translation',
  //   authorType: 'self',
  //   authorName: '王五',
  //   purpose: '支持多种语言之间的互译，保持原文语义和格式。',
  // },
])
onMounted(() => {
  loadData()
})

const form = ref<module.renderComponent.Entity>({} as any)
const editorRef = ref<typeof ComponentEditor>()
const loadData = async () => {
  const { data } = await module.renderComponent.total(searchQuery.value)
  total.value = data
  await loadList()
}

async function loadList() {
  const { data } = await module.renderComponent.page(page.value, size.value, searchQuery.value)
  componentList.value = data
}

function onEdit(row: module.renderComponent.Entity) {
  form.value = row
  editorRef.value!.show = true
}
</script>
