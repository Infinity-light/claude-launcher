/**
 * ---
 * role: CCSwitch 下载器，从 GitHub Releases 下载对应平台版本到用户 Desktop
 * depends:
 *   - ../types.rs
 * exports:
 *   - download_ccswitch
 * status: IMPLEMENTED
 * functions:
 *   - download_ccswitch(os: &str, arch: &str) -> StepResult
 *     1. 查询最新版本：GET https://api.github.com/repos/farion1231/cc-switch/releases/latest
 *        解析 JSON 获取 tag_name（如 "v3.11.1"）
 *     2. 根据 os/arch 确定文件名：
 *        Windows:       CC-Switch-{version}-Windows.msi
 *        macOS:         CC-Switch-{version}-macOS.zip
 *        Linux x64:     CC-Switch-{version}-Linux-x86_64.deb
 *        Linux arm64:   CC-Switch-{version}-Linux-aarch64.deb
 *     3. 构建下载 URL：
 *        https://github.com/farion1231/cc-switch/releases/download/{tag}/{filename}
 *     4. 下载到用户 Desktop 目录（或 Downloads 若 Desktop 不存在）
 *     5. 返回 StepResult{status: "done", message: "已下载到 <path>，version: Some(version)}
 *     6. 网络失败返回 StepResult{status: "error", message: "下载失败，可手动访问 github.com/farion1231/cc-switch"}
 *
 *   - get_download_dir(os: &str) -> PathBuf
 *     返回下载目标目录（优先 Desktop，其次 Downloads，最后 home）
 *
 *   - get_filename(os: &str, arch: &str, version: &str) -> String
 *     根据平台和版本拼接文件名
 * ---
 */

use crate::types::StepResult;
use std::path::PathBuf;
use dirs;

pub fn detect(os: &str, arch: &str) -> crate::types::DetectResult {
    let _ = arch;
    // 优先检测 ~/.cc-switch/ 目录（CCSwitch 运行后会创建）
    let cc_switch_data = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cc-switch");
    if cc_switch_data.exists() {
        return crate::types::DetectResult {
            name: "CCSwitch".to_string(),
            installed: true,
            version: None,
        };
    }
    // 其次检查桌面/下载目录
    let download_dir = get_download_dir(os);
    let installed = std::fs::read_dir(&download_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .any(|e| {
                    let name = e.file_name().to_string_lossy().to_lowercase();
                    name.starts_with("cc-switch")
                })
        })
        .unwrap_or(false);
    crate::types::DetectResult {
        name: "CCSwitch".to_string(),
        installed,
        version: None,
    }
}

pub async fn download_ccswitch(os: &str, arch: &str) -> StepResult {
    // 检测是否已安装
    let det = detect(os, arch);
    if det.installed {
        return StepResult {
            name: "CCSwitch".to_string(),
            status: "skipped".to_string(),
            message: "已安装，跳过".to_string(),
            version: None,
        };
    }

    // Step 1: Query the GitHub API for the latest release tag
    let api_url = "https://api.github.com/repos/farion1231/cc-switch/releases/latest";
    let client = match reqwest::Client::builder()
        .user_agent("claude-env-installer")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return StepResult {
                name: "CCSwitch".to_string(),
                status: "error".to_string(),
                message: format!(
                    "下载失败，请访问 https://github.com/farion1231/cc-switch（{}）",
                    e
                ),
                version: None,
            };
        }
    };

    let tag_name = match client.get(api_url).send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(json) => match json.get("tag_name").and_then(|v| v.as_str()) {
                Some(tag) => tag.to_string(),
                None => {
                    return StepResult {
                        name: "CCSwitch".to_string(),
                        status: "error".to_string(),
                        message: "下载失败，请访问 https://github.com/farion1231/cc-switch（无法解析版本号）".to_string(),
                        version: None,
                    };
                }
            },
            Err(e) => {
                return StepResult {
                    name: "CCSwitch".to_string(),
                    status: "error".to_string(),
                    message: format!(
                        "下载失败，请访问 https://github.com/farion1231/cc-switch（{}）",
                        e
                    ),
                    version: None,
                };
            }
        },
        Err(e) => {
            return StepResult {
                name: "CCSwitch".to_string(),
                status: "error".to_string(),
                message: format!(
                    "下载失败，请访问 https://github.com/farion1231/cc-switch（{}）",
                    e
                ),
                version: None,
            };
        }
    };

    // Step 2: Determine the filename based on os/arch
    // Use raw tag_name (with 'v' prefix) as it appears in actual asset names
    let filename = get_filename(os, arch, &tag_name);

    // Step 3: Build the download URL
    let download_url = format!(
        "https://github.com/farion1231/cc-switch/releases/download/{}/{}",
        tag_name, filename
    );

    // Step 4: Determine download directory
    let download_dir = get_download_dir(os);
    let dest_path = download_dir.join(&filename);

    // Step 5: Download the file
    let response = match client.get(&download_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return StepResult {
                name: "CCSwitch".to_string(),
                status: "error".to_string(),
                message: format!(
                    "下载失败，请访问 https://github.com/farion1231/cc-switch（{}）",
                    e
                ),
                version: None,
            };
        }
    };

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return StepResult {
                name: "CCSwitch".to_string(),
                status: "error".to_string(),
                message: format!(
                    "下载失败，请访问 https://github.com/farion1231/cc-switch（{}）",
                    e
                ),
                version: None,
            };
        }
    };

    if let Err(e) = std::fs::write(&dest_path, &bytes) {
        return StepResult {
            name: "CCSwitch".to_string(),
            status: "error".to_string(),
            message: format!(
                "下载失败，请访问 https://github.com/farion1231/cc-switch（写入文件失败：{}）",
                e
            ),
            version: None,
        };
    }

    // Step 6: Return success with the file path and version
    StepResult {
        name: "CCSwitch".to_string(),
        status: "done".to_string(),
        message: format!("CCSwitch 已下载到 {}", dest_path.display()),
        version: Some(tag_name),
    }
}

fn get_download_dir(_os: &str) -> PathBuf {
    // Prefer Desktop, then Downloads, then home directory
    if let Some(dir) = dirs::desktop_dir() {
        return dir;
    }
    if let Some(dir) = dirs::download_dir() {
        return dir;
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn get_filename(os: &str, arch: &str, tag: &str) -> String {
    // tag includes 'v' prefix, e.g., "v3.11.1"
    // Actual asset names: CC-Switch-v3.11.1-Windows.msi, CC-Switch-v3.11.1-Windows-Portable.zip
    match os {
        "windows" => format!("CC-Switch-{}-Windows-Portable.zip", tag),
        "macos" => format!("CC-Switch-{}-macOS.zip", tag),
        "linux" => match arch {
            "arm64" => format!("CC-Switch-{}-Linux-aarch64.AppImage", tag),
            _ => format!("CC-Switch-{}-Linux-x86_64.AppImage", tag),
        },
        _ => format!("CC-Switch-{}-Linux-x86_64.AppImage", tag),
    }
}
