/**
 * ---
 * role: Miniconda 检测与安装，使用 Node.js 下载 + 清华镜像
 * depends:
 *   - ../types.rs
 *   - ./download.rs
 *   - ./process.rs
 * exports:
 *   - ensure_conda
 *   - detect
 *   - detect_conda
 *   - download_conda_installer
 *   - install_conda_silent
 * status: IMPLEMENTED
 * functions:
 *   - detect_conda(os: &str) -> Option<String>
 *     检测 Miniconda 是否已安装：
 *       1. 固定路径检测：~/miniconda3/condabin/conda.bat（Windows）或 ~/miniconda3/bin/conda（macOS/Linux）
 *       2. PATH 检测：运行 `conda --version`
 *     返回版本字符串或 None
 *
 *   - download_conda_installer(os: &str, arch: &str, dest: &Path) -> Result<(), String>
 *     使用 Node.js 下载 Miniconda 安装程序
 *     下载源优先级：
 *       1. 清华镜像：https://mirrors.tuna.tsinghua.edu.cn/anaconda/miniconda/
 *       2. 回退到官方源：https://repo.anaconda.com/miniconda/
 *     根据 os/arch 选择正确的安装包文件名
 *
 *   - install_conda_silent(os: &str, installer_path: &Path) -> Result<(), String>
 *     静默安装 Miniconda：
 *       Windows: .exe /S /D=%USERPROFILE%\miniconda3
 *       macOS/Linux: bash installer.sh -b -p ~/miniconda3
 *
 *   - ensure_conda(os: &str, arch: &str) -> StepResult
 *     完整流程：检测 → 下载 → 安装 → 验证
 *     1. 调用 detect_conda()，已安装则返回 skipped
 *     2. 未安装则调用 download_conda_installer 下载安装程序
 *     3. 调用 install_conda_silent 执行静默安装
 *     4. 安装后重新验证
 *     5. 返回 done 或 error
 *
 *   - get_miniconda_url(os: &str, arch: &str) -> String
 *     根据 os/arch 返回对应的 Miniconda 下载 URL（优先清华镜像）
 *
 *   - get_default_install_path(os: &str) -> PathBuf
 *     返回默认安装路径（~/miniconda3 或 %USERPROFILE%\miniconda3）
 *
 *   - detect() -> DetectResult
 *     供前端检测页面调用，返回 Miniconda 安装状态
 * ---
 */

use std::path::{Path, PathBuf};

use crate::types::{DetectResult, StepResult};

use super::download::download_with_fallback;
use super::process::{command, output_text, summarize_output};

fn run_conda_version(executable: &Path) -> Option<String> {
    let output = command(executable).arg("--version").output().ok()?;
    output.status.success().then(|| output_text(&output.stdout))
}

fn run_conda_version_from_path() -> Option<String> {
    let output = command("conda").arg("--version").output().ok()?;
    output.status.success().then(|| output_text(&output.stdout))
}

#[cfg(target_os = "windows")]
fn candidate_conda_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // USERPROFILE paths (most common)
    if let Ok(home_dir) = std::env::var("USERPROFILE") {
        let home = PathBuf::from(home_dir);

        // Check both Scripts/conda.exe and condabin/conda.bat for each variant
        for variant in &["miniconda3", "anaconda3", "Anaconda3"] {
            paths.push(home.join(variant).join("Scripts").join("conda.exe"));
            paths.push(home.join(variant).join("condabin").join("conda.bat"));
        }
    }

    // LOCALAPPDATA paths
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let local = PathBuf::from(local_app_data);

        for variant in &["miniconda3", "anaconda3", "Anaconda3"] {
            paths.push(local.join(variant).join("Scripts").join("conda.exe"));
            paths.push(local.join(variant).join("condabin").join("conda.bat"));
        }
    }

    // ProgramData paths (system-wide installations)
    let program_data = PathBuf::from("C:\\ProgramData");
    for variant in &["miniconda3", "anaconda3", "Anaconda3"] {
        paths.push(program_data.join(variant).join("Scripts").join("conda.exe"));
        paths.push(program_data.join(variant).join("condabin").join("conda.bat"));
    }

    paths
}

#[cfg(not(target_os = "windows"))]
fn candidate_conda_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(home_dir) = std::env::var("HOME") {
        let home = PathBuf::from(home_dir);
        paths.push(home.join("miniconda3").join("bin").join("conda"));
        paths.push(home.join("anaconda3").join("bin").join("conda"));
        paths.push(home.join("Anaconda3").join("bin").join("conda"));
    }

    paths
}

/// 检测 Miniconda/Anaconda 是否已安装
pub fn detect_conda(_os: &str) -> Option<String> {
    let candidates = candidate_conda_paths();
    eprintln!("[conda] Checking {} candidate paths", candidates.len());

    for candidate in candidates {
        eprintln!("[conda] Checking: {}", candidate.display());

        if candidate.exists() {
            eprintln!("[conda] Found executable at: {}", candidate.display());

            match run_conda_version(&candidate) {
                Some(version) => {
                    eprintln!("[conda] Successfully detected: {}", version.trim());
                    return Some(version);
                }
                None => {
                    eprintln!("[conda] File exists but failed to execute or get version");
                }
            }
        }
    }

    eprintln!("[conda] No conda found in candidate paths, checking PATH");

    match run_conda_version_from_path() {
        Some(version) => {
            eprintln!("[conda] Found in PATH: {}", version.trim());
            Some(version)
        }
        None => {
            eprintln!("[conda] Not found in PATH either");
            None
        }
    }
}

