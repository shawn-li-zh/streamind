<template>
  <!-- 动态背景，支持暗黑模式平滑过渡 -->
  <div
    class="relative min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-900 transition-colors duration-500 overflow-hidden w-full"
  >
    <!-- 背景装饰元素：光晕效果 -->
    <div
      class="absolute top-1/4 left-1/4 w-96 h-96 bg-blue-400 dark:bg-blue-600 rounded-full mix-blend-multiply dark:mix-blend-overlay filter blur-3xl opacity-20 animate-blob"
    ></div>
    <div
      class="absolute top-1/3 right-1/4 w-96 h-96 bg-teal-300 dark:bg-teal-700 rounded-full mix-blend-multiply dark:mix-blend-overlay filter blur-3xl opacity-20 animate-blob animation-delay-2000"
    ></div>

    <!-- 顶部暗黑模式切换按钮 -->
    <div class="absolute top-6 right-6">
      <el-switch
        v-model="isDark"
        inline-prompt
        :active-icon="Moon"
        :inactive-icon="Sunny"
        style="--el-switch-on-color: #4b5563; --el-switch-off-color: #e5e7eb"
        @change="toggleTheme"
      />
    </div>

    <!-- 登录卡片 (毛玻璃质感) -->
    <div
      class="relative z-10 w-full max-w-md p-10 bg-white/80 dark:bg-gray-800/80 backdrop-blur-xl rounded-3xl shadow-2xl border border-white/50 dark:border-gray-700/50 transition-all duration-300"
    >
      <!-- Logo 与 标题 -->
      <div class="text-center mb-10">
        <div
          class="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-gradient-to-br from-blue-500 to-teal-400 mb-6 shadow-lg shadow-blue-500/30"
        >
          <el-image :src="logo" fit="cover" class="size-18"></el-image>
        </div>
        <h1
          class="text-4xl font-black tracking-tight text-transparent bg-clip-text bg-gradient-to-r from-gray-900 to-gray-600 dark:from-white dark:to-gray-300"
        >
          Streamind
        </h1>
        <p class="mt-3 text-sm font-medium text-gray-500 dark:text-gray-400">
          Agent Framework · 智能节点接入
        </p>
      </div>

      <!-- 登录表单 -->
      <el-form
        ref="loginFormRef"
        :model="loginForm"
        :rules="rules"
        @keyup.enter="handleLogin(loginFormRef)"
        class="space-y-6"
      >
        <el-form-item prop="code">
          <div class="w-full relative group">
            <el-input
              v-model="loginForm.code"
              placeholder="请输入控制台打印的 Access Code"
              size="large"
              clearable
              class="streamind-input"
            >
              <template #prefix>
                <el-icon class="text-gray-400 group-focus-within:text-blue-500 transition-colors">
                  <Key />
                </el-icon>
              </template>
            </el-input>
          </div>
        </el-form-item>

        <el-form-item class="!mt-8">
          <el-button
            type="primary"
            size="large"
            :loading="loading"
            class="w-full h-12 text-base font-semibold rounded-xl bg-gradient-to-r from-blue-500 to-teal-400 border-0 shadow-lg shadow-blue-500/30 hover:shadow-blue-500/50 hover:-translate-y-0.5 transition-all duration-300"
            @click="handleLogin(loginFormRef)"
          >
            连接终端
          </el-button>
        </el-form-item>
      </el-form>

      <div class="mt-6 text-center text-xs text-gray-400 dark:text-gray-500">
        请在 Rust 后端终端中查看随机生成的 16 进制登录码
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElLoading, ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Key, Moon, Sunny } from '@element-plus/icons-vue'
import logo from '@/assets/logo.png'
import * as module from '@/api/modules'
import { useAppStore } from '@/stores/app.ts'
import { useAgentStore } from '@/stores/agent.ts'

const router = useRouter()
const loginFormRef = ref<FormInstance>()
const loading = ref(false)
const isDark = ref(false)
const appStore = useAppStore()
const agentStore = useAgentStore()
// 表单数据
const loginForm = reactive({
  code: '',
})

// 表单校验规则
const rules = reactive<FormRules>({
  code: [
    { required: true, message: '接入码不能为空', trigger: 'blur' },
    { min: 6, message: '请完整输入后台生成的接入码', trigger: 'blur' },
  ],
})

// 主题切换逻辑
const toggleTheme = (val: string | number | boolean) => {
  const isDarkTheme = !!val
  if (isDarkTheme) {
    document.documentElement.classList.add('dark')
    localStorage.setItem('streamind_theme', 'dark')
  } else {
    document.documentElement.classList.remove('dark')
    localStorage.setItem('streamind_theme', 'light')
  }
}

// 初始化时检查主题
onMounted(() => {})

// 登录提交
const handleLogin = async (formEl: FormInstance | undefined) => {
  if (!formEl) return
  await formEl.validate(async valid => {
    if (valid) {
      loading.value = true
      try {
        const { data } = await module.app.login(loginForm.code)
        // 模拟网络请求延迟
        await new Promise(resolve => setTimeout(resolve, 800))
        appStore.token = data
        await init()
        // 跳转到控制台首页
        await router.push('/')
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
      } catch (error) {
        ElMessage.error('连接失败，请检查登录码是否正确')
      } finally {
        loading.value = false
      }
    }
  })
}

async function init() {
  const service = ElLoading.service({
    lock: true,
    text: '正在初始化 Agent 环境...',
    background: 'rgba(0, 0, 0, 0.7)',
  })
  try {
    await agentStore.init()
  } finally {
    service.close()
  }
}
</script>

<style scoped>
/* Element Plus 输入框深度定制，融入 Tailwind 风格 */
:deep(.streamind-input .el-input__wrapper) {
  @apply bg-gray-50 dark:bg-gray-900 border-none shadow-[inset_0_2px_4px_rgba(0,0,0,0.05)] dark:shadow-[inset_0_2px_4px_rgba(0,0,0,0.2)] rounded-xl h-12 transition-all duration-300;
}

:deep(.streamind-input .el-input__wrapper.is-focus) {
  @apply bg-white dark:bg-gray-800 ring-2 ring-blue-500 shadow-lg shadow-blue-500/10;
}

:deep(.streamind-input .el-input__inner) {
  @apply text-base font-medium text-gray-700 dark:text-gray-200 placeholder-gray-400;
}

/* Tailwind 动画拓展 (如果你没有在 tailwind.config.js 配置，这些手写动画会生效) */
@keyframes blob {
  0% {
    transform: translate(0px, 0px) scale(1);
  }
  33% {
    transform: translate(30px, -50px) scale(1.1);
  }
  66% {
    transform: translate(-20px, 20px) scale(0.9);
  }
  100% {
    transform: translate(0px, 0px) scale(1);
  }
}

.animate-blob {
  animation: blob 7s infinite;
}

.animation-delay-2000 {
  animation-delay: 2s;
}
</style>
