/**
 * ---
 * role: Tauri 命令模块聚合，暴露给前端调用的所有命令
 * depends:
 *   - ./detect.rs
 *   - ./installer.rs
 *   - ./node.rs
 *   - ./git.rs
 *   - ./conda.rs
 *   - ./claude.rs
 *   - ./onboarding.rs
 *   - ./ccswitch.rs
 *   - ./download.rs
 *   - ./process.rs
 * exports:
 *   - detect
 *   - installer
 *   - node
 *   - git
 *   - conda
 *   - claude
 *   - onboarding
 *   - ccswitch
 *   - download
 *   - process
 * status: PENDING
 * ---
 */

pub mod detect;
pub mod installer;
pub mod node;
pub mod git;
pub mod conda;
pub mod claude;
pub mod onboarding;
pub mod ccswitch;
pub mod download;
pub mod process;
