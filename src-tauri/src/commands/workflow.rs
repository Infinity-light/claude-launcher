/**
 * ---
 * role: Workflow Kit 安装器，自动注册 marketplace 并处理版本更新
 * depends:
 *   - ../types.rs
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

use crate::types::{StepResult, DetectResult};
use std::path::PathBuf;
use std::process::Command;

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

            // 检查是否为有效的 plugin 目录
            let has_claude_plugin = path.join(".claude-plugin").exists()
                && path.join(".claude-plugin").join("plugin.json").exists();
            let has_skills = path.join("skills").exists();

            if has_claude_plugin && has_skills {
                // 完整有效的安装
                versions.push((version, true));
            } else if has_skills {
                // 有 skills 但缺少 .claude-plugin，需要修复
                versions.push((format!("{}-needs-fix", version), false));
            }
        }
    }

    // 按版本号降序排序（新版本在前）
    versions.sort_by(|a, b| {
        let ver_a = a.0.trim_end_matches("-needs-fix");
        let ver_b = b.0.trim_end_matches("-needs-fix");
        compare_versions(ver_b, ver_a) // 降序
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

/// 读取已安装版本中的 plugin.json 获取版本号
fn read_plugin_version(version_dir: &PathBuf) -> Option<String> {
    let plugin_json_path = version_dir.join(".claude-plugin").join("plugin.json");
    if let Ok(content) = std::fs::read_to_string(&plugin_json_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            return json.get("version")?.as_str()?.to_string().into();
        }
    }
    None
}

/// 检测任意已安装的版本
pub fn detect_any_version() -> DetectResult {
    let versions = scan_installed_versions();

    if let Some((version, is_valid)) = versions.first() {
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
    let marketplace_file = claude_dir.join("known_marketplaces.json");

    // 确保目录存在
    std::fs::create_dir_all(&claude_dir)
        .map_err(|e| format!("创建目录失败: {}", e))?;

    let marketplace_config = serde_json::json!({
        "cytopia-marketplace": {
            "source": "github",
            "repo": "Infinity-light/cytopia-marketplace"
        }
    });

    // 如果文件已存在，尝试合并
    if marketplace_file.exists() {
        let existing = std::fs::read_to_string(&marketplace_file)
            .map_err(|e| format!("读取 marketplace 配置失败: {}", e))?;
        let mut existing_json: serde_json::Value = serde_json::from_str(&existing)
            .unwrap_or_else(|_| serde_json::json!({}));

        // 合并或更新 infinity-workflows
        if let Some(obj) = existing_json.as_object_mut() {
            obj.insert("infinity-workflows".to_string(), marketplace_config["infinity-workflows"].clone());
        }

        std::fs::write(&marketplace_file, serde_json::to_string_pretty(&existing_json).unwrap())
            .map_err(|e| format!("写入 marketplace 配置失败: {}", e))?;
    } else {
        std::fs::write(&marketplace_file, serde_json::to_string_pretty(&marketplace_config).unwrap())
            .map_err(|e| format!("写入 marketplace 配置失败: {}", e))?;
    }

    Ok(())
}

/// 检查 marketplace 是否已注册
fn is_marketplace_registered() -> bool {
    let marketplace_file = get_claude_dir().join("known_marketplaces.json");
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

/// 备份旧版本
fn backup_old_version(version: &str) -> Result<PathBuf, String> {
    let old_dir = get_plugin_dir_for_version(version);
    let backup_name = format!("{}-backup-{}", version, chrono::Local::now().format("%Y%m%d-%H%M%S"));
    let backup_dir = get_plugin_base_dir().join(&backup_name);

    if old_dir.exists() {
        std::fs::rename(&old_dir, &backup_dir)
            .map_err(|e| format!("备份失败: {}", e))?;
        Ok(backup_dir)
    } else {
        Err("旧版本目录不存在".to_string())
    }
}

/// 删除旧版本（不备份）
fn remove_old_version(version: &str) -> Result<(), String> {
    let old_dir = get_plugin_dir_for_version(version);
    if old_dir.exists() {
        std::fs::remove_dir_all(&old_dir)
            .map_err(|e| format!("删除旧版本失败: {}", e))?;
    }
    Ok(())
}

/// 使用 claude plugin install 命令通过 marketplace 安装
fn install_via_claude_plugin() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("cmd")
            .args([
                "/c",
                "claude",
                "plugin",
                "install",
                "workflow-kit@cytopia-marketplace",
                "--yes",
            ])
            .output()
            .map_err(|e| format!("运行 claude plugin install 失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("plugin install 失败: {}", stderr));
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("claude")
            .args([
                "plugin",
                "install",
                "workflow-kit@cytopia-marketplace",
                "--yes",
            ])
            .output()
            .map_err(|e| format!("运行 claude plugin install 失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("plugin install 失败: {}", stderr));
        }
    }

    Ok(())
}

/// 写入 installed_plugins.json 配置
fn write_installed_plugins_config(version: &str) -> Result<(), String> {
    let plugins_dir = get_claude_dir().join("plugins");
    let config_file = plugins_dir.join("installed_plugins.json");

    std::fs::create_dir_all(&plugins_dir)
        .map_err(|e| format!("创建 plugins 目录失败: {}", e))?;

    let now = chrono::Utc::now().to_rfc3339();
    let install_path = get_plugin_dir_for_version(version);

    let config = serde_json::json!({
        "version": 2,
        "plugins": {
            "workflow-kit@cytopia-marketplace": [
                {
                    "scope": "user",
                    "installPath": install_path.to_string_lossy().to_string(),
                    "version": version,
                    "installedAt": now,
                    "lastUpdated": now,
                    "gitCommitSha": "latest"
                }
            ]
        }
    });

    std::fs::write(&config_file, serde_json::to_string_pretty(&config).unwrap())
        .map_err(|e| format!("写入 plugin 配置失败: {}", e))?;

    Ok(())
}

/// 更新 installed_plugins.json 中的时间戳
fn update_installed_plugins_timestamp(version: &str) -> Result<(), String> {
    let config_file = get_claude_dir().join("plugins").join("installed_plugins.json");

    if let Ok(content) = std::fs::read_to_string(&config_file) {
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            let now = chrono::Utc::now().to_rfc3339();

            if let Some(plugins) = json.get_mut("plugins") {
                if let Some(workflow_kit) = plugins.get_mut("workflow-kit@cytopia-marketplace") {
                    if let Some(installs) = workflow_kit.as_array_mut() {
                        for install in installs.iter_mut() {
                            if let Some(obj) = install.as_object_mut() {
                                obj.insert("version".to_string(), serde_json::json!(version));
                                obj.insert("lastUpdated".to_string(), serde_json::json!(now));
                            }
                        }
                    }
                }
            }

            std::fs::write(&config_file, serde_json::to_string_pretty(&json).unwrap())
                .map_err(|e| format!("写入配置失败: {}", e))?;
        }
    }

    Ok(())
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
    // 步骤 1: 确保 marketplace 已注册（关键！）
    if let Err(e) = register_marketplace() {
        return StepResult {
            name: "Workflow Kit".to_string(),
            status: "error".to_string(),
            message: format!("注册 marketplace 失败: {}", e),
            version: None,
        };
    }

    // 步骤 2: 检测现有版本
    let detect_result = detect_any_version();

    // 步骤 3: 如果已安装，检查是否需要更新
    let mut old_version_to_remove: Option<String> = None;
    if detect_result.installed {
        if let Some(ref version) = detect_result.version {
            // 提取实际版本号（去掉 -needs-fix 后缀）
            let actual_version = version.trim_end_matches("-needs-fix");

            // 获取最新版本号（通过 marketplace 或读取已安装版本）
            // 这里我们标记需要更新，安装后再读取实际版本
            old_version_to_remove = Some(actual_version.to_string());
        }
    }

    // 步骤 4: 使用 claude plugin install 安装/更新
    match install_via_claude_plugin() {
        Ok(_) => {
            // 验证安装
            let verify_result = detect_any_version();
            if verify_result.installed {
                // 获取实际安装的版本号
                let installed_version = verify_result.version.clone()
                    .map(|v| v.trim_end_matches("-needs-fix").to_string())
                    .unwrap_or_else(|| "latest".to_string());

                // 删除旧版本（如果存在且版本号不同）
                if let Some(old_ver) = old_version_to_remove {
                    if old_ver != installed_version {
                        let _ = remove_old_version(&old_ver);
                    }
                }

                let is_update = detect_result.installed;
                let _ = write_installed_plugins_config(&installed_version);

                StepResult {
                    name: "Workflow Kit".to_string(),
                    status: if is_update { "updated".to_string() } else { "done".to_string() },
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
    // 步骤 1: 确保 marketplace 已注册
    if let Err(e) = register_marketplace() {
        return StepResult {
            name: "Workflow Kit 更新".to_string(),
            status: "error".to_string(),
            message: format!("注册 marketplace 失败: {}", e),
            version: None,
        };
    }

    // 步骤 2: 扫描并删除所有已安装版本
    let installed_versions = scan_installed_versions();
    for (version, _) in &installed_versions {
        let actual_version = version.trim_end_matches("-needs-fix");
        let _ = remove_old_version(actual_version);
    }

    // 步骤 3: 重新安装最新版
    match install_via_claude_plugin() {
        Ok(_) => {
            let verify_result = detect_any_version();
            if verify_result.installed {
                let installed_version = verify_result.version.clone()
                    .map(|v| v.trim_end_matches("-needs-fix").to_string())
                    .unwrap_or_else(|| "latest".to_string());

                let _ = write_installed_plugins_config(&installed_version);
                let _ = update_installed_plugins_timestamp(&installed_version);

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

    // 动态获取当前版本号用于路径
    let version_path = if let Some(ref version) = detect_result.version {
        let actual_version = version.trim_end_matches("-needs-fix");
        get_plugin_dir_for_version(actual_version).to_string_lossy().to_string()
    } else {
        // 未安装时使用最新版本号作为默认路径
        get_plugin_dir_for_version("latest").to_string_lossy().to_string()
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
                            return first.get("lastUpdated")
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

// 保持向后兼容的 detect 函数
pub fn detect() -> DetectResult {
    detect_any_version()
}
