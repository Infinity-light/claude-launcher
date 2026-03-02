/**
 * ---
 * role: 安装主编排器，按序执行 7 个安装步骤，通过 Tauri 事件流式上报进度
 * depends:
 *   - ../types.rs
 *   - ./git.rs
 *   - ./node.rs
 *   - ./conda.rs
 *   - ./claude.rs
 *   - ./onboarding.rs
 *   - ./ccswitch.rs
 *   - ./workflow.rs
 * exports:
 *   - start_installation
 *   - open_path
 * status: IMPLEMENTED
 * functions:
 *   - start_installation(window: tauri::Window, os: String, arch: String) -> Result<InstallationResult, String>
 *     按序执行以下步骤（每步开始前 emit step-update{status: "running"}，结束后 emit step-update{status: done|skipped|error}）：
 *       0: git::ensure_git(&os)
 *       1: node::ensure_node(&os, &arch)
 *       2: conda::ensure_conda(&os, &arch)
 *       3: claude::ensure_claude(&os)
 *       4: onboarding::configure_onboarding()
 *       5: ccswitch::download_ccswitch(&os, &arch)
 *       6: workflow::install_workflow_kit(&os)
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

use tauri::{Window, Emitter};
use crate::types::{InstallationResult, StepResult, StepUpdate};
use crate::commands::{git, node, conda, claude, onboarding, ccswitch, workflow};

const STEP_NAMES: [&str; 7] = [
    "Git",
    "Node.js",
    "Miniconda",
    "Claude Code",
    "Onboarding 配置",
    "CCSwitch",
    "Workflow Kit",
];

#[tauri::command]
pub async fn start_installation(
    window: Window,
    os: String,
    arch: String,
) -> Result<InstallationResult, String> {
    let mut steps: Vec<StepResult> = Vec::new();

    // Step 0: Git
    window.emit("step-update", StepUpdate {
        index: 0,
        name: STEP_NAMES[0].to_string(),
        status: "running".to_string(),
        message: "正在安装 Git...".to_string(),
        version: None,
    }).ok();
    let result = git::ensure_git(&os).await;
    window.emit("step-update", StepUpdate {
        index: 0,
        name: result.name.clone(),
        status: result.status.clone(),
        message: result.message.clone(),
        version: result.version.clone(),
    }).ok();
    steps.push(result);

    // Step 1: Node.js
    window.emit("step-update", StepUpdate {
        index: 1,
        name: STEP_NAMES[1].to_string(),
        status: "running".to_string(),
        message: "正在安装 Node.js...".to_string(),
        version: None,
    }).ok();
    let result = node::ensure_node(&os, &arch).await;
    window.emit("step-update", StepUpdate {
        index: 1,
        name: result.name.clone(),
        status: result.status.clone(),
        message: result.message.clone(),
        version: result.version.clone(),
    }).ok();
    steps.push(result);

    // Step 2: Miniconda
    window.emit("step-update", StepUpdate {
        index: 2,
        name: STEP_NAMES[2].to_string(),
        status: "running".to_string(),
        message: "正在安装 Miniconda...".to_string(),
        version: None,
    }).ok();
    let result = conda::ensure_conda(&os, &arch).await;
    window.emit("step-update", StepUpdate {
        index: 2,
        name: result.name.clone(),
        status: result.status.clone(),
        message: result.message.clone(),
        version: result.version.clone(),
    }).ok();
    steps.push(result);

    // Step 3: Claude Code
    window.emit("step-update", StepUpdate {
        index: 3,
        name: STEP_NAMES[3].to_string(),
        status: "running".to_string(),
        message: "正在安装 Claude Code...".to_string(),
        version: None,
    }).ok();
    let result = claude::ensure_claude(&os).await;
    window.emit("step-update", StepUpdate {
        index: 3,
        name: result.name.clone(),
        status: result.status.clone(),
        message: result.message.clone(),
        version: result.version.clone(),
    }).ok();
    steps.push(result);

    // Step 4: Onboarding 配置
    window.emit("step-update", StepUpdate {
        index: 4,
        name: STEP_NAMES[4].to_string(),
        status: "running".to_string(),
        message: "正在配置 Onboarding...".to_string(),
        version: None,
    }).ok();
    let result = onboarding::configure_onboarding().await;
    window.emit("step-update", StepUpdate {
        index: 4,
        name: result.name.clone(),
        status: result.status.clone(),
        message: result.message.clone(),
        version: result.version.clone(),
    }).ok();
    steps.push(result);

    // Step 5: CCSwitch
    window.emit("step-update", StepUpdate {
        index: 5,
        name: STEP_NAMES[5].to_string(),
        status: "running".to_string(),
        message: "正在下载 CCSwitch...".to_string(),
        version: None,
    }).ok();
    let result = ccswitch::download_ccswitch(&os, &arch).await;
    window.emit("step-update", StepUpdate {
        index: 5,
        name: result.name.clone(),
        status: result.status.clone(),
        message: result.message.clone(),
        version: result.version.clone(),
    }).ok();
    steps.push(result);

    // Step 6: Workflow Kit
    window.emit("step-update", StepUpdate {
        index: 6,
        name: STEP_NAMES[6].to_string(),
        status: "running".to_string(),
        message: "正在安装 Workflow Kit...".to_string(),
        version: None,
    }).ok();
    let result = workflow::install_workflow_kit(&os).await;
    window.emit("step-update", StepUpdate {
        index: 6,
        name: result.name.clone(),
        status: result.status.clone(),
        message: result.message.clone(),
        version: result.version.clone(),
    }).ok();
    steps.push(result);

    // Tally results
    let success_count = steps.iter().filter(|s| s.status == "done").count();
    let skip_count = steps.iter().filter(|s| s.status == "skipped").count();
    let error_count = steps.iter().filter(|s| s.status == "error").count();

    let installation_result = InstallationResult {
        steps,
        success_count,
        skip_count,
        error_count,
    };

    window.emit("installation-complete", &installation_result).ok();

    Ok(installation_result)
}

#[tauri::command]
pub async fn open_path(path: String) -> Result<(), String> {
    let status = if cfg!(target_os = "windows") {
        std::process::Command::new("explorer").arg(&path).status()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(&path).status()
    } else {
        std::process::Command::new("xdg-open").arg(&path).status()
    };
    match status {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("打开路径失败: {}", e)),
    }
}

#[tauri::command]
pub async fn detect_all(os: String, arch: String) -> Vec<crate::types::DetectResult> {
    tokio::task::spawn_blocking(move || {
        vec![
            git::detect(),
            node::detect(),
            conda::detect(&os),
            claude::detect(),
            onboarding::detect(),
            ccswitch::detect(&os, &arch),
            workflow::detect(),
        ]
    })
    .await
    .unwrap_or_default()
}
