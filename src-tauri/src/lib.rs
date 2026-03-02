/**
 * ---
 * role: Tauri 应用构建器，注册所有命令和插件，对外暴露 run()
 * depends:
 *   - ./types.rs
 *   - ./commands/mod.rs
 * exports:
 *   - run
 * status: IMPLEMENTED
 * functions:
 *   - run(): void
 *     构建 Tauri 应用：
 *     1. 注册插件：opener, shell, fs, http, process
 *     2. 注册 Tauri 命令：get_system_info, start_installation, open_path
 *     3. 启动应用
 *
 * commands registered:
 *   commands::detect::get_system_info
 *   commands::installer::start_installation
 *   commands::installer::open_path
 * ---
 */

pub mod commands;
pub mod types;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::detect::get_system_info,
            commands::installer::start_installation,
            commands::installer::open_path,
            commands::installer::detect_all,
            commands::workflow::update_workflow_kit,
            commands::workflow::get_workflow_kit_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
