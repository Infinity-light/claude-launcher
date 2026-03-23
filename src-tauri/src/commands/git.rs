/**
 * ---
 * role: Git 检测与安装，Windows 下使用 Node.js 下载安装程序
 * depends:
 *   - ../types.rs
 *   - ./download.rs
 *   - ./process.rs
 * exports:
 *   - ensure_git
 *   - detect
 *   - detect_git
 *   - download_git_installer
 *   - install_git_silent
 * status: IMPLEMENTED
 * functions:
 *   - detect_git() -> Option<String>
 *     检测 Git 是否已安装：
 *       1. 固定路径检测：C:\Program Files\Git\cmd\git.exe（Windows）
 *       2. PATH 检测：运行 `git --version`
 *     返回版本字符串或 None
 *
 *   - download_git_installer(os: &str, arch: &str, dest: &Path) -> Result<(), String>
 *     使用 Node.js 的 https 模块下载 Git 安装程序
 *     下载源优先级：
 *       1. 清华镜像：https://mirrors.tuna.tsinghua.edu.cn/github-release/git-for-windows/git/
 *       2. 回退到 GitHub 官方：https://github.com/git-for-windows/git/releases/download/
 *     支持显示下载进度（通过回调或事件）
 *
 *   - install_git_silent(installer_path: &Path) -> Result<(), String>
 *     Windows 静默安装 Git：
 *       installer.exe /VERYSILENT /NORESTART /SUPPRESSMSGBOXES
 *     macOS: xcode-select --install
 *     Linux: 按发行版检测包管理器（apt/yum/dnf/pacman）执行安装
 *
 *   - ensure_git(os: &str, arch: &str) -> StepResult
 *     完整流程：检测 → 下载 → 安装 → 验证
 *     1. 调用 detect_git()，已安装则返回 skipped
 *     2. 未安装则调用 download_git_installer 下载安装程序
 *     3. 调用 install_git_silent 执行静默安装
 *     4. 安装后重新验证
 *     5. 返回 done 或 error
 *
 *   - detect() -> DetectResult
 *     供前端检测页面调用，返回 Git 安装状态
 * ---
 */

use std::path::Path;

use crate::types::{DetectResult, StepResult};

use super::download::download_with_fallback;
use super::process::{command, output_text, summarize_output};
#[cfg(target_os = "macos")]
use super::process::error_text;

/// 检测 Git 是否已安装
/// 1. 固定路径检测：C:\Program Files\Git\cmd\git.exe（Windows）
/// 2. PATH 检测：运行 `git --version`
/// 返回版本字符串或 None
pub fn detect_git() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let fixed_path = r"C:\Program Files\Git\cmd\git.exe";
        if std::path::Path::new(fixed_path).exists() {
            if let Ok(output) = command(fixed_path).arg("--version").output() {
                if output.status.success() {
                    return Some(output_text(&output.stdout));
                }
            }
        }
    }

    if let Ok(output) = command("git").arg("--version").output() {
        if output.status.success() {
            return Some(output_text(&output.stdout));
        }
    }

    None
}

/// 获取 Git for Windows 下载 URL
fn get_git_windows_url(arch: &str) -> Vec<String> {
    let arch_suffix = match arch {
        "arm64" => "arm64",
        _ => "64-bit",
    };

    let version = "2.43.0";
    let filename = format!("Git-{}-{}.exe", version, arch_suffix);

    vec![
        format!(
            "https://npmmirror.com/mirrors/git-for-windows/v{}.windows.1/{}",
            version, filename
        ),
        format!(
            "https://github.com/git-for-windows/git/releases/download/v{}.windows.1/{}",
            version, filename
        ),
    ]
}

/// 使用 Node.js 下载 Git 安装程序
pub fn download_git_installer(os: &str, arch: &str, dest: &Path) -> Result<(), String> {
    if os != "windows" {
        return Err("Git download only supported on Windows. Use system package manager on macOS/Linux.".to_string());
    }

    let urls = get_git_windows_url(arch);
    let url_refs: Vec<&str> = urls.iter().map(|s| s.as_str()).collect();
    download_with_fallback(&url_refs, dest)
}

