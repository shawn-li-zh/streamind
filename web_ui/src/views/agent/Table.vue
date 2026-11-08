<template>
  <div class="flex flex-col flex-1">
    <!-- 过滤器操作区 -->
    <div class="flex justify-between items-center mb-4">
      <div class="flex gap-3">
        <el-input
          v-model="searchQuery"
          placeholder="搜索智能体名称"
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
        :data="list"
        style="width: 100%"
        header-cell-class-name="bg-gray-50 text-gray-600 font-medium"
      >
        <!-- 模型名称 -->
        <el-table-column label="模型名称" min-width="150">
          <template #default="{ row }">
            <div class="flex flex-col">
              <div class="flex items-center gap-2">
                <el-avatar :size="32" shape="square" :src="row.avatar" />
                <span class="font-bold text-gray-800">{{ row.name }}</span>
              </div>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="运行时间" width="80">
          <template #default="{ row }">
            <div class="text-gray-500 rounded bg-gray-300 text-center">
              {{ formattedTime(row.startup_time) }}
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
import { MoreFilled, Refresh, Search } from '@element-plus/icons-vue'
import * as module from '@/api/modules'
import { computed, onMounted, ref } from 'vue'
import { dayjs } from 'element-plus'

defineEmits<{ update: [row: module.agent.Entity] }>()

const list = ref<module.agent.Entity[]>([])
onMounted(() => {
  module.agent.tree().then(res => {
    list.value = res.data
  })
})

const searchQuery = ref('')
const filterModality = ref('')
const filterStatus = ref('')

const filteredList = computed(() => {
  return list.value.filter(item => {
    const matchSearch = item.name.toLowerCase().includes(searchQuery.value.toLowerCase())
    const matchStatus = filterStatus.value ? item.status === filterStatus.value : true
    return matchSearch && matchStatus
  })
})

const currentTime = dayjs()
const formattedTime = (time: string) => {
  if (!time) return '-:-:-'
  const startTime = dayjs(time)
  const sec = currentTime.diff(startTime, 'second')
  const h = Math.floor(sec / 3600)
    .toString()
    .padStart(2, '0')
  const m = Math.floor((sec % 3600) / 60)
    .toString()
    .padStart(2, '0')
  const s = (sec % 60).toString().padStart(2, '0')
  return `${h}:${m}:${s}`
}
</script>
