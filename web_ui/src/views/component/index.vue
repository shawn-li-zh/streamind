<template>
  <div class="h-full bg-[#f8fafc] p-6 flex flex-col gap-4">
    <!-- 顶部标题区 -->
    <div class="flex justify-between items-start">
      <div>
        <h1 class="text-xl font-bold text-slate-800 flex items-center gap-2">
          <el-icon class="text-blue-500">
            <Grid />
          </el-icon>
          {{ activeTab === 'component' ? '组件列表' : '依赖列表' }}
        </h1>
        <p class="text-sm text-slate-500 mt-1">
          {{
            activeTab === 'component'
              ? '浏览、使用和管理系统中的可用组件，支持注册新的组件。'
              : '管理系统中的依赖包，支持添加、修改、删除和下载操作。'
          }}
        </p>
      </div>
    </div>

    <!-- Tab 切换 -->
    <el-tabs v-model="activeTab" class="flex-1 flex flex-col">
      <el-tab-pane label="组件" name="component">
        <ComponentList />
      </el-tab-pane>
      <el-tab-pane label="依赖" name="dependence">
        <DependenceList ref="dependenceRef" />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { Grid } from '@element-plus/icons-vue'
import ComponentList from './ComponentList.vue'
import DependenceList from './DependenceList.vue'

const activeTab = ref('component')
const dependenceRef = ref<InstanceType<typeof DependenceList> | null>(null)
</script>

<style>
.el-tabs__content {
  flex: 1;
  overflow: hidden;
}
.el-tab-pane {
  height: 100%;
  overflow: auto;
}
</style>
