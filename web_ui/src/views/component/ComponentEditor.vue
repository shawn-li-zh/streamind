<template>
  <el-dialog v-model="show" :title="isEdit ? '编辑组件' : '添加组件'" width="500">
    <el-form :model="form" label-width="100" :rules="rules">
      <el-form-item label="组件名称" required prop="name">
        <el-input v-model="form.name" placeholder="请输入组件名称" />
      </el-form-item>
      <el-form-item label="ESM路径" prop="path" required>
        <el-input v-model="form.path" placeholder="请输入ESM文件地址 " />
      </el-form-item>
      <el-form-item label="描述" required prop="annotation">
        <el-input v-model="form.annotation" type="textarea" :rows="3" placeholder="请输入描述" />
      </el-form-item>
      <el-form-item label="调用文档" prop="doc" required>
        <el-input v-model="form.doc" type="textarea" :rows="5" placeholder="请输入调用文档" />
      </el-form-item>
      <el-form-item label="预览参数" prop="preview_attrs">
        <el-input v-model="attrs" type="textarea" :rows="3" placeholder="请输入json数据" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="show = false">取消</el-button>
      <el-button type="primary" :loading="addFormLoading" @click="handleAddModel">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import * as module from '@/api/modules'
import { ElMessage, type FormRules } from 'element-plus'

const props = defineProps<{
  modelValue: module.renderComponent.Entity
}>()
const emits = defineEmits<{
  (e: 'update:modelValue', model: module.renderComponent.Entity): void
}>()
const isEdit = computed(() => !!props.modelValue.id)
const show = ref(false)
const form = reactive<module.renderComponent.Entity>({ ...props.modelValue })
const attrs = ref(JSON.stringify(form.preview_attrs || 'null'))
const addFormLoading = ref(false)
const rules: FormRules<module.renderComponent.Entity> = {
  name: [{ required: true, message: '请填写名称', trigger: 'blur' }],
  path: [{ required: true, message: '请填写ESM文件地址', trigger: 'blur' }],
}
const handleAddModel = async () => {
  if (!form.name || !form.path) {
    ElMessage.warning('请填写名称和调用地址')
    return
  }
  addFormLoading.value = true
  try {
    form.preview_attrs = JSON.parse(attrs.value)
  } catch (e) {
    ElMessage.warning('请填写正确的json数据')
    return
  }
  try {
    if (isEdit.value) {
      await module.renderComponent.update(form.id, form)
      ElMessage.success('更新成功')
    } else {
      const { data } = await module.renderComponent.create(form)
      form.id = data
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
