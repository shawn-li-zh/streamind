<template>
  <div class="w-full bg-white border border-slate-200 rounded-xl shadow-lg overflow-hidden font-sans">
    <!-- 头部：图标与标题 -->
    <div class="flex items-center gap-3 px-5 py-4 border-b border-slate-100 bg-slate-50/50">
      <div class="bg-blue-100 p-2 rounded-lg">
        <el-icon class="text-blue-600" :size="20">
          <Lock/>
        </el-icon>
      </div>
      <div>
        <h3 class="text-sm font-bold text-slate-800">权限授权申请</h3>
        <p class="text-[11px] text-slate-400 tracking-wider">{{ authCode }}</p>
      </div>
    </div>

    <!-- 内容区 -->
    <div class="p-5 space-y-4">
      <!-- 申请用途描述 -->
      <div class="bg-slate-50 rounded-lg p-3 border border-slate-100">
        <p class="text-xs text-slate-500 mb-1 font-medium">申请用途：</p>
        <p class="text-sm text-slate-700 leading-relaxed">{{ attrs.purpose || '请求访问相关权限以继续操作' }}</p>
      </div>

      <!-- 授权选项 -->
      <div class="space-y-3">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-slate-600">授权有效期</span>
          <!-- 紧凑的单选框 -->
          <el-radio-group v-model="type" size="small" class="!flex flex-nowrap">
            <el-radio-button v-for="(label, key) in typeLabel" :key="key" :label="key">
              {{ label }}
            </el-radio-button>
          </el-radio-group>
        </div>

        <!-- 动态输入框：次数或时长 -->
        <div v-if="type === LifeType.Times || type === LifeType.Duration"
             class="flex items-center justify-between bg-blue-50/50 p-2 rounded-lg border border-blue-100 transition-all">
          <span class="text-xs text-blue-700 font-medium ml-1">
            {{ type === LifeType.Times ? '允许调用次数' : '允许持续时长 (分钟)' }}
          </span>
          <el-input-number
              v-model="value"
              :min="1"
              size="small"
              controls-position="right"
              class="!w-24"
          />
        </div>
      </div>
    </div>

    <!-- 底部按钮区 -->
    <div class="px-5 py-4 bg-slate-50 border-t border-slate-100 flex gap-3">
      <el-button
          class="flex-1 !rounded-lg"
          @click="onReject"
          :disabled="valueRef.submit"
      >
        拒绝
      </el-button>
      <el-button
          type="primary"
          class="flex-1 !rounded-lg !bg-blue-600 hover:!bg-blue-700 border-none shadow-sm shadow-blue-200"
          @click="onAuthorize"
          :loading="valueRef.submit && !valueRef.rejected"
          :disabled="valueRef.submit && valueRef.rejected"
      >
        确认授权
      </el-button>
    </div>

    <!-- 已提交状态遮罩（可选） -->
    <div v-if="valueRef.submit"
         class="absolute inset-0 bg-white/60 backdrop-blur-[1px] flex items-center justify-center rounded-xl">
       <span
           class="bg-white px-4 py-2 rounded-full shadow-md text-xs font-medium text-slate-600 border border-slate-100">
         {{ valueRef.rejected ? '已拒绝申请' : '已完成授权' }}
       </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import {Lock} from '@element-plus/icons-vue'
import {ElButton, ElIcon, ElInputNumber, ElMessage, ElRadioButton, ElRadioGroup} from "element-plus"
import {computed, inject, ref, watch} from "vue";
import type {StreamindRuntime} from "@/streamind.ts";

const rt = inject<StreamindRuntime>("runtime")!;

// 授权有效期类型
enum LifeType {
  Once = "once",
  Times = "times",
  Duration = "duration",
  Forever = "forever",
}

const typeLabel = {
  [LifeType.Once]: "单次",
  [LifeType.Times]: "按次",
  [LifeType.Duration]: "时长",
  [LifeType.Forever]: "永久",
}

interface Attrs {
  // 授权码
  code: string;
  // 子授权码
  subCode?: string;
  // 申请用途
  purpose: string;
}

interface Value {
  submit: boolean
  rejected: boolean
  type: LifeType
  value?: number | string;
}

const props = defineProps<{
  attrs: Attrs
  value?: Value
}>();

const authCode = computed(() => {
  if (props.attrs.subCode) {
    return `${props.attrs.code}[${props.attrs.subCode}]`
  }
  return props.attrs.code;
})
const emits = defineEmits(["updateValue"])

// -- 状态管理 --
const valueRef = ref(props.value || {
  submit: false,
  rejected: false,
  type: LifeType.Once,
});

const type = ref<LifeType>(LifeType.Once);
const value = ref<number | undefined>();

// 监听类型切换
watch(type, (newType) => {
  if (newType === LifeType.Once || newType === LifeType.Forever) {
    value.value = undefined;
  } else if (newType === LifeType.Times) {
    value.value = 1;
  } else if (newType === LifeType.Duration) {
    value.value = 30;
  }
});

// -- 逻辑处理 --
async function onReject() {
  if (valueRef.value.submit) return
  await rt.sendEvent("用户拒绝了权限申请");
  valueRef.value.submit = true;
  valueRef.value.rejected = true;
  emits("updateValue", valueRef.value);
}

async function onAuthorize() {
  if (valueRef.value.submit) return

  if ((type.value === LifeType.Times || type.value === LifeType.Duration) && !value.value) {
    ElMessage.warning('请输入有效的数值');
    return;
  }

  await rt.sendEvent("用户授权了权限申请");
  valueRef.value.submit = true;
  valueRef.value.rejected = false;
  valueRef.value.type = type.value;
  valueRef.value.value = value.value;

  emits("updateValue", valueRef.value);
}
</script>

<style scoped>
/* 针对 Element Plus 样式的微调，使其更契合 Tailwind 风格 */
:deep(.el-radio-button__inner) {
  @apply px-3 py-1.5 text-xs;
}

:deep(.el-input-number.is-controls-right .el-input-number__increase) {
  @apply border-none bg-transparent hover:text-blue-600;
}

:deep(.el-input-number.is-controls-right .el-input-number__decrease) {
  @apply border-none bg-transparent hover:text-blue-600;
}

:deep(.el-input__wrapper) {
  @apply shadow-none border border-slate-200 bg-white;
}
</style>
