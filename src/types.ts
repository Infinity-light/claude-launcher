/**
 * ---
 * role: 前端全局共享类型定义，与 Rust 后端 types.rs 保持一致
 * depends: []
 * exports:
 *   - SystemInfo
 *   - StepUpdate
 *   - StepResult
 *   - InstallationResult
 *   - StepStatus
 *   - STEP_DEFINITIONS
 * status: IMPLEMENTED
 * functions:
 *   - (types and constants only)
 *
 * notes:
 *   STEP_DEFINITIONS 定义 6 个安装步骤的名称和描述，顺序固定：
 *   0: Node.js
 *   1: Git
 *   2: Miniconda
 *   3: Claude Code
 *   4: Onboarding 配置
 *   5: CCSwitch
 * ---
 */

export interface SystemInfo {
  os: string
  arch: string
}

export interface StepUpdate {
  index: number
  name: string
  status: 'running' | 'done' | 'skipped' | 'error'
  message: string
  version?: string
}

export interface StepResult {
  name: string
  status: 'done' | 'skipped' | 'error'
  message: string
  version?: string
}

export interface InstallationResult {
  steps: StepResult[]
  success_count: number
  skip_count: number
  error_count: number
}

export type StepStatus = 'waiting' | 'running' | 'done' | 'skipped' | 'error'

export interface DetectResult {
  name: string
  installed: boolean
  version?: string
}

export interface StepDef {
  name: string
  desc: string
}

export const STEP_DEFINITIONS: StepDef[] = [
  { name: 'Node.js', desc: 'JavaScript 运行时（LTS）' },
  { name: 'Git', desc: '版本控制工具' },
  { name: 'Miniconda', desc: 'Python 环境管理' },
  { name: 'Claude Code', desc: '官方 CLI，npm 优先，脚本兜底' },
  { name: 'Onboarding 配置', desc: '跳过登录验证' },
  { name: 'CCSwitch', desc: 'API 源切换工具' },
]
