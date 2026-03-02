/**
 * ---
 * role: Miniconda 检测与安装，支持 Windows / macOS Intel / macOS ARM / Linux
 * depends:
 *   - ../types.rs
 * exports:
 *   - ensure_conda
 * status: IMPLEMENTED
 * functions:
 *   - ensure_conda(os: &str, arch: &str) -> StepResult
 *     1. 检测：运行 `conda --version`（优先检测常见路径下 conda 可执行文件）：
 *        Windows: %USERPROFILE%\miniconda3\condabin\conda.bat 或 PATH 中
 *        macOS/Linux: ~/miniconda3/bin/conda 或 PATH 中
 *        已安装则返回 StepResult{status: "skipped"}
 *     2. 安装（如未安装）：
 *        下载对应平台安装包（从 repo.anaconda.com/miniconda/）：
 *        Windows: Miniconda3-latest-Windows-x86_64.exe → 静默安装 /S /D=%USERPROFILE%\miniconda3
 *        macOS Intel: Miniconda3-latest-MacOSX-x86_64.sh → bash installer.sh -b -p ~/miniconda3
 *        macOS ARM:   Miniconda3-latest-MacOSX-arm64.sh  → bash installer.sh -b -p ~/miniconda3
 *        Linux:       Miniconda3-latest-Linux-x86_64.sh  → bash installer.sh -b -p ~/miniconda3
 *     3. 安装后验证 conda --version
 *     4. 失败返回 StepResult{status: "error"}
 *
 *   - get_miniconda_url(os: &str, arch: &str) -> String
 *     根据 os/arch 返回对应的 Miniconda 下载 URL
 *
 *   - get_default_install_path(os: &str) -> PathBuf
 *     返回默认安装路径（~/miniconda3 或 %USERPROFILE%\miniconda3）
 * ---
 */

use crate::types::StepResult;
use std::path::PathBuf;
use std::process::Command;

/// 检测 conda 是否已安装，已安装返回版本字符串，否则返回 None。
/// 检测顺序：先检查常见固定路径，再用 which 搜 PATH。
fn detect_conda(os: &str) -> Option<String> {
    // 1. 检查常见固定路径（不依赖 PATH，对刚安装未重启 shell 的情况友好）
    let home = dirs::home_dir();
    let fixed_paths: Vec<PathBuf> = if os == "windows" {
        let mut paths = vec![];
        if let Some(h) = &home {
            paths.push(h.join("miniconda3").join("condabin").join("conda.bat"));
            paths.push(h.join("miniconda3").join("Scripts").join("conda.exe"));
            // Also check common Anaconda paths (not just miniconda3)
            paths.push(h.join("anaconda3").join("condabin").join("conda.bat"));
            paths.push(h.join("anaconda3").join("Scripts").join("conda.exe"));
            paths.push(h.join("Anaconda3").join("condabin").join("conda.bat"));
            paths.push(h.join("Anaconda3").join("Scripts").join("conda.exe"));
        }
        paths
    } else {
        // macOS / Linux
        let mut paths = vec![];
        if let Some(h) = &home {
            paths.push(h.join("miniconda3").join("bin").join("conda"));
        }
        paths
    };

    for path in &fixed_paths {
        if path.exists() {
            // 尝试运行获取版本
            if let Some(ver) = run_conda_version(path.to_str().unwrap_or("conda")) {
                return Some(ver);
            }
            // 文件存在但运行失败也认为已安装（.bat 在非 cmd 环境下可能失败）
            return Some("unknown".to_string());
        }
    }

    // 2. 通过 PATH 检测（Windows 用 cmd /c where 获取完整 PATH）
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("cmd").args(["/c", "where conda"]).output() {
            if output.status.success() {
                let first = String::from_utf8_lossy(&output.stdout);
                let first_path = first.lines().next().unwrap_or("").trim();
                if !first_path.is_empty() {
                    if let Some(ver) = run_conda_version(first_path) {
                        return Some(ver);
                    }
                    return Some("unknown".to_string());
                }
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if which::which("conda").is_ok() {
            if let Some(ver) = run_conda_version("conda") {
                return Some(ver);
            }
            return Some("unknown".to_string());
        }
    }

    None
}

/// 运行 `<conda_bin> --version` 并返回版本字符串，失败返回 None。
fn run_conda_version(conda_bin: &str) -> Option<String> {
    // On Windows, .bat files must be run via cmd /c
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/c", &format!("\"{}\" --version", conda_bin)])
            .output()
            .ok()?
    } else {
        Command::new(conda_bin).arg("--version").output().ok()?
    };

    if output.status.success() {
        let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if ver.is_empty() {
            let ver_err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if !ver_err.is_empty() { return Some(ver_err); }
        }
        Some(ver)
    } else {
        // conda --version sometimes exits non-zero but still prints version to stderr
        let ver_err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !ver_err.is_empty() { return Some(ver_err); }
        None
    }
}

