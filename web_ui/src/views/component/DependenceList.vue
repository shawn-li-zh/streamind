<template>
  <div class="flex flex-col h-full">
    <!-- 搜索过滤区 -->
    <div
      class="bg-white p-3 rounded-xl border border-slate-100 shadow-sm flex justify-between items-center mb-4"
    >
      <div class="flex gap-3 flex-1">
        <el-input
          v-model="searchQuery"
          placeholder="搜索依赖名称或版本号"
          class="!w-80"
          :prefix-icon="Search"
          clearable
        />
      </div>
      <div class="flex items-center gap-3">
        <el-button type="primary" :icon="Plus" class="!rounded-lg shadow-sm"> 添加依赖</el-button>
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
      <el-table :data="filteredList" v-loading="loading" border>
        <el-table-column prop="name" label="依赖名称" min-width="150">
          <template #default="{ row }">
            <div class="flex items-center gap-2">
              <span class="font-bold text-slate-800">{{ row.name }}</span>
              <el-tag size="small">{{ row.version }}</el-tag>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="path" label="路径" min-width="150">
          <template #default="{ row }">
            <a
              v-if="row.path.startsWith('http')"
              :href="row.path"
              target="_blank"
              class="text-blue-500 hover:underline truncate"
            >
              {{ row.path }}
            </a>
            <div v-else class="flex items-center gap-2">
              <el-tag type="info" size="small">本地: {{ row.path }}</el-tag>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <div class="flex gap-2">
              <el-button link type="primary" size="small" @click="handleEdit(row)">
                修改
              </el-button>
              <el-button link type="danger" size="small" @click="handleDelete(row)">
                删除
              </el-button>
              <el-button
                link
                type="success"
                size="small"
                @click="handleDownload(row)"
                :disabled="!row.path.startsWith('http')"
              >
                下载到本地
              </el-button>
            </div>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <!-- 分页 -->
    <div class="flex justify-between items-center mt-4 text-gray-500">
      <span class="text-sm">共 {{ filteredList.length }} 条</span>
      <el-pagination background layout="prev, pager, next" :total="filteredList.length" />
    </div>

    <!-- 修改弹窗 -->
    <el-dialog v-model="editDialogVisible" title="修改依赖" width="400px">
      <el-form :model="editForm" label-width="80px">
        <el-form-item label="名称">
          <el-input v-model="editForm.name" />
        </el-form-item>
        <el-form-item label="路径">
          <el-input v-model="editForm.path" />
        </el-form-item>
        <el-form-item label="版本">
          <el-input v-model="editForm.version" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="handleSave">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { Plus, Refresh, Search } from '@element-plus/icons-vue'
import * as renderDependence from '@/api/modules/renderDependence'
import { ElMessage } from 'element-plus'

const searchQuery = ref('')
const loading = ref(false)
const list = ref<renderDependence.Entity[]>([])

onMounted(() => {
  loadData()
})
const loadData = async () => {
  loading.value = true
  try {
    const { data } = await renderDependence.list()
    list.value = data
  } finally {
    loading.value = false
  }
}

const filteredList = computed(() => {
  if (!searchQuery.value) return list.value
  const q = searchQuery.value.toLowerCase()
  return list.value.filter(
    item => item.name.toLowerCase().includes(q) || item.version.toLowerCase().includes(q),
  )
})

const editDialogVisible = ref(false)
const editForm = reactive({
  id: '',
  name: '',
  path: '',
  version: '',
})
const saving = ref(false)

const handleEdit = (row: renderDependence.Entity) => {
  Object.assign(editForm, row)
  editDialogVisible.value = true
}

const handleSave = async () => {
  saving.value = true
  try {
    await renderDependence.save(editForm)
    const index = list.value.findIndex(item => item.id === editForm.id)
    if (index > -1) list.value[index] = { ...editForm }
    ElMessage.success('保存成功')
    editDialogVisible.value = false
  } catch {
    ElMessage.error('保存失败')
  } finally {
    saving.value = false
  }
}

const handleDelete = async (row: renderDependence.Entity) => {
  try {
    await renderDependence.remove(row.id)
    list.value = list.value.filter(item => item.id !== row.id)
    ElMessage.success('删除成功')
  } catch {
    ElMessage.error('删除失败')
  }
}

const handleDownload = async (row: renderDependence.Entity) => {
  try {
    await renderDependence.download_to_local(row.id)
    ElMessage.success('下载成功')
  } catch {
    ElMessage.error('下载失败')
  }
}

defineExpose({ loadData })
</script>
