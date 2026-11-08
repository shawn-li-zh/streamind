<template>
  <el-dialog v-model="show" :title="isEdit ? '编辑模型' : '添加模型'" width="500">
    <el-form :model="form" label-width="100" :rules="rules">
      <el-form-item label="模型名称" required prop="name">
        <el-input v-model="form.name" placeholder="请输入模型名称" />
      </el-form-item>
      <el-form-item label="备注" prop="remark">
        <el-input v-model="form.remark" type="textarea" :rows="3" placeholder="请输入备注" />
      </el-form-item>
      <el-form-item label="调用地址" required prop="api_url">
        <el-input v-model="form.api_url" placeholder="请输入API调用地址" />
      </el-form-item>
      <el-form-item label="API密钥" prop="api_key">
        <el-input v-model="form.api_key" placeholder="请输入API密钥" />
      </el-form-item>
      <el-form-item label="输入模态" required prop="input_modals">
        <el-checkbox-group v-model="form.input_modals">
          <el-checkbox label="text">文本</el-checkbox>
          <el-checkbox label="image">图片</el-checkbox>
          <el-checkbox label="audio">音频</el-checkbox>
          <el-checkbox label="video">视频</el-checkbox>
        </el-checkbox-group>
      </el-form-item>
      <el-form-item label="输出模态" required prop="output_modals">
        <el-checkbox-group v-model="form.output_modals">
          <el-checkbox label="text">文本</el-checkbox>
          <el-checkbox label="image">图片</el-checkbox>
          <el-checkbox label="audio">音频</el-checkbox>
          <el-checkbox label="video">视频</el-checkbox>
        </el-checkbox-group>
      </el-form-item>
      <el-form-item label="调用成本" required prop="cost">
        <el-radio-group v-model="form.cost">
          <el-radio value="low">低</el-radio>
          <el-radio value="medium">中</el-radio>
          <el-radio value="high">高</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item label="状态" v-if="isEdit" prop="status">
        <el-radio-group v-model="form.status">
          <el-radio value="active">启用</el-radio>
          <el-radio value="inactive">禁用</el-radio>
        </el-radio-group>
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
  modelValue: module.model.Entity
}>()
const emits = defineEmits<{
  (e: 'update:modelValue', model: module.model.Entity): void
}>()
const isEdit = computed(() => !!props.modelValue.id)
const show = ref(false)

const form = reactive<module.model.Entity>({ ...props.modelValue })

const addFormLoading = ref(false)
const rules: FormRules<module.model.Entity> = {
  name: [{ required: true, message: '请填写名称', trigger: 'blur' }],
  api_url: [
    { required: true, message: '请填写API调用地址', trigger: 'blur' },
    { pattern: /^https?:\/\/.+/, message: '请填写正确的API调用地址', trigger: 'blur' },
  ],
}
const handleAddModel = async () => {
  if (!form.name || !form.api_url) {
    ElMessage.warning('请填写名称和调用地址')
    return
  }
  addFormLoading.value = true
  try {
    if (isEdit.value) {
      await module.model.update(form.id, form)
      ElMessage.success('更新成功')
    } else {
      const res = await module.model.create(form)
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
