/**
 * ---
 * role: 子进程辅助模块，为各安装命令统一封装跨平台进程创建细节
 * depends:
 *   - ../types.rs
 * exports:
 *   - command
 *   - apply_windows_command_flags
 *   - output_text
 *   - error_text
 *   - summarize_output
 * status: IMPLEMENTED
 * functions:
 *   - command(program: impl AsRef<OsStr>) -> Command
 *     创建一个已应用平台默认进程参数的 Command。
 *     Windows 下自动设置 CREATE_NO_WINDOW，避免弹出黑框；非 Windows 保持无害。
 *
 *   - apply_windows_command_flags(command: &mut Command) -> &mut Command
 *     对已有 Command 应用平台相关创建参数。
 *
 *   - output_text(bytes: &[u8]) -> String
 *     将 stdout/stderr 字节转换为去首尾空白的 UTF-8 文本。
 *
 *   - error_text(output: &Output) -> String
 *     优先返回 stderr，若 stderr 为空则回退 stdout。
 *
 *   - summarize_output(output: &Output) -> String
 *     组合退出码与 stdout/stderr，便于将真实错误返回给 StepResult。
 * ---
 */

use std::ffi::OsStr;
use std::process::{Command, Output};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 创建一个已应用平台默认参数的 Command。
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut cmd = Command::new(program);
    apply_windows_command_flags(&mut cmd);
    cmd
}

/// 为已有 Command 应用平台相关参数。
pub fn apply_windows_command_flags(command: &mut Command) -> &mut Command {
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}

/// 将输出字节转换为可读文本。
pub fn output_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}

/// 优先返回 stderr，若为空则回退 stdout。
pub fn error_text(output: &Output) -> String {
    let stderr = output_text(&output.stderr);
    if !stderr.is_empty() {
        return stderr;
    }

    let stdout = output_text(&output.stdout);
    if !stdout.is_empty() {
        return stdout;
    }

    format!("process exited with code {:?}", output.status.code())
}

/// 汇总进程输出，便于错误透传。
pub fn summarize_output(output: &Output) -> String {
    let stdout = output_text(&output.stdout);
    let stderr = output_text(&output.stderr);
    let exit_code = output.status.code();

    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => format!("exit code: {:?}", exit_code),
        (false, true) => format!("exit code: {:?}; stdout: {}", exit_code, stdout),
        (true, false) => format!("exit code: {:?}; stderr: {}", exit_code, stderr),
        (false, false) => format!("exit code: {:?}; stdout: {}; stderr: {}", exit_code, stdout, stderr),
    }
}