/// 下载文件到指定路径，使用 curl（macOS/Linux）或 PowerShell（Windows）。
fn download_file(url: &str, dest: &PathBuf, os: &str) -> Result<(), String> {
    let status = if os == "windows" {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
                    url,
                    dest.display()
                ),
            ])
            .status()
    } else {
        Command::new("curl")
            .args(["-fsSL", "-o", dest.to_str().unwrap_or(""), url])
            .status()
    };

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("下载失败，退出码：{}", s)),
        Err(e) => Err(format!("下载命令执行失败：{}", e)),
    }
}

pub fn detect(os: &str) -> crate::types::DetectResult {
    let version = detect_conda(os);
    crate::types::DetectResult {
        name: "Miniconda".to_string(),
        installed: version.is_some(),
        version,
    }
}

pub async fn ensure_conda(os: &str, arch: &str) -> StepResult {
    let os = os.to_string();
    let arch = arch.to_string();
    tokio::task::spawn_blocking(move || ensure_conda_sync(&os, &arch))
        .await
        .unwrap_or_else(|_| StepResult {
            name: "Miniconda".to_string(),
            status: "error".to_string(),
            message: "任务执行失败".to_string(),
            version: None,
        })
}

fn ensure_conda_sync(os: &str, arch: &str) -> StepResult {
    // --- 检测阶段 ---
    if let Some(ver) = detect_conda(os) {
        return StepResult {
            name: "Miniconda".to_string(),
            status: "skipped".to_string(),
            message: "Conda 已安装，跳过".to_string(),
            version: Some(ver),
        };
    }

    // --- 安装阶段 ---
    let url = get_miniconda_url(os, arch);
    let install_path = get_default_install_path(os);

    // 确定临时文件名
    let installer_name = if os == "windows" {
        "miniconda_installer.exe"
    } else {
        "miniconda_installer.sh"
    };
    let installer_path = std::env::temp_dir().join(installer_name);

    // 下载安装包
    if let Err(e) = download_file(&url, &installer_path, os) {
        return StepResult {
            name: "Miniconda".to_string(),
            status: "error".to_string(),
            message: format!("下载 Miniconda 安装包失败：{}", e),
            version: None,
        };
    }

    // 执行安装
    let install_path_str = install_path.to_string_lossy().to_string();
    let install_status = if os == "windows" {
        Command::new(&installer_path)
            .args(["/S", &format!("/D={}", install_path_str)])
            .status()
    } else {
        // 确保脚本有可执行权限
        let _ = Command::new("chmod")
            .args(["+x", installer_path.to_str().unwrap_or("")])
            .status();
        Command::new("bash")
            .args([
                installer_path.to_str().unwrap_or(""),
                "-b",
                "-p",
                &install_path_str,
            ])
            .status()
    };

    match install_status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            return StepResult {
                name: "Miniconda".to_string(),
                status: "error".to_string(),
                message: format!("Miniconda 安装程序退出码：{}", s),
                version: None,
            };
        }
        Err(e) => {
            return StepResult {
                name: "Miniconda".to_string(),
                status: "error".to_string(),
                message: format!("无法启动 Miniconda 安装程序：{}", e),
                version: None,
            };
        }
    }

    // --- 安装后验证（检查文件路径，不检查 PATH，因为需要重启 shell）---
    if let Some(ver) = detect_conda(os) {
        StepResult {
            name: "Miniconda".to_string(),
            status: "done".to_string(),
            message: "Miniconda 安装成功（重启终端后生效）".to_string(),
            version: Some(ver),
        }
    } else {
        StepResult {
            name: "Miniconda".to_string(),
            status: "error".to_string(),
            message: "Miniconda 安装完成但验证失败，请手动检查".to_string(),
            version: None,
        }
    }
}

fn get_miniconda_url(os: &str, arch: &str) -> String {
    let base = "https://repo.anaconda.com/miniconda";
    match (os, arch) {
        ("windows", _) => format!("{}/Miniconda3-latest-Windows-x86_64.exe", base),
        ("macos", "arm64") => format!("{}/Miniconda3-latest-MacOSX-arm64.sh", base),
        ("macos", _) => format!("{}/Miniconda3-latest-MacOSX-x86_64.sh", base),
        ("linux", "arm64") => format!("{}/Miniconda3-latest-Linux-aarch64.sh", base),
        _ => format!("{}/Miniconda3-latest-Linux-x86_64.sh", base),
    }
}

fn get_default_install_path(os: &str) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    // Windows 和 Unix 都使用 ~/miniconda3（dirs::home_dir 在 Windows 上返回 %USERPROFILE%）
    let _ = os; // os 参数在此实现中路径相同，保留参数签名与契约一致
    home.join("miniconda3")
}
