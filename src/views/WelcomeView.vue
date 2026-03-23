<!--
---
role: 欢迎页，展示 7 个待安装步骤列表，检测 OS，提供"开始安装"按钮
depends:
  - ../types.ts
  - ../stores/installer.ts
  - ../router/index.ts
exports:
  - WelcomeView (default)
status: IMPLEMENTED
functions:
  - setup(): void
    onMounted:
      invoke get_system_info() → 写入 store.systemInfo
      显示 OS/架构信息
    显示 7 个步骤列表（固定顺序，带图标和描述）：
      Node.js, Git, Miniconda, Claude Code, Onboarding配置, CCSwitch, Workflow Kit
    点击"开始安装" → 导航到 /progress 并触发 store.startInstallation()

UI layout:
  - 顶部：应用标题 + 副标题
  - 中部：7 步骤列表（每行：序号 + 名称 + 描述）
  - 底部：检测到的 OS 信息 + "开始安装"按钮
---
-->

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useInstallerStore } from '../stores/installer'
import { STEP_DEFINITIONS } from '../types'
import type { SystemInfo, DetectResult } from '../types'

const router = useRouter()
const store = useInstallerStore()
const loading = ref(false)
const detectResults = ref<DetectResult[]>([])

onMounted(async () => {
  try {
    const info = await invoke<SystemInfo>('get_system_info')
    store.systemInfo = info
  } catch (e) {
    console.error('Failed to get system info:', e)
  }
  try {
    const results = await invoke<DetectResult[]>('detect_all', {
      os: store.systemInfo?.os ?? 'unknown',
      arch: store.systemInfo?.arch ?? 'unknown',
    })
    detectResults.value = results
  } catch (e) {
    console.error('Failed to detect components:', e)
  }
})

async function handleStartInstallation() {
  loading.value = true
  await router.replace('/progress')
}
</script>

<template>
  <div class="h-full bg-gray-950 text-gray-100 flex flex-col">
    <!-- Header -->
    <div class="px-8 pt-10 pb-6">
      <h1 class="text-3xl font-bold text-white">Claude 环境安装程序</h1>
      <p class="mt-2 text-gray-400 text-base">自动安装 Claude Code 所需的全部依赖组件</p>
    </div>

    <!-- Step list -->
    <div class="flex-1 px-8 overflow-y-auto">
      <p class="text-sm text-gray-500 mb-3 uppercase tracking-wide font-medium">将安装以下组件</p>
      <div class="space-y-2">
        <div
          v-for="(step, index) in STEP_DEFINITIONS"
          :key="index"
          class="flex items-center gap-4 bg-gray-900 rounded-lg px-4 py-3 border border-gray-800"
        >
          <span class="w-7 h-7 rounded-full bg-gray-700 flex items-center justify-center text-xs font-semibold text-gray-300 shrink-0">
            {{ index + 1 }}
          </span>
          <div class="flex-1 min-w-0">
            <span class="font-medium text-white text-sm">{{ step.name }}</span>
            <span class="ml-2 text-gray-500 text-sm">{{ step.desc }}</span>
          </div>
          <span
            v-if="detectResults[index]"
            class="ml-auto shrink-0 text-xs px-2 py-0.5 rounded-full"
            :class="detectResults[index].installed
              ? 'bg-green-900/50 text-green-400'
              : 'bg-gray-800 text-gray-500'"
          >
            {{ detectResults[index].installed
              ? (detectResults[index].version ? detectResults[index].version : '已安装')
              : '待安装' }}
          </span>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="px-8 py-6 border-t border-gray-800">
      <div class="flex items-center justify-between gap-6">
        <div class="text-sm text-gray-500">
          <span v-if="store.systemInfo">
            检测到系统：
            <span class="text-gray-300 font-medium">{{ store.systemInfo.os }}</span>
            <span class="mx-1">/</span>
            <span class="text-gray-300 font-medium">{{ store.systemInfo.arch }}</span>
          </span>
          <span v-else class="text-gray-600">正在检测系统信息...</span>
          <p class="mt-1 text-xs text-gray-600">
            如果 Windows 首次启动提示缺少 WebView2，可先手动安装 Microsoft Edge WebView2 Runtime 后再重试。
          </p>
        </div>
        <button
          @click="handleStartInstallation"
          :disabled="loading"
          class="px-6 py-2.5 bg-blue-600 hover:bg-blue-500 disabled:bg-blue-800 disabled:cursor-not-allowed text-white font-semibold rounded-lg transition-colors text-sm"
        >
          {{ loading ? '正在启动...' : '开始安装' }}
        </button>
      </div>
    </div>
  </div>
</template>
