/**
 * ---
 * role: 使用 Node.js 下载文件的共享模块
 * depends:
 *   - ../types.rs
 *   - ./process.rs
 * exports:
 *   - download_file_with_node
 *   - verify_file_size
 *   - get_node_path
 * status: IMPLEMENTED
 * functions:
 *   - get_node_path() -> Option<PathBuf>
 *     获取 Node.js 可执行文件路径：
 *       1. 固定路径：C:\Program Files\nodejs\node.exe（Windows）
 *       2. PATH 搜索：which node
 *     返回 node 可执行文件的完整路径
 *
 *   - download_file_with_node(url: &str, dest: &Path, progress_callback: Option<Fn(f64)>) -> Result<(), String>
 *     使用 Node.js 的 https 模块下载文件：
 *       1. 生成临时 Node.js 脚本，使用内置 https 模块下载
 *       2. 执行 node download_script.js
 *       3. 支持进度回调（通过 stdout 解析进度信息）
 *       4. 处理重定向（301/302）
 *       5. 设置合理的超时时间
 *     返回成功或错误信息
 *
 *   - verify_file_size(path: &Path, expected_size: Option<u64>) -> bool
 *     验证文件大小（简单的完整性检查）：
 *       1. 如果提供了 expected_size，验证是否匹配
 *       2. 如果没有提供，仅验证文件存在且非空
 *     返回是否通过验证
 *
 *   - download_with_fallback(urls: &[&str], dest: &Path) -> Result<(), String>
 *     带回退的下载：
 *       1. 依次尝试多个 URL（镜像源优先）
 *       2. 任一成功即返回 Ok
 *       3. 全部失败返回最后一个错误
 *
 *   - cleanup_temp_downloads() -> Result<(), String>
 *     清理临时下载文件（可选，用于安装完成后清理）
 * ---
 */

use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use super::process::{command, error_text};

/// 获取 Node.js 可执行文件路径
pub fn get_node_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let fixed_path = r"C:\Program Files\nodejs\node.exe";
        if Path::new(fixed_path).exists() {
            return Some(PathBuf::from(fixed_path));
        }
    }

    let cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
    if let Ok(output) = command(cmd).arg("node").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout);
            let path = path.lines().next().unwrap_or("").trim();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }

    None
}

/// 读取代理环境变量
fn get_proxy_env() -> Option<String> {
    std::env::var("HTTPS_PROXY")
        .ok()
        .or_else(|| std::env::var("https_proxy").ok())
        .or_else(|| std::env::var("HTTP_PROXY").ok())
        .or_else(|| std::env::var("http_proxy").ok())
}

fn wait_for_completion(child: &mut Child, script_path: &Path) -> Result<(), String> {
    let timeout = Duration::from_secs(600); // 10 minutes for slow networks
    let started_at = Instant::now();
    let mut last_progress_log = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => {
                let elapsed = started_at.elapsed();

                // Log progress every 30 seconds to show activity
                if last_progress_log.elapsed() > Duration::from_secs(30) {
                    println!("Download in progress... ({:.0}s elapsed)", elapsed.as_secs());
                    last_progress_log = Instant::now();
                }

                if elapsed >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(script_path);
                    return Err("Download timed out after 600s".to_string());
                }
                sleep(Duration::from_millis(200));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(script_path);
                return Err(format!("Failed while waiting for download script: {}", e));
            }
        }
    }
}

