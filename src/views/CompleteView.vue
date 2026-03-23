<!--
---
role: 安装完成页，展示安装结果总结和后续指引
depends:
  - ../types.ts
  - ../stores/installer.ts
exports:
  - CompleteView (default)
status: IMPLEMENTED
functions:
  - setup(): void
    从 store.installationResult 读取 7 步结果
    显示整体状态（全部成功 / 部分失败）
    提供操作按钮：
      "打开 CCSwitch 所在目录"（invoke open_path 传入下载路径）
      "关闭"（关闭窗口或退出应用）

UI layout:
  - 顶部：成功/部分失败的大图标 + 标题
  - 中部：7 步结果汇总列表（done✓ / skipped⊙ / error✗ + 各自版本或错误信息）
  - CCSwitch 引导区：
    - 官方源用户：可选使用 CCSwitch（说明用途）
    - 通用：下载位置路径 + "打开文件夹"按钮
  - 底部："关闭"按钮 + 快速上手提示（运行 `claude` 开始使用）
---
-->

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { exit } from '@tauri-apps/plugin-process'
import { useInstallerStore } from '../stores/installer'
import { STEP_DEFINITIONS } from '../types'

const store = useInstallerStore()

// Workflow Kit update state
const workflowInfo = ref<{ installed: boolean; version: string | null; lastUpdated: string } | null>(null)
const isUpdating = ref(false)
const updateResult = ref<{ success: boolean; message: string } | null>(null)

onMounted(async () => {
  await loadWorkflowInfo()
})

async function loadWorkflowInfo() {
  try {
    const info = await invoke('get_workflow_kit_info') as { installed: boolean; version: string | null; lastUpdated: string }
    workflowInfo.value = info
  } catch (e) {
    console.error('Failed to get workflow kit info:', e)
  }
}

async function updateWorkflowKit() {
  if (isUpdating.value) return
  isUpdating.value = true
  updateResult.value = null

  try {
    const result = await invoke('update_workflow_kit') as { status: string; message: string }
    updateResult.value = {
      success: result.status === 'done',
      message: result.message
    }
    // Reload info after update
    await loadWorkflowInfo()
  } catch (e) {
    updateResult.value = {
      success: false,
      message: '更新失败: ' + String(e)
    }
  } finally {
    isUpdating.value = false
  }
}

const isFullSuccess = computed(() => {
  return store.result ? store.result.error_count === 0 : false
})

const ccSwitchPath = computed(() => {
  // Try store.ccSwitchDownloadPath first
  if (store.ccSwitchDownloadPath) return store.ccSwitchDownloadPath
  // Fallback: parse from result steps[5].message
  // Handle formats:
  // - "CCSwitch 已下载到 C:\Users\...\CC-Switch-xxx.msi"
  // - "已安装，跳过。安装包位置: C:\Users\..."
  if (store.result?.steps?.[5]?.message) {
    const msg = store.result.steps[5].message
    // Try to extract path after "安装包位置: " or "已下载到 "
    const locationMatch = msg.match(/(?:安装包位置:|已下载到)\s+(.+?)(?:\s*$|\.msi|\.zip|\.deb|\.AppImage)/i)
    if (locationMatch) {
      let fullPath = locationMatch[1].trim()
      // If it's a file path, get the directory
      const lastSep = Math.max(fullPath.lastIndexOf('/'), fullPath.lastIndexOf('\\'))
      if (lastSep > 0) {
        // Check if it ends with a filename (has extension)
        const lastDot = fullPath.lastIndexOf('.', lastSep)
        if (lastDot > lastSep || fullPath.match(/\.(msi|zip|deb|AppImage)$/i)) {
          return fullPath.substring(0, lastSep)
        }
      }
      return fullPath
    }
    // Fallback: try to match any Windows or Unix path
    const pathMatch = msg.match(/([A-Za-z]:[\\\/][^\s]+|\/[^\s]+)/)
    if (pathMatch) {
      const fullPath = pathMatch[1]
      const lastSep = Math.max(fullPath.lastIndexOf('/'), fullPath.lastIndexOf('\\'))
      return lastSep > 0 ? fullPath.substring(0, lastSep) : fullPath
    }
  }
  return null
})

