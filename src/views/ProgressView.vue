<!--
---
role: 安装进度页，实时展示 7 个步骤的执行状态，禁止后退
depends:
  - ../types.ts
  - ../stores/installer.ts
  - ../router/index.ts
exports:
  - ProgressView (default)
status: IMPLEMENTED
functions:
  - setup(): void
    onMounted:
      监听 Tauri 事件 "step-update" → 更新 store 中对应步骤状态
      监听 Tauri 事件 "installation-complete" → 跳转 /complete
    展示 7 步骤列表，每步状态：
      waiting（灰色）| running（蓝色 + 转圈）| done（绿色 ✓ + 版本号）| skipped（灰色 ⊙）| error（红色 ✗ + 错误信息）
    底部：实时日志滚动区（最新 20 条）
    步骤全部结束自动跳转 /complete（不提供手动跳转按钮）

UI layout:
  - 顶部：安装进度标题 + 总体进度条（completed/7）
  - 中部：7 步骤状态列表
  - 底部：可折叠的日志区域
---
-->

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import { useInstallerStore } from '../stores/installer'
import type { StepUpdate, InstallationResult } from '../types'

const router = useRouter()
const store = useInstallerStore()
const logLines = ref<string[]>([])
const logContainer = ref<HTMLElement | null>(null)
const logExpanded = ref(false)

// Unlisten functions for cleanup
let unlistenStepUpdate: (() => void) | null = null
let unlistenComplete: (() => void) | null = null

const completedCount = computed(() => {
  return store.steps.filter((s) => s.status === 'done' || s.status === 'skipped' || s.status === 'error').length
})

const progressPercent = computed(() => {
  return Math.round((completedCount.value / store.steps.length) * 100)
})

function addLog(message: string) {
  const now = new Date()
  const time = `${now.getHours().toString().padStart(2, '0')}:${now.getMinutes().toString().padStart(2, '0')}:${now.getSeconds().toString().padStart(2, '0')}`
  logLines.value.push(`[${time}] ${message}`)
  if (logLines.value.length > 20) {
    logLines.value.shift()
  }
  nextTick(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight
    }
  })
}

function getStatusIcon(status: string): string {
  switch (status) {
    case 'done': return '✓'
    case 'skipped': return '⊙'
    case 'error': return '✗'
    case 'running': return '⟳'
    default: return '○'
  }
}

function getStatusClass(status: string): string {
  switch (status) {
    case 'done': return 'text-green-400'
    case 'skipped': return 'text-gray-500'
    case 'error': return 'text-red-400'
    case 'running': return 'text-blue-400 animate-spin'
    default: return 'text-gray-600'
  }
}

function getRowClass(status: string): string {
  switch (status) {
    case 'done': return 'border-green-900 bg-green-950/30'
    case 'skipped': return 'border-gray-800 bg-gray-900/50'
    case 'error': return 'border-red-900 bg-red-950/30'
    case 'running': return 'border-blue-800 bg-blue-950/30'
    default: return 'border-gray-800 bg-gray-900/30'
  }
}

function getStepSubtext(step: { status: string; message: string; version?: string }): string {
  if (step.status === 'done' && step.version) return step.version
  if (step.status === 'done') return '安装成功'
  if (step.status === 'skipped') return '已安装，已跳过'
  if (step.status === 'running') return '安装中...'
  if (step.status === 'error') return step.message || '安装失败'
  return ''
}

onMounted(async () => {
  addLog('正在初始化事件监听...')

  unlistenStepUpdate = await listen<StepUpdate>('step-update', (event) => {
    const update = event.payload
    addLog(`[${update.name}] ${update.status}: ${update.message}`)
    const step = store.steps[update.index]
    if (step) {
      step.status = update.status as any
      step.message = update.message
      if (update.version !== undefined) {
        step.version = update.version
      }
    }
    // 提取 CCSwitch 路径（无论是 done 还是 skipped，只要是 CCSwitch 步骤）
    if (update.index === 5 && (update.status === 'done' || update.status === 'skipped')) {
      const msg = update.message
      // Try to extract path after "安装包位置: " or "已下载到 "
      const locationMatch = msg.match(/(?:安装包位置:|已下载到)\s+(.+?)(?:\s*$|\.msi|\.zip|\.deb|\.AppImage)/i)
      if (locationMatch) {
        let fullPath = locationMatch[1].trim()
        const lastSep = Math.max(fullPath.lastIndexOf('/'), fullPath.lastIndexOf('\\'))
        if (lastSep > 0) {
          // Check if it ends with a filename (has extension in the last segment)
          const lastSegment = fullPath.substring(lastSep + 1)
          if (lastSegment.match(/\.(msi|zip|deb|AppImage)$/i)) {
            fullPath = fullPath.substring(0, lastSep)
          }
        }
        store.ccSwitchDownloadPath = fullPath
      } else {
        // Fallback: try to match any Windows or Unix path
        const pathMatch = msg.match(/([A-Za-z]:[\\\/][^\s]+|\/[^\s]+)/)
        if (pathMatch) {
          const fullPath = pathMatch[1]
          const lastSep = Math.max(fullPath.lastIndexOf('/'), fullPath.lastIndexOf('\\'))
          store.ccSwitchDownloadPath = lastSep > 0 ? fullPath.substring(0, lastSep) : fullPath
        }
      }
    }
  })

  unlistenComplete = await listen<InstallationResult>('installation-complete', (event) => {
    addLog('安装流程结束，正在跳转...')
    store.result = event.payload
    store.isComplete = true

    if (event.payload.steps && event.payload.steps[5]) {
      const msg = event.payload.steps[5].message
      const pathMatch = msg.match(/([A-Za-z]:[\\\/][^\s]+|\/[^\s]+)/)
      if (pathMatch) {
        const fullPath = pathMatch[1]
        const lastSep = Math.max(fullPath.lastIndexOf('/'), fullPath.lastIndexOf('\\'))
        store.ccSwitchDownloadPath = lastSep > 0 ? fullPath.substring(0, lastSep) : fullPath
      }
    }

    setTimeout(() => router.replace('/complete'), 1000)
  })

  addLog('事件监听就绪，开始安装...')
  try {
    await store.startInstallation()
  } catch (e) {
    addLog(`安装启动失败: ${e}`)
  }
})

