/**
 * ---
 * role: Git 检测与安装，支持 Windows / macOS / Linux
 * depends:
 *   - ../types.rs
 * exports:
 *   - ensure_git
 * status: IMPLEMENTED
 * functions:
 *   - ensure_git(os: &str) -> StepResult
 *     1. 检测：运行 `git --version`，成功则返回 StepResult{status: "skipped", version: Some(ver)}
 *     2. 安装（如未安装）：
 *        Windows: winget install --id Git.Git -e --source winget --silent
 *        macOS:   xcode-select --install（会弹出系统对话框，等待完成）
 *        Linux:   按发行版检测包管理器（apt/yum/dnf/pacman），执行对应 install 命令
 *     3. 安装后重新验证 git --version
 *     4. 任一步骤失败返回 StepResult{status: "error", message: 具体错误}
 * ---
 */

use crate::types::StepResult;
use std::process::Command;

/// 运行 `git --version` 并返回版本字符串，失败则返回 None。
fn detect_git_version() -> Option<String> {
    if which::which("git").is_err() {
        return None;
    }
    let output = Command::new("git").arg("--version").output().ok()?;
    if output.status.success() {
        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(raw)
    } else {
        None
    }
}

/// 在 Linux 上尝试用 apt-get 安装 git，失败则回退到 yum/dnf。
fn install_git_linux() -> Result<(), String> {
    let apt_status = Command::new("sh")
        .args(["-c", "apt-get install -y git"])
        .status();

    match apt_status {
        Ok(s) if s.success() => return Ok(()),
        _ => {}
    }

    // 回退到 yum / dnf
    let yum_status = Command::new("sh")
        .args(["-c", "yum install -y git || dnf install -y git"])
        .status();

    match yum_status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!(
            "yum/dnf install git 失败，退出码: {}",
            s.code().unwrap_or(-1)
        )),
        Err(e) => Err(format!("执行 yum/dnf 命令失败: {}", e)),
    }
}

pub async fn ensure_git(os: &str) -> StepResult {
    let os = os.to_string();
    tokio::task::spawn_blocking(move || ensure_git_sync(&os))
        .await
        .unwrap_or_else(|_| StepResult {
            name: "Git".to_string(),
            status: "error".to_string(),
            message: "任务执行失败".to_string(),
            version: None,
        })
}

fn ensure_git_sync(os: &str) -> StepResult {
    // 1. 检测是否已安装
    if let Some(version) = detect_git_version() {
        return StepResult {
            name: "git".to_string(),
            status: "skipped".to_string(),
            message: format!("Git 已安装：{}", version),
            version: Some(version),
        };
    }

    // 2. 按平台执行安装
    let install_result: Result<(), String> = match os {
        "windows" => {
            let status = Command::new("winget")
                .args([
                    "install",
                    "--id",
                    "Git.Git",
                    "-e",
                    "--source",
                    "winget",
                    "--silent",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ])
                .status();
            match status {
                Ok(s) if s.success() => Ok(()),
                Ok(s) => Err(format!(
                    "winget install Git.Git 失败，退出码: {}",
                    s.code().unwrap_or(-1)
                )),
                Err(e) => Err(format!("执行 winget 命令失败: {}", e)),
            }
        }
        "macos" => {
            // xcode-select --install 会弹出系统对话框；这里同时触发安装并等待 git 可用
            let status = Command::new("sh")
                .args(["-c", "xcode-select --install 2>/dev/null; git --version"])
                .status();
            match status {
                Ok(s) if s.success() => Ok(()),
                Ok(s) => Err(format!(
                    "macOS git 安装失败，退出码: {}",
                    s.code().unwrap_or(-1)
                )),
                Err(e) => Err(format!("执行 xcode-select 命令失败: {}", e)),
            }
        }
        "linux" => install_git_linux(),
        other => Err(format!("不支持的操作系统: {}", other)),
    };

    if let Err(err_msg) = install_result {
        return StepResult {
            name: "git".to_string(),
            status: "error".to_string(),
            message: err_msg,
            version: None,
        };
    }

    // 3. 安装后重新验证
    match detect_git_version() {
        Some(version) => StepResult {
            name: "git".to_string(),
            status: "done".to_string(),
            message: format!("Git 安装成功：{}", version),
            version: Some(version),
        },
        None => StepResult {
            name: "git".to_string(),
            status: "error".to_string(),
            message: "Git 安装命令执行成功，但安装后仍无法检测到 git --version，请重启终端后重试".to_string(),
            version: None,
        },
    }
}

pub fn detect() -> crate::types::DetectResult {
    let version = detect_git_version();
    crate::types::DetectResult {
        name: "Git".to_string(),
        installed: version.is_some(),
        version,
    }
}