function getStatusIcon(status: string): string {
  switch (status) {
    case 'done': return '✓'
    case 'skipped': return '⊙'
    case 'error': return '✗'
    default: return '○'
  }
}

function getStatusClass(status: string): string {
  switch (status) {
    case 'done': return 'text-green-400'
    case 'skipped': return 'text-gray-500'
    case 'error': return 'text-red-400'
    default: return 'text-gray-600'
  }
}

function getRowClass(status: string): string {
  switch (status) {
    case 'done': return 'border-green-900 bg-green-950/20'
    case 'skipped': return 'border-gray-800 bg-gray-900/40'
    case 'error': return 'border-red-900 bg-red-950/20'
    default: return 'border-gray-800 bg-gray-900/30'
  }
}

function getStepSubtext(step: { status: string; message: string; version?: string }): string {
  if (step.status === 'done' && step.version) return step.version
  if (step.status === 'done') return '安装成功'
  if (step.status === 'skipped') return '已安装，已跳过'
  if (step.status === 'error') return step.message || '安装失败'
  return ''
}

async function openCCSwitchDirectory() {
  if (ccSwitchPath.value) {
    try {
      await invoke('open_path', { path: ccSwitchPath.value })
    } catch (e) {
      console.error('Failed to open path:', e)
    }
  }
}

async function handleExit() {
  try {
    await exit(0)
  } catch (e) {
    console.error('Failed to exit:', e)
  }
}

// Get a merged list of step results - fallback to store.steps if result not available
const stepResults = computed(() => {
  if (store.result?.steps?.length) {
    return store.result.steps.map((r, i) => ({
      name: STEP_DEFINITIONS[i]?.name ?? r.name,
      status: r.status,
      message: r.message,
      version: r.version,
    }))
  }
  return store.steps.map((s, i) => ({
    name: STEP_DEFINITIONS[i]?.name ?? s.name,
    status: s.status as string,
    message: s.message,
    version: s.version,
  }))
})
</script>

