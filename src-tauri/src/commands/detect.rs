/**
 * ---
 * role: OS 和架构检测，返回平台信息供安装流程使用
 * depends:
 *   - ../types.rs
 * exports:
 *   - get_system_info
 *   - SystemInfo
 * status: IMPLEMENTED
 * functions:
 *   - get_system_info() -> Result<SystemInfo, String>
 *     检测当前操作系统和 CPU 架构：
 *     os: "windows" | "macos" | "linux"（读取 std::env::consts::OS）
 *     arch: "x64" | "arm64"（读取 std::env::consts::ARCH，映射 x86_64→x64, aarch64→arm64）
 *     此命令在欢迎页启动时调用一次，结果保存到前端 store
 * ---
 */

use crate::types::SystemInfo;

#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    let os = match std::env::consts::OS {
        "windows" => "windows",
        "macos" => "macos",
        "linux" => "linux",
        other => return Err(format!("Unsupported OS: {}", other)),
    };

    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => return Err(format!("Unsupported architecture: {}", other)),
    };

    Ok(SystemInfo {
        os: os.to_string(),
        arch: arch.to_string(),
    })
}
