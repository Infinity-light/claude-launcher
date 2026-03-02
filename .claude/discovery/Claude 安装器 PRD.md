# PRD: Claude 环境傻瓜式安装器（最终版）

## 一、需求理解（为什么）

### 目标
一个打包成 EXE 的跨平台安装器，帮助用户在全新电脑（啥都没有的情况下）一键完成 Claude Code 开发环境的全部配置，无需任何技术背景。

### 完整安装步骤（顺序不可变）

| 步骤 | 组件 | 说明 |
|------|------|------|
| 1 | **Git** | 自动检测，缺失则安装（Windows: Git for Windows） |
| 2 | **Node.js** | 安装 LTS 版本（npm 作为 Claude Code 回退必需） |
| 3 | **Miniconda** | Python 环境管理（conda + python） |
| 4 | **Claude Code** | 官方脚本优先，npm 回退 |
| 5 | **Onboarding 配置** | 写入 ~/.claude.json，免登录验证 |
| 6 | **CCSwitch** | 根据 OS 下载对应版本，引导使用 |
| 7 | **Workflow Kit** | git clone 到 ~/.claude/ |

### 分发形态
- **打包方式**：Tauri 桌面应用 → Windows EXE / macOS App / Linux AppImage
- **目标用户**：没有任何开发环境的普通用户
- **运行方式**：双击运行，无需命令行

### 边界条件
- **包含**：以上 7 步 + Tauri GUI
- **不包含**：IDE 检测/启动、CUDA、复杂工作空间管理、云同步

---

## 二、调研结论（是什么）

### 2.1 Git 安装

| 平台 | 安装方式 | 来源 |
|------|---------|------|
| Windows | `winget install Git.Git` 或下载 `Git-*.exe` | git-scm.com |
| macOS | `xcode-select --install` (内置 git) 或 Homebrew | 系统自带 |
| Linux | `apt install git` / `yum install git` | 包管理器 |

检测命令：`git --version`

---

### 2.2 Node.js 安装

**官方推荐方式（按平台）**：

| 平台 | 方式 |
|------|------|
| Windows | winget: `winget install OpenJS.NodeJS.LTS` 或下载 MSI |
| macOS | `brew install node@lts` 或下载 PKG |
| Linux | NodeSource 脚本: `curl -fsSL https://deb.nodesource.com/setup_lts.x | bash` |

检测命令：`node --version` 和 `npm --version`

---

### 2.3 Miniconda 安装

**下载链接（按平台）**：

| 平台 | 安装包 | 安装命令 |
|------|--------|---------|
| Windows | `Miniconda3-latest-Windows-x86_64.exe` | 静默: `/S /D=C:\miniconda3` |
| macOS (Intel) | `Miniconda3-latest-MacOSX-x86_64.sh` | `bash installer.sh -b -p ~/miniconda3` |
| macOS (ARM) | `Miniconda3-latest-MacOSX-arm64.sh` | 同上 |
| Linux x64 | `Miniconda3-latest-Linux-x86_64.sh` | `bash installer.sh -b -p ~/miniconda3` |

**下载源**：`https://repo.anaconda.com/miniconda/`

检测：`conda --version`

---

### 2.4 Claude Code 安装

**官方方式（优先）**：
- macOS/Linux: `curl -fsSL https://claude.ai/install.sh | bash`
- Windows PowerShell: `irm https://claude.ai/install.ps1 | iex`

**npm 回退（官方已废弃，但作 fallback）**：
```bash
npm install -g @anthropic-ai/claude-code
```

检测：`claude --version`

---

### 2.5 Onboarding 配置

**文件**：`~/.claude.json`（用户 home 目录）

**内容**：
```json
{
  "hasCompletedOnboarding": true,
  "hasTrustDialogAccepted": true
}
```

**逻辑**：读取现有文件 → 合并字段 → 写回（幂等操作）

---

### 2.6 CCSwitch

**仓库**：`https://github.com/farion1231/cc-switch`
**版本获取**：`GET https://api.github.com/repos/farion1231/cc-switch/releases/latest`

**按 OS 下载**：

| 平台 | 文件名模式 |
|------|-----------|
| Windows | `CC-Switch-v{version}-Windows.msi` |
| macOS | `CC-Switch-v{version}-macOS.zip` |
| Linux (deb) | `CC-Switch-v{version}-Linux-x86_64.deb` |
| Linux (rpm) | `CC-Switch-v{version}-Linux-x86_64.rpm` |

**下载位置**：用户 Desktop 或 Downloads 目录

**引导语**：
- 官方源用户：可选使用，用于切换 API endpoint
- 镜像源用户：**推荐**，一键切换回官方源

---

### 2.7 Workflow Kit

**仓库**：`https://github.com/Infinity-light/claude-workflow-kit`

