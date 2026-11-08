<template>
  <div
    class="h-screen bg-[#001529] transition-all duration-300 ease-in-out flex flex-col select-none"
    :class="[isCollapse ? 'w-14' : 'w-48']"
  >
    <div class="h-16 border-b flex items-center justify-center text-white font-bold">
      <el-image :src="logo" fit="cover" class="size-14"></el-image>
      {{ isCollapse ? '' : 'streamind' }}
    </div>

    <!-- router: 开启路由模式 -->
    <!-- default-active: 当前激活菜单的 index (对应当前路由 path) -->
    <el-menu
      :default-active="route.path"
      router
      :collapse-transition="false"
      class="el-menu-vertical border-none flex-1"
      style="border-right: unset"
      :collapse="isCollapse"
      background-color="#001529"
      text-color="#b7bdc3"
      active-text-color="#ffffff"
    >
      <template v-for="item in menus" :key="item.path">
        <el-menu-item :index="item.path" v-if="!item.children?.length">
          <SvgIcon
            v-if="item.icon"
            :name="item.icon + ''"
            class="mr-2 size-10!"
            :class="[route.path == item.path ? 'text-blue-500' : 'text-white']"
          />
          <el-icon v-else>
            <Menu />
          </el-icon>
          <template #title>{{ item.name }}</template>
        </el-menu-item>
        <el-sub-menu :index="item.path" v-else>
          <template #title>
            <SvgIcon
              v-if="item.icon"
              :name="item.icon + ''"
              class="mr-2 size-10!"
              :class="[route.path == item.path ? 'text-blue-500' : 'text-white']"
            />
            <el-icon v-else>
              <Menu />
            </el-icon>
            <span>{{ item.name }}</span>
          </template>
          <el-menu-item :index="sub.path" v-for="sub in item.children" :key="sub.path"
            >>{{ sub.name }}
          </el-menu-item>
        </el-sub-menu>
      </template>
    </el-menu>
    <div
      class="h-12 border-t border-gray-700 flex items-center justify-center cursor-pointer hover:bg-gray-800 text-white"
      @click="isCollapse = !isCollapse"
    >
      <el-icon>
        <Expand v-if="isCollapse" />
        <Fold v-else />
      </el-icon>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRoute } from 'vue-router'
import { Expand, Fold, Menu } from '@element-plus/icons-vue'
import SvgIcon from '@/components/SvgIcon.vue'
import logo from '@/assets/logo.png'
import { menus } from '@/router/memu.ts'

const isCollapse = ref(true)
const route = useRoute() // 获取当前路由状态
</script>
<style scoped>
:deep(.el-menu-tooltip__trigger) {
  width: unset;
}

:deep(.el-menu--collapse) {
  width: unset;
}
</style>