/// Windows 静默安装 Git
pub fn install_git_silent(installer_path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let output = command(installer_path)
            .args(["/VERYSILENT", "/NORESTART", "/SUPPRESSMSGBOXES"])
            .output()
            .map_err(|e| format!("Failed to run Git installer: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!("Git installer failed: {}", summarize_output(&output)))
        }
    }

    #[cfg(target_os = "macos")]
    {
        let xcode_output = command("xcode-select")
            .arg("--install")
            .output();

        match xcode_output {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => {
                let brew_output = command("brew")
                    .args(["install", "git"])
                    .output()
                    .map_err(|e| format!("Failed to install git via brew: {}", e))?;

                if brew_output.status.success() {
                    Ok(())
                } else {
                    Err(format!(
                        "xcode-select failed: {}; brew failed: {}",
                        error_text(&output),
                        summarize_output(&brew_output)
                    ))
                }
            }
            Err(_) => {
                let brew_output = command("brew")
                    .args(["install", "git"])
                    .output()
                    .map_err(|e| format!("Failed to install git via brew: {}", e))?;

                if brew_output.status.success() {
                    Ok(())
                } else {
                    Err(format!("brew failed: {}", summarize_output(&brew_output)))
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let package_managers = vec![
            ("apt-get", vec!["update"], vec!["install", "-y", "git"]),
            ("yum", vec![], vec!["install", "-y", "git"]),
            ("dnf", vec![], vec!["install", "-y", "git"]),
            ("pacman", vec!["-Sy"], vec!["-S", "--noconfirm", "git"]),
        ];

        for (pm, update_args, install_args) in package_managers {
            let exists = command("which")
                .arg(pm)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if !exists {
                continue;
            }

            if !update_args.is_empty() {
                let update_output = command(pm)
                    .args(update_args.as_slice())
                    .output()
                    .map_err(|e| format!("Failed to update package lists via {}: {}", pm, e))?;

                if !update_output.status.success() {
                    return Err(format!(
                        "{} update failed: {}",
                        pm,
                        summarize_output(&update_output)
                    ));
                }
            }

            let install_output = command(pm)
                .args(install_args.as_slice())
                .output()
                .map_err(|e| format!("Failed to install git via {}: {}", pm, e))?;

            if install_output.status.success() {
                return Ok(());
            }

            return Err(format!("{} install failed: {}", pm, summarize_output(&install_output)));
        }

        Err("No supported package manager found on Linux".to_string())
    }
}

/// 完整流程：检测 → 下载 → 安装 → 验证
pub fn ensure_git(os: &str, arch: &str) -> StepResult {
    let step_name = "Git";

    if let Some(version) = detect_git() {
        return StepResult {
            name: step_name.to_string(),
            status: "skipped".to_string(),
            message: format!("Git already installed: {}", version),
            version: Some(version),
        };
    }

    #[cfg(target_os = "windows")]
    {
        let temp_dir = std::env::temp_dir();
        let installer_path = temp_dir.join("git-installer.exe");

        if let Err(e) = download_git_installer(os, arch, &installer_path) {
            return StepResult {
                name: step_name.to_string(),
                status: "error".to_string(),
                message: format!("Failed to download Git installer: {}", e),
                version: None,
            };
        }

        if let Err(e) = install_git_silent(&installer_path) {
            return StepResult {
                name: step_name.to_string(),
                status: "error".to_string(),
                message: format!("Failed to install Git: {}", e),
                version: None,
            };
        }

        let _ = std::fs::remove_file(&installer_path);
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Err(e) = install_git_silent(Path::new("")) {
            return StepResult {
                name: step_name.to_string(),
                status: "error".to_string(),
                message: format!("Failed to install Git: {}", e),
                version: None,
            };
        }
    }

    match detect_git() {
        Some(version) => StepResult {
            name: step_name.to_string(),
            status: "done".to_string(),
            message: format!("Git installed successfully: {}", version),
            version: Some(version),
        },
        None => StepResult {
            name: step_name.to_string(),
            status: "error".to_string(),
            message: "Git installation completed but verification failed".to_string(),
            version: None,
        },
    }
}

/// 供前端检测页面调用，返回 Git 安装状态
pub fn detect() -> DetectResult {
    match detect_git() {
        Some(version) => DetectResult {
            name: "Git".to_string(),
            installed: true,
            version: Some(version),
        },
        None => DetectResult {
            name: "Git".to_string(),
            installed: false,
            version: None,
        },
    }
}
