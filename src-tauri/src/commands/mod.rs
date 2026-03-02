/**
 * ---
 * role: Tauri 命令模块聚合，暴露给前端调用的所有命令
 * depends:
 *   - ./detect.rs
 *   - ./installer.rs
 *   - ./git.rs
 *   - ./node.rs
 *   - ./conda.rs
 *   - ./claude.rs
 *   - ./onboarding.rs
 *   - ./ccswitch.rs
 *   - ./workflow.rs
 * exports:
 *   - detect
 *   - installer
 *   - git
 *   - node
 *   - conda
 *   - claude
 *   - onboarding
 *   - ccswitch
 *   - workflow
 * status: PENDING
 * ---
 */

pub mod detect;
pub mod installer;
pub mod git;
pub mod node;
pub mod conda;
pub mod claude;
pub mod onboarding;
pub mod ccswitch;
pub mod workflow;