/// 获取 Miniconda 下载 URL
fn get_miniconda_url(os: &str, arch: &str) -> Vec<String> {
    let (ext, platform) = match os {
        "windows" => ("exe", "Windows"),
        "macos" => ("sh", "MacOSX"),
        "linux" => ("sh", "Linux"),
        _ => ("sh", "Linux"),
    };

    let arch_suffix = match arch {
        "arm64" => "arm64",
        _ => "x86_64",
    };

    let filename = format!("Miniconda3-latest-{}-{}.{}", platform, arch_suffix, ext);

    vec![
        format!("https://mirrors.tuna.tsinghua.edu.cn/anaconda/miniconda/{}", filename),
        format!("https://repo.anaconda.com/miniconda/{}", filename),
    ]
}

/// 获取默认安装路径
fn get_default_install_path(_os: &str) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
        PathBuf::from(home).join("miniconda3")
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join("miniconda3")
    }
}

/// 下载 Miniconda 安装程序
pub fn download_conda_installer(os: &str, arch: &str, dest: &Path) -> Result<(), String> {
    let urls = get_miniconda_url(os, arch);
    let url_refs: Vec<&str> = urls.iter().map(|s| s.as_str()).collect();
    download_with_fallback(&url_refs, dest)
}

/// 静默安装 Miniconda
pub fn install_conda_silent(os: &str, installer_path: &Path) -> Result<(), String> {
    let install_path = get_default_install_path(os);

    println!("Starting Miniconda installation (this may take several minutes)...");

    if os == "windows" {
        let mut child = command(installer_path)
            .args(["/S", &format!("/D={}", install_path.display())])
            .spawn()
            .map_err(|e| format!("Failed to start Miniconda installer: {}", e))?;

        wait_for_installation(&mut child, 300)?; // 5 minutes timeout

        Ok(())
    } else {
        let mut child = command("bash")
            .arg(installer_path)
            .args(["-b", "-p", install_path.to_string_lossy().as_ref()])
            .spawn()
            .map_err(|e| format!("Failed to start Miniconda installer: {}", e))?;

        wait_for_installation(&mut child, 300)?; // 5 minutes timeout

        Ok(())
    }
}

/// Wait for installation process with timeout and progress logging
fn wait_for_installation(child: &mut std::process::Child, timeout_secs: u64) -> Result<(), String> {
    use std::time::{Duration, Instant};
    use std::thread::sleep;

    let timeout = Duration::from_secs(timeout_secs);
    let started_at = Instant::now();
    let mut last_progress_log = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    println!("Miniconda installation completed successfully");
                    return Ok(());
                } else {
                    return Err(format!("Miniconda installer exited with status: {}", status));
                }
            }
            Ok(None) => {
                let elapsed = started_at.elapsed();

                // Log progress every 30 seconds
                if last_progress_log.elapsed() > Duration::from_secs(30) {
                    println!("Installation in progress... ({:.0}s elapsed)", elapsed.as_secs());
                    last_progress_log = Instant::now();
                }

                if elapsed >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("Installation timed out after {}s", timeout_secs));
                }
                sleep(Duration::from_millis(500));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Failed while waiting for installer: {}", e));
            }
        }
    }
}

/// 完整流程：检测 → 下载 → 安装 → 验证
pub fn ensure_conda(os: &str, arch: &str) -> StepResult {
    let step_name = "Miniconda";

    let maybe_proxy = std::env::var("HTTPS_PROXY")
        .ok()
        .or_else(|| std::env::var("HTTP_PROXY").ok())
        .or_else(|| std::env::var("https_proxy").ok())
        .or_else(|| std::env::var("http_proxy").ok());

    if let Some(version) = detect_conda(os) {
        return StepResult {
            name: step_name.to_string(),
            status: "skipped".to_string(),
            message: format!("Miniconda already installed: {}", version),
            version: Some(version),
        };
    }

    let temp_dir = std::env::temp_dir();
    let ext = if os == "windows" { "exe" } else { "sh" };
    let installer_path = temp_dir.join(format!("miniconda-installer.{}", ext));

    if let Err(e) = download_conda_installer(os, arch, &installer_path) {
        let proxy_hint = maybe_proxy
            .as_ref()
            .map(|p| format!(" (detected proxy: {})", p))
            .unwrap_or_default();

        return StepResult {
            name: step_name.to_string(),
            status: "error".to_string(),
            message: format!(
                "Failed to download Miniconda installer (network unstable or blocked).{} Error: {}",
                proxy_hint, e
            ),
            version: None,
        };
    }

    if let Err(e) = install_conda_silent(os, &installer_path) {
        return StepResult {
            name: step_name.to_string(),
            status: "error".to_string(),
            message: format!("Failed to install Miniconda: {}", e),
            version: None,
        };
    }

    let _ = std::fs::remove_file(&installer_path);

    match detect_conda(os) {
        Some(version) => StepResult {
            name: step_name.to_string(),
            status: "done".to_string(),
            message: format!("Miniconda installed successfully: {}", version),
            version: Some(version),
        },
        None => StepResult {
            name: step_name.to_string(),
            status: "error".to_string(),
            message: "Miniconda installation completed but verification failed".to_string(),
            version: None,
        },
    }
}

/// 供前端检测页面调用
pub fn detect() -> DetectResult {
    let os = std::env::consts::OS;
    match detect_conda(os) {
        Some(version) => DetectResult {
            name: "Miniconda".to_string(),
            installed: true,
            version: Some(version),
        },
        None => DetectResult {
            name: "Miniconda".to_string(),
            installed: false,
            version: None,
        },
    }
}
