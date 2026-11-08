<template>
  <el-table
    :data="data"
    style="width: 100%"
    height="100%"
    class="custom-table"
    header-cell-class-name="!bg-slate-50 !text-slate-600 font-medium py-4"
  >
    <!-- 组件名称 -->
    <el-table-column label="组件名称" min-width="240">
      <template #default="{ row }">
        <div class="flex items-center py-1">
          <div class="flex flex-col">
            <span class="font-bold text-slate-700 leading-tight">{{ row.name }}</span>
            <span class="text-xs text-slate-400 mt-1 font-mono">{{ row.path }}</span>
          </div>
        </div>
      </template>
    </el-table-column>

    <!-- 作者 -->
    <el-table-column label="作者" width="200">
      <template #default="{ row }">
        <div class="flex items-center gap-2">
          <!-- 身份标签 -->
          <el-tag
            size="small"
            :type="getAuthorTag(row.authorType).type"
            effect="plain"
            class="!border-none !bg-opacity-10 font-medium"
            :style="{
              backgroundColor: getAuthorTag(row.authorType).color + '20',
              color: getAuthorTag(row.authorType).color,
            }"
          >
            {{ getAuthorTag(row.authorType).label }}
          </el-tag>
          <!-- 作者名字 -->
          <div class="flex items-center text-slate-600 text-sm">
            <el-icon class="mr-1">
              <User v-if="row.authorType !== 'official'" />
              <Monitor v-else />
            </el-icon>
            {{ row.authorName }}
          </div>
        </div>
      </template>
    </el-table-column>

    <!-- 用途 -->
    <el-table-column label="描述" min-width="350">
      <template #default="{ row }">
        <p class="text-slate-500 text-sm leading-relaxed line-clamp-2">
          {{ row.annotation }}
        </p>
      </template>
    </el-table-column>

    <!-- 操作 -->
    <el-table-column label="操作" width="120" align="right" fixed="right">
      <template #default="{ row }">
        <div class="flex items-center justify-end gap-1">
          <el-button
            link
            type="primary"
            class="!font-normal"
            @click="((previewItem = row), (preview = true))"
          >
            <el-icon class="mr-1">
              <View />
            </el-icon>
            预览
          </el-button>
          <el-dropdown trigger="click">
            <el-button link class="!p-1">
              <el-icon class="rotate-90 text-slate-400">
                <MoreFilled />
              </el-icon>
            </el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item :icon="Edit" @click="emits('edit', row)"
                  >编辑组件
                </el-dropdown-item>
                <el-dropdown-item :icon="CopyDocument">复制 Key</el-dropdown-item>
                <el-dropdown-item divided :icon="Delete" class="!text-red-500"
                  >删除
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </template>
    </el-table-column>
  </el-table>
  <el-dialog v-model="preview" title="组件预览" :key="previewItem.path">
    <div class="p-4">
      <!--      组件-->
      <component :is="RemoteComp" :attrs="previewItem.preview_attrs" />
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import {
  CopyDocument,
  Delete,
  Edit,
  Monitor,
  MoreFilled,
  User,
  View,
} from '@element-plus/icons-vue'
import * as module from '@/api/modules'
import { computed, defineAsyncComponent, ref } from 'vue'
import requestHttp from '@/api'

defineProps<{ data: module.renderComponent.Entity[] }>()
const preview = ref(false)
const previewItem = ref<module.renderComponent.Entity>({} as any)

const RemoteComp = computed(() => {
  const url = requestHttp.getUri(`/public/components/${previewItem.value.path}`)
  return defineAsyncComponent(() => import(url))
})
// 作者标签样式映射
const getAuthorTag = (type: string) => {
  const map: Record<string, any> = {
    official: { label: '官方', color: '#3b82f6', type: 'primary' },
    agent: { label: 'Agent', color: '#8b5cf6', type: 'warning' },
    self: { label: '自注册', color: '#10b981', type: 'success' },
    others: { label: '其他用户', color: '#f59e0b', type: 'info' },
  }
  return map[type] || map.others
}
const emits = defineEmits(['edit'])
</script>

<style scoped>
.custom-table {
  --el-table-border-color: #f1f5f9;
  --el-table-row-hover-bg-color: #f8fafc;
  border-radius: 12px;
  overflow: hidden;
  border: 1px solid #f1f5f9;
}

/* 去掉 Table 默认的边框线条感，使其更现代 */
:deep(.el-table__inner-wrapper::before) {
  display: none;
}
</style>