**安装策略**（内容直接铺到 `~/.claude/`，不是子目录）：

```bash
# Windows PowerShell
git clone --depth 1 https://github.com/Infinity-light/claude-workflow-kit.git "$env:TEMP\cwk"
Copy-Item -Recurse -Force "$env:TEMP\cwk\*" "$env:USERPROFILE\.claude\"
Remove-Item -Recurse -Force "$env:TEMP\cwk"

# macOS/Linux
git clone --depth 1 https://github.com/Infinity-light/claude-workflow-kit.git /tmp/cwk
cp -r /tmp/cwk/. ~/.claude/
rm -rf /tmp/cwk
```

---

## 三、实现方案（怎么办）

### 3.1 用户动线

```
用户双击 EXE / App / AppImage
    ↓
欢迎页
  ├─ 显示将要安装的组件列表
  ├─ 检测 OS 和架构（自动）
  └─ "开始安装" 按钮
    ↓
安装进度页（逐步显示）
  ├─ [1/7] Git          [进度条] → ✓ 已安装 / 已跳过
  ├─ [2/7] Node.js      [进度条] → ✓ / ✗
  ├─ [3/7] Miniconda    [进度条] → ✓ / ✗
  ├─ [4/7] Claude Code  [进度条] → ✓ 官方安装 / ✓ npm 安装 / ✗
  ├─ [5/7] 配置跳过引导   [快速]   → ✓
  ├─ [6/7] CCSwitch 下载 [进度条] → ✓ 已下载到 Desktop
  └─ [7/7] Workflow Kit  [进度条] → ✓ / ✗
    ↓
完成页
  ├─ 总结：成功/失败数量
  ├─ 提示：运行 `claude` 开始使用
  ├─ CCSwitch 使用说明（如镜像源 → 推荐使用）
  └─ "完成" 按钮
```

### 3.2 系统架构（Tauri + Vue 3 + Rust）

```
claude-env-installer/
├── src/                        # Vue 3 前端
│   ├── App.vue
│   ├── main.ts
│   ├── views/
│   │   ├── WelcomeView.vue     # 欢迎页：组件列表 + 开始按钮
│   │   ├── ProgressView.vue    # 安装进度：步骤列表 + 日志输出
│   │   └── CompleteView.vue    # 完成页：结果总结 + 使用说明
│   ├── router/index.ts
│   └── stores/installer.ts     # Pinia：安装状态管理
│
├── src-tauri/                  # Rust 后端
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       └── commands/
│           ├── mod.rs
│           ├── detect.rs       # OS/架构检测
│           ├── git.rs          # Git 安装
│           ├── node.rs         # Node.js 安装
│           ├── conda.rs        # Miniconda 安装
│           ├── claude.rs       # Claude Code 安装
│           ├── onboarding.rs   # ~/.claude.json 写入
│           ├── ccswitch.rs     # CCSwitch 下载
│           └── workflow.rs     # Workflow Kit 克隆
│
├── vite.config.ts
├── package.json
└── requirements.txt（已弃用，全部迁到 Rust）
```

### 3.3 Tauri 事件通信设计

```
前端 (Vue)  ←────────────── 后端 (Rust)
                events:
                  install:progress  { step, status, message }
                  install:log       { level, text }
                  install:done      { results[] }

invoke commands:
  start_installation()
  get_system_info() → { os, arch, ... }
```

### 3.4 已有代码的处置

**保留并重写**（现有实现不符合需求）：
- `src-tauri/` 骨架 → 实现 Rust 命令
- `src/views/` → 重写为 3 个页面
- `src/stores/installer.ts` → 重写状态管理

**删除**（超出范围）：
- `src/views/CCSwitchView.vue`（功能不对，CCSwitch 只是下载引导）
- `src/views/IDELauncherView.vue`
- `src/views/WorkspacesView.vue`
- `src/views/SettingsView.vue`
- `src/installers/` 下的 Python 文件（全部迁到 Rust）
- `src/core/orchestrator.py`（过度设计）
- `src/ide/`、`src/gui/`（超出范围）
- `src/installers/miniconda.py`（Python 版本替换为 Rust）

---

## 四、验收标准

- [ ] Windows 全新机器：双击 EXE，7步全绿，`claude --version` 可用
- [ ] macOS 全新机器：双击 App，7步全绿，`claude --version` 可用
- [ ] Linux 全新机器：双击 AppImage，7步全绿，`claude --version` 可用
- [ ] `~/.claude.json` 包含 `hasCompletedOnboarding: true`
- [ ] CCSwitch 可执行文件出现在用户 Desktop 或 Downloads
- [ ] `~/.claude/skills/` 存在 Workflow Kit 的 skills 文件
- [ ] 已有组件检测正确（跳过已装，不重装）
- [ ] 任一步骤失败时，后续步骤继续尝试，失败项在完成页明确标出
