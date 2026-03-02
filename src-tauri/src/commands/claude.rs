/**
 * ---
 * role: Claude Code 检测与安装，官方脚本优先，npm 安装作回退
 * depends:
 *   - ../types.rs
 * exports:
 *   - ensure_claude
 * status: IMPLEMENTED
 * functions:
 *   - ensure_claude(os: &str) -> StepResult
 *     1. 检测：运行 `claude --version`，已安装则返回 StepResult{status: "skipped"}
 *     2. 尝试官方脚本安装：
 *        Windows: PowerShell -Command "irm https://claude.ai/install.ps1 | iex"
 *        macOS/Linux: bash -c "curl -fsSL https://claude.ai/install.sh | bash"
 *        等待完成，验证 claude --version
 *     3. 若官方脚本失败，回退到 npm：
 *        运行 `npm install -g @anthropic-ai/claude-code`
 *        验证 claude --version
 *     4. 两种方式均失败，返回 StepResult{status: "error", message: "请手动访问 https://claude.ai 安装"}
 *     5. 任一方式成功，返回 StepResult{status: "done", version: Some(ver)}
 * ---
 */

use crate::types::StepResult;
use std::process::Command;

/// 运行 `claude --version` 并返回版本字符串，失败返回 None。
fn get_claude_version() -> Option<String> {
    // Windows: claude is claude.cmd, must run via cmd /c
    // Also check %APPDATA%\npm\claude.cmd directly as fallback
    #[cfg(target_os = "windows")]
    {
        // Try cmd /c claude --version (works even if PATH is set via registry)
        if let Ok(output) = Command::new("cmd").args(["/c", "claude --version"]).output() {
            if output.status.success() {
                let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Some(if ver.is_empty() { "installed".to_string() } else { ver });
            }
        }
        // Fallback: check %APPDATA%\npm\claude.cmd exists
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = std::path::Path::new(&appdata).join("npm").join("claude.cmd");
            if p.exists() {
                return Some("installed".to_string());
            }
        }
        return None;
    }
    #[cfg(not(target_os = "windows"))]
    {
        if which::which("claude").is_err() {
            return None;
        }
        let output = Command::new("claude").arg("--version").output().ok()?;
        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !ver.is_empty() { return Some(ver); }
            let ver_err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if !ver_err.is_empty() { return Some(ver_err); }
            Some("installed".to_string())
        } else {
            None
        }
    }
}

/// 尝试使用官方脚本安装 Claude Code。
/// 成功返回 true，失败返回 false。
fn install_via_official_script(os: &str) -> bool {
    let status = if os == "windows" {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "irm https://claude.ai/install.ps1 | iex",
            ])
            .status()
    } else {
        Command::new("bash")
            .args(["-c", "curl -fsSL https://claude.ai/install.sh | bash"])
            .status()
    };

    matches!(status, Ok(s) if s.success())
}

/// 尝试使用 npm 全局安装 Claude Code。
/// 成功返回 true，失败返回 false。
fn install_via_npm() -> bool {
    let status = Command::new("npm")
        .args(["install", "-g", "@anthropic-ai/claude-code"])
        .status();

    matches!(status, Ok(s) if s.success())
}

pub async fn ensure_claude(os: &str) -> StepResult {
    let os = os.to_string();
    tokio::task::spawn_blocking(move || ensure_claude_sync(&os))
        .await
        .unwrap_or_else(|_| StepResult {
            name: "Claude Code".to_string(),
            status: "error".to_string(),
            message: "任务执行失败".to_string(),
            version: None,
        })
}

fn ensure_claude_sync(os: &str) -> StepResult {
    // --- 检测阶段 ---
    if let Some(ver) = get_claude_version() {
        return StepResult {
            name: "Claude Code".to_string(),
            status: "skipped".to_string(),
            message: "Claude Code 已安装，跳过".to_string(),
            version: Some(ver),
        };
    }

    // --- 策略 1：官方脚本安装 ---
    let official_ok = install_via_official_script(os);

    if official_ok {
        if let Some(ver) = get_claude_version() {
            return StepResult {
                name: "Claude Code".to_string(),
                status: "done".to_string(),
                message: "Claude Code 已安装（官方脚本）".to_string(),
                version: Some(ver),
            };
        }
    }

    // --- 策略 2：npm 回退安装 ---
    let npm_ok = install_via_npm();

    if npm_ok {
        if let Some(ver) = get_claude_version() {
            return StepResult {
                name: "Claude Code".to_string(),
                status: "done".to_string(),
                message: "Claude Code 已安装（npm）".to_string(),
                version: Some(ver),
            };
        }
    }

    // --- 两种方式均失败 ---
    StepResult {
        name: "Claude Code".to_string(),
        status: "error".to_string(),
        message: "安装失败，请手动访问 https://claude.ai 安装".to_string(),
        version: None,
    }
}

pub fn detect() -> crate::types::DetectResult {
    let version = get_claude_version();
    crate::types::DetectResult {
        name: "Claude Code".to_string(),
        installed: version.is_some(),
        version,
    }
}