onUnmounted(() => {
  if (unlistenStepUpdate) unlistenStepUpdate()
  if (unlistenComplete) unlistenComplete()
})

// Watch for isComplete from store (set by store listener)
watch(() => store.isComplete, (val) => {
  if (val) {
    setTimeout(() => {
      router.replace('/complete')
    }, 1000)
  }
})
</script>

<template>
  <div class="h-full bg-gray-950 text-gray-100 flex flex-col">
    <!-- Header with progress bar -->
    <div class="px-8 pt-10 pb-6">
      <h1 class="text-2xl font-bold text-white">正在安装</h1>
      <p class="mt-1 text-gray-400 text-sm">请耐心等待，安装完成后将自动跳转</p>

      <div class="mt-4">
        <div class="flex items-center justify-between text-sm mb-1.5">
          <span class="text-gray-400">总进度</span>
          <span class="text-gray-300 font-medium">{{ completedCount }} / {{ store.steps.length }}</span>
        </div>
        <div class="w-full bg-gray-800 rounded-full h-2">
          <div
            class="bg-blue-500 h-2 rounded-full transition-all duration-500"
            :style="{ width: progressPercent + '%' }"
          ></div>
        </div>
      </div>
    </div>

    <!-- Step list -->
    <div class="flex-1 px-8 overflow-y-auto space-y-2 pb-2">
      <div
        v-for="(step, index) in store.steps"
        :key="index"
        class="flex items-center gap-4 rounded-lg px-4 py-3 border transition-all duration-300"
        :class="getRowClass(step.status)"
      >
        <!-- Status icon -->
        <span
          class="w-6 h-6 flex items-center justify-center text-base font-bold shrink-0"
          :class="getStatusClass(step.status)"
        >
          {{ getStatusIcon(step.status) }}
        </span>

        <!-- Step info -->
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-2">
            <span class="font-medium text-sm text-white">{{ step.name }}</span>
            <span
              v-if="getStepSubtext(step)"
              class="text-xs truncate"
              :class="{
                'text-green-400': step.status === 'done',
                'text-gray-500': step.status === 'skipped' || step.status === 'waiting',
                'text-blue-400': step.status === 'running',
                'text-red-400': step.status === 'error',
              }"
            >
              {{ getStepSubtext(step) }}
            </span>
          </div>
        </div>

        <!-- Running spinner indicator -->
        <div v-if="step.status === 'running'" class="shrink-0">
          <div class="w-4 h-4 border-2 border-blue-400 border-t-transparent rounded-full animate-spin"></div>
        </div>
      </div>
    </div>

    <!-- Log area -->
    <div class="px-8 pb-6 pt-3 border-t border-gray-800">
      <button
        @click="logExpanded = !logExpanded"
        class="flex items-center gap-2 text-xs text-gray-500 hover:text-gray-300 transition-colors mb-2"
      >
        <span>{{ logExpanded ? '▼' : '▶' }}</span>
        <span>安装日志</span>
      </button>
      <div
        v-if="logExpanded"
        ref="logContainer"
        class="bg-gray-900 rounded-lg border border-gray-800 p-3 h-28 overflow-y-auto font-mono text-xs text-gray-400 space-y-0.5"
      >
        <div v-for="(line, i) in logLines" :key="i" class="leading-5">{{ line }}</div>
        <div v-if="logLines.length === 0" class="text-gray-600">等待日志输出...</div>
      </div>
    </div>
  </div>
</template>
