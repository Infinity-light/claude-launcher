/**
 * ---
 * role: 安装主编排器，按序执行 6 个安装步骤，通过 Tauri 事件流式上报进度
 * depends:
 *   - ../types.rs
 *   - ./node.rs
 *   - ./git.rs
 *   - ./conda.rs
 *   - ./claude.rs
 *   - ./onboarding.rs
 *   - ./ccswitch.rs
 * exports:
 *   - start_installation
 *   - open_path
 * status: IMPLEMENTED
 * functions:
 *   - start_installation(window: tauri::Window, os: String, arch: String) -> Result<InstallationResult, String>
 *     按序执行以下步骤（每步开始前 emit step-update{status: "running"}，结束后 emit step-update{status: done|skipped|error}）：
 *       0: node::ensure_node(&os, &arch) - Node.js 优先安装（后续步骤依赖 Node 下载文件）
 *       1: git::ensure_git(&os, &arch) - Git 使用 Node.js 下载安装程序
 *       2: conda::ensure_conda(&os, &arch) - Miniconda 使用 Node.js 下载+清华镜像
 *       3: claude::ensure_claude(&os) - Claude Code 最后安装（依赖 npm）
 *       4: onboarding::configure_onboarding()
 *       5: ccswitch::download_ccswitch(&os, &arch)
 *     步骤失败不中断，继续后续步骤
 *     全部完成后 emit installation-complete，并返回 InstallationResult
 *
 *   - open_path(path: String) -> Result<(), String>
 *     用系统默认方式打开文件/目录（用于完成页打开 CCSwitch 下载位置）
 *     Windows: explorer.exe <path>
 *     macOS: open <path>
 *     Linux: xdg-open <path>
 * ---
 */

use crate::types::{InstallationResult, StepResult, StepUpdate};
use tauri::{Window, Emitter};
use serde_json::json;

// 各步骤模块
use super::node;
use super::git;
use super::conda;
use super::claude;
use super::onboarding;
use super::ccswitch;

/// 步骤名称列表
const STEP_NAMES: [&str; 6] = [
    "Node.js",
    "Git",
    "Miniconda",
    "Claude Code",
    "Onboarding 配置",
    "CCSwitch",
];

/// 按序执行 6 个安装步骤
#[tauri::command]
pub async fn start_installation(window: Window, os: String, arch: String) -> Result<InstallationResult, String> {
    let mut results = vec![];

    // Step 0: Node.js
    emit_step_update(&window, 0, "running", None);
    let result = node::ensure_node(&os, &arch);
    emit_step_result(&window, 0, &result);
    results.push(result);

    // Step 1: Git
    emit_step_update(&window, 1, "running", None);
    let result = git::ensure_git(&os, &arch);
    emit_step_result(&window, 1, &result);
    results.push(result);

    // Step 2: Miniconda
    emit_step_update(&window, 2, "running", None);
    let result = conda::ensure_conda(&os, &arch);
    emit_step_result(&window, 2, &result);
    results.push(result);

    // Step 3: Claude Code
    emit_step_update(&window, 3, "running", None);
    let result = claude::ensure_claude(&os);
    emit_step_result(&window, 3, &result);
    results.push(result);

    // Step 4: Onboarding 配置
    emit_step_update(&window, 4, "running", None);
    let result = onboarding::configure_onboarding().await;
    emit_step_result(&window, 4, &result);
    results.push(result);

    // Step 5: CCSwitch
    emit_step_update(&window, 5, "running", None);
    let result = ccswitch::download_ccswitch(&os, &arch).await;
    emit_step_result(&window, 5, &result);
    results.push(result);

    // 计算统计
    let success_count = results.iter().filter(|r| r.status == "done").count();
    let skip_count = results.iter().filter(|r| r.status == "skipped").count();
    let error_count = results.iter().filter(|r| r.status == "error").count();

    let installation_result = InstallationResult {
        steps: results,
        success_count,
        skip_count,
        error_count,
    };

    // 发射完成事件
    let _ = window.emit("installation-complete", json!({
        "steps": &installation_result.steps,
        "success_count": installation_result.success_count,
        "skip_count": installation_result.skip_count,
        "error_count": installation_result.error_count,
    }));

    Ok(installation_result)
}

/// 发射步骤更新事件
fn emit_step_update(window: &Window, step: usize, status: &str, message: Option<&str>) {
    let update = StepUpdate {
        index: step,
        name: STEP_NAMES[step].to_string(),
        status: status.to_string(),
        message: message.unwrap_or("").to_string(),
        version: None,
    };
    let _ = window.emit("step-update", &update);
}

/// 根据 StepResult 发射步骤结果事件
fn emit_step_result(window: &Window, step: usize, result: &StepResult) {
    let update = StepUpdate {
        index: step,
        name: STEP_NAMES[step].to_string(),
        status: result.status.clone(),
        message: result.message.clone(),
        version: result.version.clone(),
    };
    let _ = window.emit("step-update", &update);
}

/// 检测所有组件状态
#[tauri::command]
pub async fn detect_all(os: String, arch: String) -> Result<Vec<crate::types::DetectResult>, String> {
    Ok(vec![
        node::detect(),
        git::detect(),
        conda::detect(),
        claude::detect(),
        onboarding::detect(),
        ccswitch::detect(&os, &arch),
    ])
}

/// 用系统默认方式打开文件/目录
#[tauri::command]
pub async fn open_path(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open path: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open path: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open path: {}", e))?;
    }

    Ok(())
}
