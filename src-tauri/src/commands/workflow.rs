/**
 * ---
 * role: Workflow Kit 安装器，通过 Claude Code Plugin 系统安装
 * depends:
 *   - ../types.rs
 * exports:
 *   - install_workflow_kit
 * status: IMPLEMENTED
 * functions:
 *   - install_workflow_kit(_os: &str) -> StepResult
 *     1. 检测是否已安装：检查 ~/.claude/plugins/cache/infinity-workflows/workflow-kit/ 是否存在
 *     2. 注册 marketplace 到 ~/.claude/known_marketplaces.json
 *     3. 运行 `claude plugin install workflow-kit@infinity-workflows`
 *     4. 若命令失败，使用备用方案：手动下载并放置到 cache 目录
 *     5. 返回 StepResult{status: "done"|"error"}
 *
 *   - detect() -> DetectResult
 *     检查 plugin 是否已安装
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

/// 获取 plugin 安装路径
fn get_plugin_dir() -> PathBuf {
    get_claude_dir()
        .join("plugins")
        .join("cache")
        .join("infinity-workflows")
        .join("workflow-kit")
        .join("2.1.0")
}

/// 检测 plugin 是否已安装
pub fn detect() -> DetectResult {
    let plugin_dir = get_plugin_dir();
    let installed = plugin_dir.exists()
        && plugin_dir.join("skills").exists()
        && plugin_dir.join("CLAUDE.md").exists();
    DetectResult {
        name: "Workflow Kit".to_string(),
        installed,
        version: if installed { Some("2.1.0".to_string()) } else { None },
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
        "infinity-workflows": {
            "source": "github",
            "repo": "Infinity-light/Cytopia-claude-code-workkit-plugin"
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

/// 使用 claude plugin install 命令安装
fn install_via_claude_plugin() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 cmd /c 运行 claude plugin install
        let output = Command::new("cmd")
            .args(["/c", "claude", "plugin", "install", "workflow-kit@infinity-workflows", "--yes"])
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
            .args(["plugin", "install", "workflow-kit@infinity-workflows", "--yes"])
            .output()
            .map_err(|e| format!("运行 claude plugin install 失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("plugin install 失败: {}", stderr));
        }
    }

    Ok(())
}

/// 备用方案：手动下载并安装 plugin
fn install_manually() -> Result<(), String> {
    let plugin_dir = get_plugin_dir();
    let tmp_dir = std::env::temp_dir().join("workflow-kit-download");

    // 清理临时目录
    if tmp_dir.exists() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    // 确保 git 可用
    if which::which("git").is_err() {
        return Err("需要 Git 但未找到".to_string());
    }

    // 克隆仓库
    let clone_status = Command::new("git")
        .args([
            "clone",
            "--depth", "1",
            "--branch", "master",
            "https://github.com/Infinity-light/Cytopia-claude-code-workkit-plugin.git",
            tmp_dir.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("git clone 失败: {}", e))?;

    if !clone_status.success() {
        return Err("git clone 失败".to_string());
    }

    // 创建 plugin 目录
    std::fs::create_dir_all(&plugin_dir)
        .map_err(|e| format!("创建 plugin 目录失败: {}", e))?;

    // 复制 skills 目录
    let skills_src = tmp_dir.join("plugins").join("workflow-kit").join("skills");
    if !skills_src.exists() {
        return Err("下载的仓库中未找到 skills 目录".to_string());
    }

    copy_dir_recursive(&skills_src, &plugin_dir.join("skills"))?;

    // 复制 CLAUDE.md（如果存在）
    let claude_md_src = tmp_dir.join("plugins").join("workflow-kit").join("CLAUDE.md");
    if claude_md_src.exists() {
        std::fs::copy(&claude_md_src, &plugin_dir.join("CLAUDE.md"))
            .map_err(|e| format!("复制 CLAUDE.md 失败: {}", e))?;
    }

    // 写入 installed_plugins.json
    write_installed_plugins_config()?;

    // 清理临时目录
    let _ = std::fs::remove_dir_all(&tmp_dir);

    Ok(())
}

/// 写入 installed_plugins.json 配置
fn write_installed_plugins_config() -> Result<(), String> {
    let plugins_dir = get_claude_dir().join("plugins");
    let config_file = plugins_dir.join("installed_plugins.json");

    std::fs::create_dir_all(&plugins_dir)
        .map_err(|e| format!("创建 plugins 目录失败: {}", e))?;

    let now = chrono::Utc::now().to_rfc3339();
    let install_path = get_plugin_dir();

    let config = serde_json::json!({
        "version": 2,
        "plugins": {
            "workflow-kit@infinity-workflows": [
                {
                    "scope": "user",
                    "installPath": install_path.to_string_lossy().to_string(),
                    "version": "2.1.0",
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

/// 递归复制目录
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dst)
        .map_err(|e| format!("创建目录 {} 失败: {}", dst.display(), e))?;

    let entries = std::fs::read_dir(src)
        .map_err(|e| format!("读取目录 {} 失败: {}", src.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("遍历目录项失败: {}", e))?;
        let entry_path = entry.path();
        let file_name = entry.file_name();

        // 跳过 .git 目录
        if file_name == ".git" {
            continue;
        }

        let dest_path = dst.join(&file_name);

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &dest_path)?;
        } else {
            std::fs::copy(&entry_path, &dest_path)
                .map_err(|e| format!("复制文件 {} -> {} 失败: {}",
                    entry_path.display(), dest_path.display(), e))?;
        }
    }

    Ok(())
}

/// 主安装函数
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
    // 1. 检测是否已安装
    let detect_result = detect();
    if detect_result.installed {
        return StepResult {
            name: "Workflow Kit".to_string(),
            status: "skipped".to_string(),
            message: "Workflow Kit 已安装".to_string(),
            version: detect_result.version,
        };
    }

    // 2. 注册 marketplace
    if let Err(e) = register_marketplace() {
        return StepResult {
            name: "Workflow Kit".to_string(),
            status: "error".to_string(),
            message: format!("注册 marketplace 失败: {}", e),
            version: None,
        };
    }

    // 3. 尝试使用 claude plugin install 安装
    match install_via_claude_plugin() {
        Ok(_) => {
            // 验证安装
            let detect_result = detect();
            if detect_result.installed {
                return StepResult {
                    name: "Workflow Kit".to_string(),
                    status: "done".to_string(),
                    message: "Workflow Plugin 安装成功".to_string(),
                    version: Some("2.1.0".to_string()),
                };
            }
        }
        Err(e) => {
            // 记录错误，继续尝试备用方案
            eprintln!("claude plugin install 失败: {}, 尝试备用方案...", e);
        }
    }

    // 4. 备用方案：手动下载安装
    match install_manually() {
        Ok(_) => StepResult {
            name: "Workflow Kit".to_string(),
            status: "done".to_string(),
            message: "Workflow Kit 安装成功（手动模式）".to_string(),
            version: Some("2.1.0".to_string()),
        },
        Err(e) => StepResult {
            name: "Workflow Kit".to_string(),
            status: "error".to_string(),
            message: format!("安装失败: {}", e),
            version: None,
        },
    }
}
