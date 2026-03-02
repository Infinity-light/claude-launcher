/**
 * ---
 * role: Tauri 构建脚本，配置编译选项
 * depends:
 *   - tauri-build
 * exports:
 *   - main
 * status: PENDING
 * functions:
 *   - main()
 *     配置 Windows 子系统、资源文件等编译选项
 * ---
 */

fn main() {
    tauri_build::build()
}
