/**
 * ---
 * role: Claude Code Onboarding 配置写入，跳过首次登录验证步骤
 * depends:
 *   - ../types.rs
 * exports:
 *   - configure_onboarding
 * status: IMPLEMENTED
 * functions:
 *   - configure_onboarding() -> StepResult
 *     读取 ~/.claude.json（若不存在则创建空 {}）
 *     合并写入以下字段（保留原有字段）：
 *       "hasCompletedOnboarding": true
 *       "hasTrustDialogAccepted": true
 *     写回 ~/.claude.json
 *     成功返回 StepResult{status: "done", message: "~/.claude.json 已配置"}
 *     失败（权限不足等）返回 StepResult{status: "error"}
 *
 *   - get_claude_json_path() -> PathBuf
 *     返回 ~/.claude.json 的完整路径（跨平台）
 *     Windows: C:\Users\<user>\.claude.json
 *     macOS/Linux: /home/<user>/.claude.json 或 /Users/<user>/.claude.json
 * ---
 */

use crate::types::StepResult;
use std::path::PathBuf;
use serde_json;
use dirs;
use std::fs;

pub fn detect() -> crate::types::DetectResult {
    let path = get_claude_json_path();
    let configured = path.exists() && {
        fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            .and_then(|v| v.get("hasCompletedOnboarding").and_then(|f| f.as_bool()))
            .unwrap_or(false)
    };
    crate::types::DetectResult {
        name: "Onboarding 配置".to_string(),
        installed: configured,
        version: None,
    }
}

pub async fn configure_onboarding() -> StepResult {
    tokio::task::spawn_blocking(configure_onboarding_sync)
        .await
        .unwrap_or_else(|_| StepResult {
            name: "Onboarding 配置".to_string(),
            status: "error".to_string(),
            message: "任务执行失败".to_string(),
            version: None,
        })
}

fn configure_onboarding_sync() -> StepResult {
    // 检测是否已配置
    let det = detect();
    if det.installed {
        return StepResult {
            name: "Onboarding 配置".to_string(),
            status: "skipped".to_string(),
            message: "已配置，跳过".to_string(),
            version: None,
        };
    }

    let path = get_claude_json_path();

    let mut json: serde_json::Value = if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => {
                    return StepResult {
                        name: "Onboarding 配置".to_string(),
                        status: "error".to_string(),
                        message: format!("解析 ~/.claude.json 失败: {}", e),
                        version: None,
                    };
                }
            },
            Err(e) => {
                return StepResult {
                    name: "Onboarding 配置".to_string(),
                    status: "error".to_string(),
                    message: format!("读取 ~/.claude.json 失败: {}", e),
                    version: None,
                };
            }
        }
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };

    if let Some(map) = json.as_object_mut() {
        map.insert("hasCompletedOnboarding".to_string(), serde_json::Value::Bool(true));
        map.insert("hasTrustDialogAccepted".to_string(), serde_json::Value::Bool(true));
    } else {
        return StepResult {
            name: "Onboarding 配置".to_string(),
            status: "error".to_string(),
            message: "~/.claude.json 不是一个 JSON 对象".to_string(),
            version: None,
        };
    }

    match serde_json::to_string_pretty(&json) {
        Ok(content) => match fs::write(&path, content) {
            Ok(_) => StepResult {
                name: "Onboarding 配置".to_string(),
                status: "done".to_string(),
                message: "~/.claude.json 已配置，跳过登录验证".to_string(),
                version: None,
            },
            Err(e) => StepResult {
                name: "Onboarding 配置".to_string(),
                status: "error".to_string(),
                message: format!("写入 ~/.claude.json 失败: {}", e),
                version: None,
            },
        },
        Err(e) => StepResult {
            name: "Onboarding 配置".to_string(),
            status: "error".to_string(),
            message: format!("序列化 JSON 失败: {}", e),
            version: None,
        },
    }
}

fn get_claude_json_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude.json")
}
