/**
 * ---
 * role: Node.js LTS 检测与安装，支持 Windows / macOS / Linux
 * depends:
 *   - ../types.rs
 *   - ./process.rs
 * exports:
 *   - ensure_node
 *   - detect
 * status: IMPLEMENTED
 * functions:
 *   - detect_node_version() -> Option<String>
 *     检测 Node.js 是否已安装：
 *       1. 固定路径检测：C:\Program Files\nodejs\node.exe（Windows）
 *       2. PATH 检测：运行 `node --version`
 *     返回版本字符串或 None
 *
 *   - ensure_node(os: &str, arch: &str) -> StepResult
 *     1. 检测：调用 detect_node_version()，已安装则返回 StepResult{status: "skipped"}
 *     2. 安装（如未安装）：
 *        Windows: winget install --id OpenJS.NodeJS.LTS -e --source winget --silent
 *        macOS:   下载 .pkg 安装包 installer -pkg <file> -target /（或 brew install node@lts）
 *        Linux:   curl NodeSource 脚本 | bash，然后 apt-get install nodejs
 *     3. 安装后：重新检测 node --version（即使 PATH 未刷新也尝试固定路径）
 *     4. 失败返回 StepResult{status: "error"}
 *
 *   - get_node_lts_pkg_url(arch: &str) -> String
 *     根据 arch 拼接 nodejs.org 官方 LTS 下载 URL（x64/arm64）
 *
 *   - detect() -> DetectResult
 *     供前端检测页面调用，返回 Node.js 安装状态
 * ---
 */

use std::path::Path;
#[cfg(target_os = "linux")]
use std::process::Stdio;

use crate::types::{DetectResult, StepResult};

use super::process::{command, output_text, summarize_output};
#[cfg(target_os = "linux")]
use super::process::error_text;

const MIN_NODE_MAJOR: u32 = 18;

fn parse_node_major(version: &str) -> Option<u32> {
    version
        .trim()
        .trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|v| v.parse::<u32>().ok())
}

fn is_supported_node(version: &str) -> bool {
    parse_node_major(version)
        .map(|major| major >= MIN_NODE_MAJOR)
        .unwrap_or(false)
}

/// 检测 Node.js 是否已安装
pub fn detect_node_version() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let fixed_path = r"C:\Program Files\nodejs\node.exe";
        if Path::new(fixed_path).exists() {
            if let Ok(output) = command(fixed_path).arg("--version").output() {
                if output.status.success() {
                    return Some(output_text(&output.stdout));
                }
            }
        }
    }

    if let Ok(output) = command("node").arg("--version").output() {
        if output.status.success() {
            return Some(output_text(&output.stdout));
        }
    }

    None
}