/// 使用 Node.js 下载文件
pub fn download_file_with_node(url: &str, dest: &Path, _progress_callback: Option<impl Fn(f64)>) -> Result<(), String> {
    let node_path = get_node_path().ok_or("Node.js not found")?;
    let dest_path_str = dest.to_string_lossy().replace('\\', "/");

    let script = format!(
        r#"
const fs = require('fs');
const https = require('https');
const http = require('http');
const {{ URL }} = require('url');

const sourceUrl = '{}';
const dest = '{}';
const timeoutMs = 20000;
const maxRedirects = 5;
const proxyUrl = process.env.HTTPS_PROXY || process.env.https_proxy || process.env.HTTP_PROXY || process.env.http_proxy || '';

function cleanupAndExit(file, message) {{
  try {{ file.destroy(); }} catch (_) {{}}
  fs.unlink(dest, () => {{
    console.error(message);
    process.exit(1);
  }});
}}

function directRequest(url, redirectsLeft) {{
  const file = fs.createWriteStream(dest);
  const req = https.get(url, {{ timeout: timeoutMs }}, (res) => {{
    if ((res.statusCode === 301 || res.statusCode === 302 || res.statusCode === 307 || res.statusCode === 308) && res.headers.location) {{
      file.close();
      fs.unlink(dest, () => {{
        if (redirectsLeft <= 0) {{
          console.error('[download] Too many redirects');
          process.exit(1);
        }}
        directRequest(res.headers.location, redirectsLeft - 1);
      }});
      return;
    }}

    if (!res.statusCode || res.statusCode >= 400) {{
      cleanupAndExit(file, `[download] HTTP ${{res.statusCode}}`);
      return;
    }}

    res.pipe(file);
    file.on('finish', () => file.close(() => process.exit(0)));
  }});

  req.on('timeout', () => req.destroy(new Error('request timeout')));
  req.on('error', (err) => cleanupAndExit(file, `[download] Error: ${{err.message}}`));
}}

function proxyRequest(urlString) {{
  const target = new URL(urlString);
  const proxy = new URL(proxyUrl);

  if (proxy.protocol !== 'http:') {{
    console.error('[download] Unsupported proxy protocol, only http:// proxy is supported by built-in downloader');
    process.exit(1);
  }}

  const file = fs.createWriteStream(dest);
  const req = http.request({{
    host: proxy.hostname,
    port: proxy.port || 80,
    method: 'CONNECT',
    path: `${{target.hostname}}:${{target.port || 443}}`,
    timeout: timeoutMs,
    headers: proxy.username ? {{
      'Proxy-Authorization': 'Basic ' + Buffer.from(`${{decodeURIComponent(proxy.username)}}:${{decodeURIComponent(proxy.password)}}`).toString('base64')
    }} : undefined,
  }});

  req.on('connect', (_res, socket) => {{
    const tls = require('tls');
    const tlsSocket = tls.connect({{ socket, servername: target.hostname }}, () => {{
      const request = [
        `GET ${{target.pathname}}${{target.search}} HTTP/1.1`,
        `Host: ${{target.host}}`,
        'Connection: close',
        'User-Agent: claude-env-installer-downloader',
        '\\r\\n'
      ].join('\\r\\n');
      tlsSocket.write(request);
    }});

    let headerParsed = false;
    let headerBuf = Buffer.alloc(0);
    let statusCode = 0;

    tlsSocket.on('data', (chunk) => {{
      if (!headerParsed) {{
        headerBuf = Buffer.concat([headerBuf, chunk]);
        const splitIndex = headerBuf.indexOf('\\r\\n\\r\\n');
        if (splitIndex === -1) return;

        const headerPart = headerBuf.slice(0, splitIndex).toString('utf8');
        const bodyPart = headerBuf.slice(splitIndex + 4);
        const statusLine = headerPart.split('\\r\\n')[0] || '';
        const match = statusLine.match(/HTTP\\/\\d.\\d\\s+(\\d+)/);
        statusCode = match ? parseInt(match[1], 10) : 0;

        if (!statusCode || statusCode >= 400) {{
          cleanupAndExit(file, `[download] HTTP ${{statusCode || 0}} through proxy`);
          return;
        }}

        headerParsed = true;
        if (bodyPart.length > 0) file.write(bodyPart);
      }} else {{
        file.write(chunk);
      }}
    }});

    tlsSocket.on('error', (err) => cleanupAndExit(file, `[download] Proxy TLS error: ${{err.message}}`));
    tlsSocket.on('end', () => file.close(() => process.exit(0)));
    tlsSocket.setTimeout(timeoutMs, () => tlsSocket.destroy(new Error('proxy tls timeout')));
  }});

  req.on('timeout', () => req.destroy(new Error('proxy connect timeout')));
  req.on('error', (err) => cleanupAndExit(file, `[download] Proxy error: ${{err.message}}`));
  req.end();
}}

console.log('[download] Starting:', sourceUrl);
console.log('[download] Destination:', dest);
if (proxyUrl) console.log('[download] Using proxy:', proxyUrl);

if (proxyUrl && sourceUrl.startsWith('https://')) {{
  proxyRequest(sourceUrl);
}} else {{
  directRequest(sourceUrl, maxRedirects);
}}
"#,
        url, dest_path_str
    );

    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("download_script.js");
    std::fs::write(&script_path, script)
        .map_err(|e| format!("Failed to write download script: {}", e))?;

    let mut process = command(&node_path);
    process.arg(&script_path);
    process.stdout(Stdio::piped());
    process.stderr(Stdio::piped());

    if let Some(proxy) = get_proxy_env() {
        process.env("HTTPS_PROXY", &proxy);
        process.env("HTTP_PROXY", &proxy);
    }

    let mut child = process
        .spawn()
        .map_err(|e| format!("Failed to run download script: {}", e))?;

    wait_for_completion(&mut child, &script_path)?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to collect download output: {}", e))?;

    let _ = std::fs::remove_file(&script_path);

    if output.status.success() {
        Ok(())
    } else {
        Err(format!("Download script failed: {}", error_text(&output)))
    }
}

/// 验证文件大小
pub fn verify_file_size(path: &Path, expected_size: Option<u64>) -> bool {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            if let Some(expected) = expected_size {
                metadata.len() == expected
            } else {
                metadata.len() > 0
            }
        }
        Err(_) => false,
    }
}

/// 带回退与重试的下载
pub fn download_with_fallback(urls: &[&str], dest: &Path) -> Result<(), String> {
    if urls.is_empty() {
        return Err("No URLs provided".to_string());
    }

    let mut errors: Vec<String> = vec![];

    for url in urls {
        for attempt in 1..=3 {
            let _ = std::fs::remove_file(dest);

            match download_file_with_node(url, dest, None::<fn(f64)>) {
                Ok(()) => {
                    if verify_file_size(dest, None) {
                        return Ok(());
                    }
                    errors.push(format!("{} (attempt {}): downloaded file is empty", url, attempt));
                }
                Err(e) => {
                    errors.push(format!("{} (attempt {}): {}", url, attempt, e));
                }
            }

            if attempt < 3 {
                sleep(Duration::from_secs(attempt as u64));
            }
        }
    }

    Err(errors.join("; "))
}

/// 清理临时下载文件
pub fn cleanup_temp_downloads() -> Result<(), String> {
    let temp_dir = std::env::temp_dir();
    let entries =
        std::fs::read_dir(&temp_dir).map_err(|e| format!("Failed to read temp dir: {}", e))?;

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if let Some(name) = path.file_name() {
                let name = name.to_string_lossy();
                if name.starts_with("download_") || name.ends_with("-installer.exe") {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }

    Ok(())
}
