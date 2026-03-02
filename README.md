# Claude 环境安装器

跨平台一键式开发环境配置工具，自动安装 Claude Code 所需的全部依赖组件。

## 快速开始

```bash
python install.py
```

## 命令行选项

| 参数 | 说明 |
|------|------|
| `--minimal` | 最小安装，仅安装核心组件 |
| `--with-conda` | 包含 Miniconda 安装 |
| `--with-workflow-kit` | 包含 workflow kit |
| `--force` | 强制重新安装所有组件 |
| `--yes` | 自动确认所有提示 |

## 安装组件

- **Node.js** - Claude Code 运行时依赖
- **Claude Code** - Claude CLI 工具
- **Miniconda** (可选) - Python 环境管理

## 系统要求

- Python 3.9+
- Windows 10+、macOS 12+、Ubuntu 20.04+

## 示例

```bash
# 最小安装
python install.py --minimal

# 完整安装（含 Miniconda）
python install.py --with-conda

# 静默安装
python install.py --yes

# 强制重装
python install.py --force
```

## 构建

```bash
# 构建当前平台
python build.py --platform windows

# 构建所有平台
python build.py --platform all
```

## 项目结构

```
claude-env-installer/
├── install.py              # 入口脚本
├── build.py                # 构建入口
├── requirements.txt        # Python 依赖
├── src/
│   ├── core/               # 核心模块
│   ├── installers/         # 各组件安装器
│   ├── platform/           # 跨平台抽象
│   └── utils/              # 工具模块
├── build/
│   ├── windows/            # Windows 构建
│   ├── macos/              # macOS 构建
│   └── linux/              # Linux 构建
└── tests/                  # 测试代码
```

## 许可证

MIT
