/**
 * ---
 * role: Pinia 安装状态管理，跟踪 7 步安装流程状态，响应 Tauri 事件
 * depends:
 *   - pinia
 *   - @tauri-apps/api/core
 *   - @tauri-apps/api/event
 *   - ../types.ts
 * exports:
 *   - useInstallerStore
 * status: IMPLEMENTED
 * functions:
 *   - startInstallation(): Promise<void>
 *     invoke("start_installation", {os, arch})
 *     在调用前先注册两个事件监听：
 *       listen("step-update", handler) → 更新 steps[index] 状态
 *       listen("installation-complete", handler) → 设置 isComplete = true，保存 result
 *
 *   - getStepStatus(index: number): StepStatus
 *     返回 steps[index].status
 *
 * state:
 *   systemInfo: SystemInfo | null    # 由 WelcomeView 写入
 *   steps: StepState[]               # 7 个步骤的状态，初始全为 "waiting"
 *   isInstalling: boolean
 *   isComplete: boolean
 *   result: InstallationResult | null
 *   ccSwitchDownloadPath: string | null  # 完成后存储下载路径，供 CompleteView 使用
 * ---
 */

import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { SystemInfo, InstallationResult, StepStatus } from '../types'
import { STEP_DEFINITIONS } from '../types'

interface StepState {
  name: string
  status: StepStatus
  message: string
  version?: string
}

export const useInstallerStore = defineStore('installer', {
  state: () => ({
    systemInfo: null as SystemInfo | null,
    steps: STEP_DEFINITIONS.map((def) => ({
      name: def.name,
      status: 'waiting' as StepStatus,
      message: '',
      version: undefined as string | undefined,
    })) as StepState[],
    isInstalling: false,
    isComplete: false,
    result: null as InstallationResult | null,
    ccSwitchDownloadPath: null as string | null,
  }),

  actions: {
    async startInstallation(): Promise<void> {
      if (this.isInstalling) return
      this.isInstalling = true
      const os = this.systemInfo?.os ?? 'unknown'
      const arch = this.systemInfo?.arch ?? 'unknown'
      await invoke('start_installation', { os, arch })
    },

    getStepStatus(index: number): StepStatus {
      return this.steps[index]?.status ?? 'waiting'
    },
  },
})
