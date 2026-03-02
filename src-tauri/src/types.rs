/**
 * ---
 * role: 全局共享类型定义，所有命令模块共用
 * depends: []
 * exports:
 *   - SystemInfo
 *   - StepUpdate
 *   - StepResult
 *   - InstallationResult
 *   - StepStatus
 * status: IMPLEMENTED
 * functions:
 *   - (data types only, no functions)
 *
 * types:
 *   SystemInfo:
 *     os: String        # "windows" | "macos" | "linux"
 *     arch: String      # "x64" | "arm64"
 *
 *   StepUpdate:
 *     index: usize      # 0-6 对应 7 个安装步骤
 *     name: String      # 步骤显示名称
 *     status: String    # "running" | "done" | "skipped" | "error"
 *     message: String   # 当前状态描述
 *     version: Option<String>  # 成功时的版本号
 *
 *   StepResult:
 *     name: String
 *     status: String    # "done" | "skipped" | "error"
 *     message: String
 *     version: Option<String>
 *
 *   InstallationResult:
 *     steps: Vec<StepResult>
 *     success_count: usize
 *     skip_count: usize
 *     error_count: usize
 * ---
 */

use serde::{Deserialize, Serialize};

/// 当前运行环境的操作系统与架构信息。
///
/// `os`   取值范围：`"windows"` | `"macos"` | `"linux"`
/// `arch` 取值范围：`"x64"` | `"arm64"`
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
}

/// 单个安装步骤的实时进度更新，由后端通过 Tauri 事件推送给前端。
///
/// `index`   步骤序号，0-6 对应 7 个安装步骤
/// `name`    步骤显示名称
/// `status`  取值范围：`"running"` | `"done"` | `"skipped"` | `"error"`
/// `message` 当前状态描述文字
/// `version` 成功安装时填入的版本号，其余情况为 `None`
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StepUpdate {
    pub index: usize,
    pub name: String,
    pub status: String,
    pub message: String,
    pub version: Option<String>,
}

/// 单个安装步骤的最终结果，汇总于 `InstallationResult`。
///
/// `status` 取值范围：`"done"` | `"skipped"` | `"error"`
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StepResult {
    pub name: String,
    pub status: String,
    pub message: String,
    pub version: Option<String>,
}

/// 整次安装流程的汇总结果，安装命令执行完毕后返回给前端。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallationResult {
    pub steps: Vec<StepResult>,
    pub success_count: usize,
    pub skip_count: usize,
    pub error_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectResult {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
}
