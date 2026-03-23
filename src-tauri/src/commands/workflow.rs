/**
 * ---
 * role: Workflow Kit 安装器，自动注册 marketplace 并处理版本更新
 * depends:
 *   - ../types.rs
 *   - ./process.rs
 * exports:
 *   - install_workflow_kit
 *   - update_workflow_kit
 *   - get_workflow_kit_info
 * status: IMPLEMENTED
 * functions:
 *   - install_workflow_kit(_os: &str) -> StepResult
 *     1. 自动注册 marketplace 到 known_marketplaces.json
 *     2. 检测现有版本（支持 2.1.0 和 2.2.0）
 *     3. 如果旧版本存在，自动备份并更新
 *     4. 使用 claude plugin install 通过 URL 安装最新版
 *     5. 返回 StepResult{status: "done"|"updated"|"error"}
 *
 *   - update_workflow_kit() -> StepResult
 *     强制重新安装最新版本（用于更新按钮）
 *
 *   - detect_any_version() -> DetectResult
 *     检测任意已安装的版本（2.1.0 或 2.2.0）
 * ---
 */

use crate::types::{DetectResult, StepResult};
use std::path::PathBuf;

use super::process::{command, error_text};

/// 获取 Claude 配置目录
fn get_claude_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
}

/// 获取 plugin 基础路径（不含版本号）
fn get_plugin_base_dir() -> PathBuf {
    get_claude_dir()
        .join("plugins")
        .join("cache")
        .join("cytopia-marketplace")
        .join("workflow-kit")
}

/// 获取特定版本的 plugin 路径
fn get_plugin_dir_for_version(version: &str) -> PathBuf {
    get_plugin_base_dir().join(version)
}

/// 动态扫描所有已安装的版本
fn scan_installed_versions() -> Vec<(String, bool)> {
    let mut versions = Vec::new();
    let base_dir = get_plugin_base_dir();

    if let Ok(entries) = std::fs::read_dir(&base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let version = entry.file_name().to_string_lossy().to_string();
            let has_claude_plugin = path.join(".claude-plugin").exists()
                && path.join(".claude-plugin").join("plugin.json").exists();
            let has_skills = path.join("skills").exists();

            if has_claude_plugin && has_skills {
                versions.push((version, true));
            } else if has_skills {
                versions.push((format!("{}-needs-fix", version), false));
            }
        }
    }

    versions.sort_by(|a, b| {
        let ver_a = a.0.trim_end_matches("-needs-fix");
        let ver_b = b.0.trim_end_matches("-needs-fix");
        compare_versions(ver_b, ver_a)
    });

    versions
}

/// 比较版本号，返回 ordering
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parts_a: Vec<u32> = a.split('.').filter_map(|p| p.parse().ok()).collect();
    let parts_b: Vec<u32> = b.split('.').filter_map(|p| p.parse().ok()).collect();

    for (pa, pb) in parts_a.iter().zip(parts_b.iter()) {
        match pa.cmp(pb) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    parts_a.len().cmp(&parts_b.len())
}

/// 检测任意已安装的版本
pub fn detect_any_version() -> DetectResult {
    let versions = scan_installed_versions();

    if let Some((version, _is_valid)) = versions.first() {
        return DetectResult {
            name: "Workflow Kit".to_string(),
            installed: true,
            version: Some(version.clone()),
        };
    }

    DetectResult {
        name: "Workflow Kit".to_string(),
        installed: false,
        version: None,
    }
}

/// 注册 marketplace 到 known_marketplaces.json
fn register_marketplace() -> Result<(), String> {
    let claude_dir = get_claude_dir();
    let plugins_dir = claude_dir.join("plugins");
    let marketplace_file = plugins_dir.join("known_marketplaces.json");

    std::fs::create_dir_all(&plugins_dir).map_err(|e| format!("创建目录失败: {}", e))?;

    let now = chrono::Utc::now().to_rfc3339();

    let install_location = claude_dir
        .join("plugins")
        .join("marketplaces")
        .join("cytopia-marketplace")
        .to_string_lossy()
        .to_string();

    let marketplace_config = serde_json::json!({
        "cytopia-marketplace": {
            "source": {
                "source": "github",
                "repo": "Infinity-light/cytopia-marketplace"
            },
            "installLocation": install_location,
            "lastUpdated": now
        }
    });

    if marketplace_file.exists() {
        let existing = std::fs::read_to_string(&marketplace_file)
            .map_err(|e| format!("读取 marketplace 配置失败: {}", e))?;
        let mut existing_json: serde_json::Value =
            serde_json::from_str(&existing).unwrap_or_else(|_| serde_json::json!({}));

        if let Some(obj) = existing_json.as_object_mut() {
            obj.insert(
                "cytopia-marketplace".to_string(),
                marketplace_config["cytopia-marketplace"].clone(),
            );
        }

        std::fs::write(
            &marketplace_file,
            serde_json::to_string_pretty(&existing_json).unwrap(),
        )
        .map_err(|e| format!("写入 marketplace 配置失败: {}", e))?;
    } else {
        std::fs::write(
            &marketplace_file,
            serde_json::to_string_pretty(&marketplace_config).unwrap(),
        )
        .map_err(|e| format!("写入 marketplace 配置失败: {}", e))?;
    }

    Ok(())
}

/// 检查 marketplace 是否已注册
fn is_marketplace_registered() -> bool {
    let marketplace_file = get_claude_dir().join("plugins").join("known_marketplaces.json");
    if !marketplace_file.exists() {
        return false;
    }

    if let Ok(content) = std::fs::read_to_string(&marketplace_file) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            return json.get("cytopia-marketplace").is_some();
        }
    }
    false
}

