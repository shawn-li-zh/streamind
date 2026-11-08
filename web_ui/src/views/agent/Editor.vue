<template>
  <!-- 添加模型弹窗 -->
  <el-dialog v-model="show" :title="isEdit ? '编辑智能体' : '添加智能体'" width="500">
    <el-form :model="form" label-width="100" :rules="rules">
      <el-form-item label="名称" required prop="name">
        <el-input v-model="form.name" placeholder="请输入智能体名称" />
      </el-form-item>
      <el-form-item label="头像" required prop="avatar">
        <div class="flex flex-row">
          <el-avatar :size="64" shape="square" :src="avatar" />
          <el-icon class="ml-2 cursor-pointer" @click="random = Math.random()" size="18">
            <Refresh />
          </el-icon>
        </div>
      </el-form-item>
      <el-form-item label="模型" required prop="model_id">
        <el-select v-model="form.model_id" placeholder="请选择模型">
          <el-option v-for="item in models" :key="item.id" :label="item.name" :value="item.id">
            <div class="flex items-center">
              <span class="">{{ item.name }}</span>
              <div class="ml-auto text-[11px] text-gray-400 truncate">{{ item.remark }}</div>
            </div>
          </el-option>
        </el-select>
      </el-form-item>
      <el-form-item label="人设" prop="persona">
        <el-input v-model="form.persona" type="textarea" :rows="3" placeholder="请输入人设" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="show = false">取消</el-button>
      <el-button type="primary" :loading="addFormLoading" @click="handleAddModel">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import * as module from '@/api/modules'
import { ElMessage } from 'element-plus'

import type { FormRules } from 'tdesign-vue-next/es/form/type'
import { Refresh } from '@element-plus/icons-vue'

const props = defineProps<{
  modelValue: module.agent.Entity
}>()
const emits = defineEmits<{
  (e: 'update:modelValue', model: module.agent.Entity): void
}>()
const isEdit = computed(() => !!props.modelValue.id)
const show = ref(false)
const form = reactive<module.agent.Entity>({
  ...props.modelValue,
} as any)
const random = ref(Math.random())
const avatar = computed(() => {
  if (form.avatar) {
    return form.avatar
  }
  if (!form.name) {
    ElMessage.warning('请输入模型名称')
    return ''
  }
  return `https://api.dicebear.com/9.x/bottts/svg?scale=90&seed=${form.name || 'Felix'}-${random.value}`
})
const addFormLoading = ref(false)
//rule
const rules: FormRules<module.agent.Entity> = {
  name: [
    { required: true, message: '请输入模型名称', trigger: 'blur' },
    { min: 2, max: 10, message: '长度在 2 到 10 个字符', trigger: 'blur' },
  ],
  avatar: [{ required: true, message: '请上传模型头像', trigger: 'blur' }],
}

const models = ref<module.model.Entity[]>([])
onMounted(() => {
  module.model.list().then(res => {
    models.value = res.data
  })
})
const handleAddModel = async () => {
  addFormLoading.value = true
  try {
    form.avatar = avatar.value
    if (isEdit.value) {
      await module.agent.update(form.id, form)
      ElMessage.success('更新成功')
    } else {
      const res = await module.agent.create(form)
      form.id = res.data
      ElMessage.success('添加成功')
    }
    emits('update:modelValue', form)
    show.value = false
  } finally {
    addFormLoading.value = false
  }
}
defineExpose({
  show,
  form,
})
</script>

<style>
/* 可以在此处微调 Element Plus 样式 */
.el-table {
  --el-table-header-bg-color: #f9fafb;
}
</style>