<template>
  <div class="h-full bg-gray-950 text-gray-100 flex flex-col">
    <!-- Header -->
    <div class="px-8 pt-10 pb-5">
      <div class="flex items-center gap-4">
        <span class="text-4xl" :class="isFullSuccess ? 'text-green-400' : 'text-yellow-400'">
          {{ isFullSuccess ? '✓' : '⚠' }}
        </span>
        <div>
          <h1 class="text-2xl font-bold text-white">
            {{ isFullSuccess ? '安装完成' : '安装完成（部分失败）' }}
          </h1>
          <p class="mt-0.5 text-sm text-gray-400" v-if="store.result">
            成功 {{ store.result.success_count }}
            <span class="mx-1 text-gray-600">·</span>
            跳过 {{ store.result.skip_count }}
            <span v-if="store.result.error_count > 0">
              <span class="mx-1 text-gray-600">·</span>
              <span class="text-red-400">失败 {{ store.result.error_count }}</span>
            </span>
          </p>
        </div>
      </div>
    </div>

    <!-- Step results list -->
    <div class="flex-1 px-8 overflow-y-auto space-y-2 pb-2">
      <div
        v-for="(step, index) in stepResults"
        :key="index"
        class="flex items-center gap-4 rounded-lg px-4 py-3 border"
        :class="getRowClass(step.status)"
      >
        <span
          class="w-6 h-6 flex items-center justify-center text-base font-bold shrink-0"
          :class="getStatusClass(step.status)"
        >
          {{ getStatusIcon(step.status) }}
        </span>
        <div class="flex-1 min-w-0">
          <span class="font-medium text-sm text-white">{{ step.name }}</span>
          <span
            v-if="getStepSubtext(step)"
            class="ml-2 text-xs"
            :class="{
              'text-green-400': step.status === 'done',
              'text-gray-500': step.status === 'skipped',
              'text-red-400': step.status === 'error',
            }"
          >
            {{ getStepSubtext(step) }}
          </span>
        </div>
      </div>
    </div>

    <!-- Workflow Kit update section -->
    <div class="px-8 py-3 border-t border-gray-800">
      <div class="bg-gradient-to-r from-blue-900/20 to-indigo-900/20 rounded-lg border border-blue-800/40 p-4">
        <div class="flex items-start justify-between gap-4">
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <h3 class="text-sm font-semibold text-gray-200">Workflow Kit — 工作流套件</h3>
              <span v-if="workflowInfo?.installed" class="px-1.5 py-0.5 text-[10px] bg-green-900/50 text-green-400 rounded border border-green-800/50">
                已安装
              </span>
            </div>
            <p class="mt-1 text-xs text-gray-500 leading-relaxed">
              包含 discovery、planning、execution 等 14 个技能的完整工作流套件。
              点击更新可从 GitHub 获取最新版本。
            </p>
            <p v-if="workflowInfo?.lastUpdated" class="mt-1.5 text-xs text-gray-600">
              最后更新: {{ workflowInfo.lastUpdated }}
            </p>
            <p v-if="updateResult" class="mt-2 text-xs" :class="updateResult.success ? 'text-green-400' : 'text-red-400'">
              {{ updateResult.message }}
            </p>
          </div>
          <button
            @click="updateWorkflowKit"
            :disabled="isUpdating"
            class="shrink-0 px-4 py-2 text-xs bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 disabled:cursor-not-allowed text-white font-medium rounded-md transition-colors flex items-center gap-2"
          >
            <span v-if="isUpdating" class="w-3 h-3 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
            {{ isUpdating ? '更新中...' : '检查更新' }}
          </button>
        </div>
      </div>
    </div>

    <!-- CCSwitch guide section -->
    <div class="px-8 py-3 border-t border-gray-800">
      <div class="bg-gray-900 rounded-lg border border-gray-800 p-4">
        <div class="flex items-start justify-between gap-4">
          <div class="flex-1 min-w-0">
            <h3 class="text-sm font-semibold text-gray-200">CCSwitch — API 源切换工具</h3>
            <p class="mt-1 text-xs text-gray-500 leading-relaxed">
              用于在官方 API 源与第三方镜像源之间切换。如果你使用官方源，可跳过此步骤；
              如需切换源，双击安装包即可完成安装。
            </p>
            <p v-if="ccSwitchPath" class="mt-1.5 text-xs text-gray-600 font-mono truncate">
              {{ ccSwitchPath }}
            </p>
            <p v-else class="mt-1.5 text-xs text-gray-600">
              安装包路径未检测到（CCSwitch 步骤可能已跳过）
            </p>
          </div>
          <button
            v-if="ccSwitchPath"
            @click="openCCSwitchDirectory"
            class="shrink-0 px-3 py-1.5 text-xs bg-gray-800 hover:bg-gray-700 border border-gray-700 text-gray-300 rounded-md transition-colors"
          >
            打开目录
          </button>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="px-8 pb-8 pt-3 flex items-center justify-between">
      <div class="text-sm text-gray-500">
        <div>
          在终端运行
          <code class="ml-1 px-1.5 py-0.5 bg-gray-800 rounded text-gray-300 font-mono text-xs">claude</code>
          开始使用
        </div>
        <div class="mt-1 text-xs text-gray-600">
          若 Windows 打开失败且提示缺少 WebView2，请先手动安装 Microsoft Edge WebView2 Runtime。
        </div>
      </div>
      <button
        @click="handleExit"
        class="px-6 py-2.5 bg-gray-700 hover:bg-gray-600 text-white font-semibold rounded-lg transition-colors text-sm"
      >
        关闭
      </button>
    </div>
  </div>
</template>