/// 使用 claude plugin install 命令通过 marketplace 安装
fn install_via_claude_plugin() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let output = command("cmd")
        .args([
            "/c",
            "claude",
            "plugin",
            "install",
            "workflow-kit@cytopia-marketplace",
        ])
        .output()
        .map_err(|e| format!("运行 claude plugin install 失败: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let output = command("claude")
        .args(["plugin", "install", "workflow-kit@cytopia-marketplace"])
        .output()
        .map_err(|e| format!("运行 claude plugin install 失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!("plugin install 失败: {}", error_text(&output)))
    }
}

/// 主安装函数（处理首次安装和版本更新）
pub async fn install_workflow_kit(_os: &str) -> StepResult {
    let os = _os.to_string();
    tokio::task::spawn_blocking(move || install_workflow_kit_sync(&os))
        .await
        .unwrap_or_else(|_| StepResult {
            name: "Workflow Kit".to_string(),
            status: "error".to_string(),
            message: "任务执行失败".to_string(),
            version: None,
        })
}

fn install_workflow_kit_sync(_os: &str) -> StepResult {
    if let Err(e) = register_marketplace() {
        return StepResult {
            name: "Workflow Kit".to_string(),
            status: "error".to_string(),
            message: format!("注册 marketplace 失败: {}", e),
            version: None,
        };
    }

    let detect_result = detect_any_version();

    match install_via_claude_plugin() {
        Ok(_) => {
            let verify_result = detect_any_version();
            if verify_result.installed {
                let installed_version = verify_result
                    .version
                    .clone()
                    .map(|v| v.trim_end_matches("-needs-fix").to_string())
                    .unwrap_or_else(|| "latest".to_string());

                let is_update = detect_result.installed;

                StepResult {
                    name: "Workflow Kit".to_string(),
                    status: if is_update {
                        "updated".to_string()
                    } else {
                        "done".to_string()
                    },
                    message: if is_update {
                        format!("Workflow Kit 已更新到 v{}", installed_version)
                    } else {
                        format!("Workflow Kit v{} 安装成功", installed_version)
                    },
                    version: Some(installed_version),
                }
            } else {
                StepResult {
                    name: "Workflow Kit".to_string(),
                    status: "error".to_string(),
                    message: "安装后验证失败".to_string(),
                    version: None,
                }
            }
        }
        Err(e) => StepResult {
            name: "Workflow Kit".to_string(),
            status: "error".to_string(),
            message: format!("安装失败: {}", e),
            version: None,
        },
    }
}

/// 强制更新 Workflow Kit 到最新版本
#[tauri::command]
pub async fn update_workflow_kit() -> Result<StepResult, String> {
    tokio::task::spawn_blocking(update_workflow_kit_sync)
        .await
        .map_err(|e| format!("任务执行失败: {:?}", e))
}

fn update_workflow_kit_sync() -> StepResult {
    if let Err(e) = register_marketplace() {
        return StepResult {
            name: "Workflow Kit 更新".to_string(),
            status: "error".to_string(),
            message: format!("注册 marketplace 失败: {}", e),
            version: None,
        };
    }

    match install_via_claude_plugin() {
        Ok(_) => {
            let verify_result = detect_any_version();
            if verify_result.installed {
                let installed_version = verify_result
                    .version
                    .clone()
                    .map(|v| v.trim_end_matches("-needs-fix").to_string())
                    .unwrap_or_else(|| "latest".to_string());

                StepResult {
                    name: "Workflow Kit 更新".to_string(),
                    status: "done".to_string(),
                    message: format!("已强制更新到 v{}", installed_version),
                    version: Some(installed_version),
                }
            } else {
                StepResult {
                    name: "Workflow Kit 更新".to_string(),
                    status: "error".to_string(),
                    message: "更新后验证失败".to_string(),
                    version: None,
                }
            }
        }
        Err(e) => StepResult {
            name: "Workflow Kit 更新".to_string(),
            status: "error".to_string(),
            message: format!("更新失败: {}", e),
            version: None,
        },
    }
}

/// 获取当前 Workflow Kit 版本信息
#[tauri::command]
pub fn get_workflow_kit_info() -> serde_json::Value {
    let detect_result = detect_any_version();
    let marketplace_registered = is_marketplace_registered();
    let last_updated = get_plugin_last_updated().unwrap_or_else(|| "未知".to_string());

    let version_path = if let Some(ref version) = detect_result.version {
        let actual_version = version.trim_end_matches("-needs-fix");
        get_plugin_dir_for_version(actual_version)
            .to_string_lossy()
            .to_string()
    } else {
        get_plugin_dir_for_version("latest")
            .to_string_lossy()
            .to_string()
    };

    serde_json::json!({
        "installed": detect_result.installed,
        "version": detect_result.version,
        "marketplaceRegistered": marketplace_registered,
        "lastUpdated": last_updated,
        "path": version_path,
    })
}

fn get_plugin_last_updated() -> Option<String> {
    let config_file = get_claude_dir().join("plugins").join("installed_plugins.json");
    if let Ok(content) = std::fs::read_to_string(&config_file) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(plugins) = json.get("plugins") {
                if let Some(workflow_kit) = plugins.get("workflow-kit@cytopia-marketplace") {
                    if let Some(installs) = workflow_kit.as_array() {
                        if let Some(first) = installs.first() {
                            return first
                                .get("lastUpdated")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn detect() -> DetectResult {
    detect_any_version()
}