/// 确保 Node.js 已安装
pub fn ensure_node(_os: &str, _arch: &str) -> StepResult {
    let step_name = "Node.js";

    if let Some(version) = detect_node_version() {
        if is_supported_node(&version) {
            return StepResult {
                name: step_name.to_string(),
                status: "skipped".to_string(),
                message: format!("Node.js already installed: {}", version),
                version: Some(version),
            };
        }
    }

    #[cfg(target_os = "windows")]
    {
        let check_installed = command("winget")
            .args(["list", "--id", "OpenJS.NodeJS.LTS", "-e"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let mut args = vec![
            if check_installed { "upgrade" } else { "install" },
            "--id",
            "OpenJS.NodeJS.LTS",
            "-e",
            "--source",
            "winget",
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ];

        if check_installed {
            args.push("--include-unknown");
        }

        let output = command("winget")
            .args(args.as_slice())
            .output();

        match output {
            Ok(out) if out.status.success() => {}
            Ok(out) => {
                return StepResult {
                    name: step_name.to_string(),
                    status: "error".to_string(),
                    message: format!(
                        "Failed to install or upgrade Node.js via winget: {}",
                        summarize_output(&out)
                    ),
                    version: None,
                };
            }
            Err(e) => {
                return StepResult {
                    name: step_name.to_string(),
                    status: "error".to_string(),
                    message: format!("Failed to run winget: {}", e),
                    version: None,
                };
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let output = command("brew").args(["install", "node@lts"]).output();

        match output {
            Ok(out) if out.status.success() => {}
            Ok(out) => {
                return StepResult {
                    name: step_name.to_string(),
                    status: "error".to_string(),
                    message: format!("Failed to install Node.js via brew: {}", summarize_output(&out)),
                    version: None,
                };
            }
            Err(e) => {
                return StepResult {
                    name: step_name.to_string(),
                    status: "error".to_string(),
                    message: format!("Failed to run brew: {}", e),
                    version: None,
                };
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let curl_output = command("curl")
            .args(["-fsSL", "https://deb.nodesource.com/setup_lts.x"])
            .output();

        match curl_output {
            Ok(output) if output.status.success() => {
                let bash_result = command("bash")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .and_then(|mut child| {
                        use std::io::Write;
                        if let Some(mut stdin) = child.stdin.take() {
                            stdin.write_all(&output.stdout)?;
                        }
                        child.wait_with_output()
                    });

                match bash_result {
                    Ok(out) if out.status.success() => {}
                    Ok(out) => {
                        return StepResult {
                            name: step_name.to_string(),
                            status: "error".to_string(),
                            message: format!("Failed to setup NodeSource: {}", summarize_output(&out)),
                            version: None,
                        };
                    }
                    Err(e) => {
                        return StepResult {
                            name: step_name.to_string(),
                            status: "error".to_string(),
                            message: format!("Failed to execute NodeSource setup: {}", e),
                            version: None,
                        };
                    }
                }

                let apt_output = command("apt-get").args(["install", "-y", "nodejs"]).output();

                match apt_output {
                    Ok(out) if out.status.success() => {}
                    Ok(out) => {
                        return StepResult {
                            name: step_name.to_string(),
                            status: "error".to_string(),
                            message: format!("Failed to install Node.js via apt: {}", summarize_output(&out)),
                            version: None,
                        };
                    }
                    Err(e) => {
                        return StepResult {
                            name: step_name.to_string(),
                            status: "error".to_string(),
                            message: format!("Failed to run apt-get: {}", e),
                            version: None,
                        };
                    }
                }
            }
            Ok(output) => {
                return StepResult {
                    name: step_name.to_string(),
                    status: "error".to_string(),
                    message: format!("Failed to download NodeSource setup script: {}", error_text(&output)),
                    version: None,
                };
            }
            Err(e) => {
                return StepResult {
                    name: step_name.to_string(),
                    status: "error".to_string(),
                    message: format!("Failed to run curl for NodeSource setup: {}", e),
                    version: None,
                };
            }
        }
    }

    match detect_node_version() {
        Some(version) if is_supported_node(&version) => StepResult {
            name: step_name.to_string(),
            status: "done".to_string(),
            message: format!("Node.js ready: {}", version),
            version: Some(version),
        },
        Some(version) => StepResult {
            name: step_name.to_string(),
            status: "error".to_string(),
            message: format!(
                "Node.js version too old after installation: {} (requires >= v{})",
                version, MIN_NODE_MAJOR
            ),
            version: Some(version),
        },
        None => StepResult {
            name: step_name.to_string(),
            status: "error".to_string(),
            message: "Node.js installation completed but verification failed".to_string(),
            version: None,
        },
    }
}

/// 供前端检测页面调用
pub fn detect() -> DetectResult {
    match detect_node_version() {
        Some(version) => {
            if is_supported_node(&version) {
                DetectResult {
                    name: "Node.js".to_string(),
                    installed: true,
                    version: Some(version),
                }
            } else {
                DetectResult {
                    name: "Node.js".to_string(),
                    installed: false,
                    version: Some(format!("{} (requires >= v{})", version, MIN_NODE_MAJOR)),
                }
            }
        }
        None => DetectResult {
            name: "Node.js".to_string(),
            installed: false,
            version: None,
        },
    }
}
