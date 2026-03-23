/**
 * ---
 * role: Claude Code 检测与安装，npm 安装作为首选
 * depends:
 *   - ../types.rs
 *   - ./process.rs
 * exports:
 *   - ensure_claude
 *   - detect
 *   - get_claude_version
 * status: VERIFIED
 * functions:
 *   - get_claude_version() -> Option<String>
 *     检测 Claude Code 是否已安装：
 *       1. 官方安装路径检测：%LOCALAPPDATA%\AnthropicClaude\claude.exe（Windows）
 *       2. npm 全局安装检测：%APPDATA%\npm\claude.cmd（Windows）或 ~/.npm-global/bin/claude
 *       3. PATH 检测：运行 `claude --version`
 *     返回版本字符串或 None
 *
 *   - install_via_npm() -> Result<(), String>
 *     使用 npm 全局安装 Claude Code：
 *       npm install -g @anthropic-ai/claude-code
 *     优先尝试此方式，因为 Node.js 已先安装
 *
 *   - install_via_official_script(os: &str) -> Result<(), String>
 *     官方脚本安装（回退方案）：
 *       Windows: PowerShell -Command "Invoke-RestMethod https://claude.ai/install.ps1 | Invoke-Expression"
 *       macOS/Linux: bash -c "curl -fsSL https://claude.ai/install.sh | bash"
 *
 *   - ensure_claude(os: &str) -> StepResult
 *     完整流程：检测 → 安装 → 验证
 *     1. 调用 get_claude_version()，已安装则返回 skipped
 *     2. 未安装则优先使用 npm 安装（install_via_npm）
 *     3. npm 失败则回退到官方脚本（install_via_official_script）
 *     4. 安装后重新验证
 *     5. 返回 done 或 error
 *
 *   - detect() -> DetectResult
 *     供前端检测页面调用，返回 Claude Code 安装状态
 * ---
 */

use std::path::Path;
use std::process::Output;

use crate::types::{DetectResult, StepResult};

use super::process::{command, output_text, summarize_output};

/// 检测 Claude Code 是否已安装
pub fn get_claude_version() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let official_path = Path::new(&local_app_data)
                .join("AnthropicClaude")
                .join("claude.exe");
            if official_path.exists() {
                if let Ok(output) = command(&official_path).arg("--version").output() {
                    if output.status.success() {
                        return Some(output_text(&output.stdout));
                    }
                }
            }
        }

        if let Ok(app_data) = std::env::var("APPDATA") {
            let npm_path = Path::new(&app_data).join("npm").join("claude.cmd");
            if npm_path.exists() {
                if let Ok(output) = command(&npm_path).arg("--version").output() {
                    if output.status.success() {
                        return Some(output_text(&output.stdout));
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").ok()?;
        let npm_global_path = Path::new(&home).join(".npm-global").join("bin").join("claude");
        if npm_global_path.exists() {
            if let Ok(output) = command(&npm_global_path).arg("--version").output() {
                if output.status.success() {
                    return Some(output_text(&output.stdout));
                }
            }
        }
    }

    let cmd = if cfg!(target_os = "windows") { "claude.cmd" } else { "claude" };
    if let Ok(output) = command(cmd).arg("--version").output() {
        if output.status.success() {
            return Some(output_text(&output.stdout));
        }
    }

    None
}

fn run_and_capture(label: &str, mut cmd: std::process::Command) -> Result<Output, String> {
    cmd.output()
        .map_err(|e| format!("Failed to run {}: {}", label, e))
}

/// 使用 npm 全局安装 Claude Code
pub fn install_via_npm() -> Result<(), String> {
    let output = run_and_capture(
        "npm install",
        {
            let mut cmd = command("npm");
            cmd.args(["install", "-g", "@anthropic-ai/claude-code"]);
            cmd
        },
    )?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!("npm install failed: {}", summarize_output(&output)))
    }
}

/// 官方脚本安装
pub fn install_via_official_script(os: &str) -> Result<(), String> {
    if os == "windows" {
        let ps_script = "$ProgressPreference='SilentlyContinue'; Invoke-RestMethod -Uri 'https://claude.ai/install.ps1' | Invoke-Expression";
        let output = run_and_capture(
            "PowerShell install script",
            {
                let mut cmd = command("powershell.exe");
                cmd.args([
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    ps_script,
                ]);
                cmd
            },
        )?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "PowerShell install script failed: {}",
                summarize_output(&output)
            ))
        }
    } else {
        let output = run_and_capture(
            "bash install script",
            {
                let mut cmd = command("bash");
                cmd.args(["-c", "curl -fsSL https://claude.ai/install.sh | bash"]);
                cmd
            },
        )?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!("Bash install script failed: {}", summarize_output(&output)))
        }
    }
}

/// 完整流程：检测 → 安装 → 验证
pub fn ensure_claude(os: &str) -> StepResult {
    let step_name = "Claude Code";

    if let Some(version) = get_claude_version() {
        return StepResult {
            name: step_name.to_string(),
            status: "skipped".to_string(),
            message: format!("Claude Code already installed: {}", version),
            version: Some(version),
        };
    }

    if let Err(npm_error) = install_via_npm() {
        if let Err(script_error) = install_via_official_script(os) {
            return StepResult {
                name: step_name.to_string(),
                status: "error".to_string(),
                message: format!(
                    "Failed to install Claude Code. npm error: {}; script error: {}",
                    npm_error, script_error
                ),
                version: None,
            };
        }
    }

    match get_claude_version() {
        Some(version) => StepResult {
            name: step_name.to_string(),
            status: "done".to_string(),
            message: format!("Claude Code installed successfully: {}", version),
            version: Some(version),
        },
        None => StepResult {
            name: step_name.to_string(),
            status: "error".to_string(),
            message: "Claude Code installation completed but verification failed. Check npm/PowerShell logs above or install manually from https://claude.ai/download".to_string(),
            version: None,
        },
    }
}

/// 供前端检测页面调用
pub fn detect() -> DetectResult {
    match get_claude_version() {
        Some(version) => DetectResult {
            name: "Claude Code".to_string(),
            installed: true,
            version: Some(version),
        },
        None => DetectResult {
            name: "Claude Code".to_string(),
            installed: false,
            version: None,
        },
    }
}
