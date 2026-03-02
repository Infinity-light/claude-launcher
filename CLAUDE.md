# Claude 环境傻瓜式安装程序

## 核心原则

**透彻到底，不做假设**：无论做任何工作，都不要做任何的假设、mock 和 todo，一定要透彻到底的进行探测和执行。一旦觉得一个工作太过于冗长，就将其拆分成更小的任务，设定好交付标准，并在最后用交付标准逐项验证。

## 项目概述

跨平台一键式 Claude Code 开发环境配置工具，打包为 Tauri 桌面应用（EXE/App/AppImage），帮助用户在全新电脑上完成全部环境配置。

## 技术栈

| 层级 | 技术 |
|------|------|
| 后端 | Rust + Tauri v2 |
| 前端 | Vue 3 + TypeScript + Vite |
| 状态管理 | Pinia |
| 打包 | Tauri Bundle |

## 安装步骤（固定顺序，7步）

| # | 组件 | 说明 |
|---|------|------|
| 0 | Git | 自动检测，缺失则安装 |
| 1 | Node.js | LTS 版本 |
| 2 | Miniconda | Python 环境管理 |
| 3 | Claude Code | 官方脚本优先，npm 回退 |
| 4 | Onboarding 配置 | 写入 ~/.claude.json |
| 5 | CCSwitch | github.com/farion1231/cc-switch |
| 6 | Workflow Kit | github.com/Infinity-light/claude-workflow-kit → ~/.claude/ |

## 目录结构（当前版本）

```
claude-env-installer/
├── src/                        # Vue 3 前端
│   ├── types.ts                # 共享类型定义
│   ├── main.ts                 # 入口
│   ├── App.vue                 # 根组件
│   ├── router/index.ts         # 路由（/ → /progress → /complete）
│   ├── stores/installer.ts     # Pinia 安装状态
│   └── views/
│       ├── WelcomeView.vue     # 欢迎页：步骤列表 + 开始按钮
│       ├── ProgressView.vue    # 进度页：实时步骤状态
│       └── CompleteView.vue    # 完成页：结果总结
│
├── src-tauri/                  # Rust 后端
│   └── src/
│       ├── types.rs            # 共享类型（SystemInfo, StepUpdate 等）
│       ├── main.rs             # 入口（调用 lib::run()）
│       ├── lib.rs              # Tauri 应用构建器 + 命令注册
│       └── commands/
│           ├── mod.rs          # 模块聚合
│           ├── detect.rs       # get_system_info()
│           ├── installer.rs    # start_installation()（编排 0-6 步）
│           ├── git.rs          # ensure_git()
│           ├── node.rs         # ensure_node()
│           ├── conda.rs        # ensure_conda()
│           ├── claude.rs       # ensure_claude()
│           ├── onboarding.rs   # configure_onboarding()
│           ├── ccswitch.rs     # download_ccswitch()
│           └── workflow.rs     # install_workflow_kit()
│
├── tests/
│   └── test_integration.py     # 集成测试骨架
│
└── .claude/
    ├── validate.py             # 契约验证脚本
    └── discovery/
        └── Claude 安装器 PRD.md

```

## Tauri 命令接口

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_system_info` | - | `SystemInfo` | 检测 OS 和架构 |
| `start_installation` | `os, arch` | `InstallationResult` | 执行 7 步安装，发射事件 |
| `open_path` | `path: String` | - | 打开文件/目录 |

## Tauri 事件（后端 → 前端）

| 事件 | 数据 | 说明 |
|------|------|------|
| `step-update` | `StepUpdate` | 每步状态变更时发射 |
| `installation-complete` | `InstallationResult` | 全部完成时发射 |

## 开发阶段

- [x] Discovery: 需求调研（第四轮修正）
- [x] Planning: 架构重新设计（当前完成）
- [ ] **Execution: 实现（下一步）**
  - [ ] 清理旧文件（10 个 OBSOLETE 文件）
  - [ ] 实现 Rust 类型定义（types.rs）
  - [ ] 实现 Rust 命令（9 个模块）
  - [ ] 实现 Vue 前端（3 个页面 + store + router）
  - [ ] Cargo.toml 依赖补充（reqwest/ureq, serde_json）
- [ ] Verification: 打包验证

## 旧文件清理清单（Execution 开始时删除）

- `src/views/DashboardView.vue`
- `src/views/CCSwitchView.vue`
- `src/views/IDELauncherView.vue`
- `src/views/WorkspacesView.vue`
- `src/views/SettingsView.vue`
- `src-tauri/src/commands/cc_switch.rs`
- `src-tauri/src/commands/cc_switch_app.rs`
- `src-tauri/src/commands/ide.rs`
- `src-tauri/src/commands/workspace.rs`
- `src-tauri/src/commands/system.rs`
- `src-tauri/src/core/`（整个目录）
- `src-tauri/src/plugins/`（整个目录）

## 验证命令

```bash
# Planning 验证
python .claude/validate.py --check-planning

# 测试骨架
python -m pytest tests/test_integration.py -v --import-mode=importlib

# 开发模式
npm run tauri:dev

# 构建
npm run tauri:build
```

## 许可证

MIT
