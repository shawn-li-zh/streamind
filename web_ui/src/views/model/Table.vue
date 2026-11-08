<template>
  <div class="flex flex-col flex-1">
    <!-- 过滤器操作区 -->
    <div class="flex justify-between items-center mb-4">
      <div class="flex gap-3">
        <el-input
          v-model="searchQuery"
          placeholder="搜索模型名称"
          class="!w-64"
          :prefix-icon="Search"
        />
        <el-select v-model="filterModality" placeholder="所有模态" clearable class="!w-32">
          <el-option label="文本" value="text" />
          <el-option label="图像" value="image" />
        </el-select>
        <el-select v-model="filterStatus" placeholder="全部状态" clearable class="!w-32">
          <el-option label="可用" value="available" />
          <el-option label="禁用" value="unavailable" />
        </el-select>
      </div>
      <el-button :icon="Refresh" circle />
    </div>
    <!-- 表格 -->
    <div class="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden h-full">
      <el-table
        :data="modelList"
        style="width: 100%"
        header-cell-class-name="bg-gray-50 text-gray-600 font-medium"
      >
        <!-- 模型名称 -->
        <el-table-column label="模型名称" min-width="150">
          <template #default="{ row }">
            <div class="flex flex-col">
              <div class="flex items-center gap-2">
                <span class="font-bold text-gray-800">{{ row.name }}</span>
              </div>
              <div class="text-xs text-gray-400 mt-1">{{ row.description }}</div>
            </div>
          </template>
        </el-table-column>

        <!-- 调用地址 -->
        <el-table-column label="调用地址" min-width="150">
          <template #default="{ row }">
            <div class="flex items-center text-blue-500 cursor-pointer hover:underline">
              <el-icon class="mr-1">
                <Link />
              </el-icon>
              <span class="truncate max-w-[150px]">{{ row.api_url }}</span>
              <el-icon class="ml-1 text-gray-400" @click.stop="copy(row.api_url)">
                <CopyDocument />
              </el-icon>
            </div>
          </template>
        </el-table-column>

        <!-- 输入/输出模态 -->
        <el-table-column label="输入模态" width="200">
          <template #default="{ row }">
            <div class="flex gap-1">
              <ModalityIcon v-for="m in row.input_modals" :key="m" :type="m" />
            </div>
          </template>
        </el-table-column>

        <el-table-column label="输出模态" width="150">
          <template #default="{ row }">
            <div class="flex gap-1">
              <ModalityIcon v-for="m in row.output_modals" :key="m" :type="m" />
            </div>
          </template>
        </el-table-column>

        <!-- Token 统计 -->
        <el-table-column label="Token用量" width="180">
          <template #default="{ row }">
            <div class="flex flex-col w-full">
              <div class="flex justify-between text-xs">
                <div class="font-medium text-gray-700">
                  {{ row.consumed_tokens.toLocaleString() }}
                </div>
              </div>
            </div>
          </template>
        </el-table-column>

        <!-- 状态 -->
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <div class="flex items-center">
              <span
                :class="[
                  'w-2 h-2 rounded-full mr-2',
                  row.status === 'active' ? 'bg-green-500' : 'bg-red-500',
                ]"
              ></span>
              <span class="text-sm text-gray-600">{{
                row.status === 'active' ? '可用' : '禁用'
              }}</span>
            </div>
          </template>
        </el-table-column>

        <!-- 操作 -->
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <div class="flex items-center gap-3">
              <el-button link type="primary">查看</el-button>
              <el-dropdown>
                <el-icon class="cursor-pointer text-gray-400 rotate-90">
                  <MoreFilled />
                </el-icon>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item @click="$emit('update', row)">编辑</el-dropdown-item>
                    <el-dropdown-item divided class="text-red-500">删除</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
          </template>
        </el-table-column>
      </el-table>
    </div>
    <!-- 分页 -->
    <div class="flex justify-between items-center mt-6 text-gray-500">
      <span class="text-sm">共 {{ filteredList.length }} 条</span>
      <el-pagination background layout="prev, pager, next" :total="filteredList.length" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { CopyDocument, Link, MoreFilled, Refresh, Search } from '@element-plus/icons-vue'
import * as module from '@/api/modules'
import ModalityIcon from './ModalityIcon.vue'
import { ElMessage } from 'element-plus'
import { computed, onMounted, ref } from 'vue'

defineEmits<{ update: [row: module.model.Entity] }>()

const modelList = ref<module.model.Entity[]>([])
onMounted(() => {
  module.model.list().then(res => {
    modelList.value = res.data
  })
})

const searchQuery = ref('')
const filterModality = ref('')
const filterStatus = ref('')

const filteredList = computed(() => {
  return modelList.value.filter(item => {
    const matchSearch = item.name.toLowerCase().includes(searchQuery.value.toLowerCase())
    const matchStatus = filterStatus.value ? item.status === filterStatus.value : true
    return matchSearch && matchStatus
  })
})
const copy = (text: string) => {
  navigator.clipboard.writeText(text)
  ElMessage.success('已复制到剪贴板')
}
</script>
