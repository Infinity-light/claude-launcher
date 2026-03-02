# Claude Launcher

Claude Code 环境一键安装器 - Tauri 桌面应用

![App Cover](./src-tauri/assets/app-cover.png)

## 使用方法

直接双击运行，无需安装：

```
Claude-Launcher.exe
```

## 功能

自动完成 7 步环境配置：

1. **Git** - 检测或安装
2. **Node.js** - 安装 LTS 版本
3. **Miniconda** - Python 环境管理
4. **Claude Code** - 官方 CLI 工具
5. **Onboarding** - 配置文件写入
6. **CCSwitch** - 版本管理工具
7. **Workflow Kit** - Claude Code Plugin 工作流套件

## 系统要求

- Windows 10+
- macOS 12+
- Ubuntu 20.04+

## 开发

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri:dev

# 构建
npm run tauri:build
```

## 技术栈

- **后端**: Rust + Tauri v2
- **前端**: Vue 3 + TypeScript + Tailwind CSS
- **状态管理**: Pinia

## 许可证

MIT
