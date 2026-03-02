#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
validate.py - Claude ENV Installer 契约验证脚本
用于 Planning 阶段检查骨架完整性

用法:
  python .claude/validate.py                    # 完整验证
  python .claude/validate.py --check-planning   # 仅检查骨架存在性
"""

import os
import sys
import re
import argparse
from pathlib import Path

# 项目根目录（本脚本在 .claude/ 下）
ROOT = Path(__file__).parent.parent

# 预期存在的所有契约文件
EXPECTED_FILES = [
    # Rust 后端
    "src-tauri/src/types.rs",
    "src-tauri/src/main.rs",
    "src-tauri/src/lib.rs",
    "src-tauri/src/commands/mod.rs",
    "src-tauri/src/commands/detect.rs",
    "src-tauri/src/commands/installer.rs",
    "src-tauri/src/commands/git.rs",
    "src-tauri/src/commands/node.rs",
    "src-tauri/src/commands/conda.rs",
    "src-tauri/src/commands/claude.rs",
    "src-tauri/src/commands/onboarding.rs",
    "src-tauri/src/commands/ccswitch.rs",
    "src-tauri/src/commands/workflow.rs",
    # Vue 前端
    "src/types.ts",
    "src/main.ts",
    "src/App.vue",
    "src/router/index.ts",
    "src/stores/installer.ts",
    "src/views/WelcomeView.vue",
    "src/views/ProgressView.vue",
    "src/views/CompleteView.vue",
]

# 应该已删除的旧文件（Execution 阶段清理任务）
OBSOLETE_FILES = [
    "src/views/DashboardView.vue",
    "src/views/CCSwitchView.vue",
    "src/views/IDELauncherView.vue",
    "src/views/WorkspacesView.vue",
    "src/views/SettingsView.vue",
    "src-tauri/src/commands/cc_switch.rs",
    "src-tauri/src/commands/cc_switch_app.rs",
    "src-tauri/src/commands/ide.rs",
    "src-tauri/src/commands/workspace.rs",
    "src-tauri/src/commands/system.rs",
]

# 契约关键词（文件中必须包含 status 字段）
CONTRACT_PATTERN = re.compile(r"status:\s*(PENDING|IMPLEMENTED|VERIFIED)", re.IGNORECASE)

# Tauri commands 注册验证（lib.rs 必须包含这些命令）
REQUIRED_TAURI_COMMANDS = [
    "get_system_info",
    "start_installation",
    "open_path",
]


def check_planning_mode():
    """Planning 阶段检查：验证所有契约文件存在且包含 status 字段"""
    errors = []
    warnings = []

    print("=" * 60)
    print("Planning 阶段验证：契约文件完整性检查")
    print("=" * 60)

    # 1. 检查预期文件是否存在
    print("\n[1/3] 检查契约文件存在性...")
    for rel_path in EXPECTED_FILES:
        full_path = ROOT / rel_path
        if full_path.exists():
            content = full_path.read_text(encoding="utf-8", errors="ignore")
            if CONTRACT_PATTERN.search(content):
                print(f"  ✓ {rel_path}")
            else:
                errors.append(f"缺少 status 字段: {rel_path}")
                print(f"  ✗ {rel_path} (缺少 status 字段)")
        else:
            errors.append(f"文件不存在: {rel_path}")
            print(f"  ✗ {rel_path} (文件不存在)")

    # 2. 检查 lib.rs 是否提及必要的 Tauri 命令
    print("\n[2/3] 检查 lib.rs 命令注册...")
    lib_path = ROOT / "src-tauri/src/lib.rs"
    if lib_path.exists():
        lib_content = lib_path.read_text(encoding="utf-8", errors="ignore")
        for cmd in REQUIRED_TAURI_COMMANDS:
            if cmd in lib_content:
                print(f"  ✓ {cmd}")
            else:
                warnings.append(f"lib.rs 中未找到命令: {cmd}")
                print(f"  ⚠ {cmd} (Execution 阶段需在 lib.rs 注册)")
    else:
        errors.append("lib.rs 不存在")

    # 3. 检查旧文件是否还存在（提醒清理）
    print("\n[3/3] 检查旧文件清理状态...")
    for rel_path in OBSOLETE_FILES:
        full_path = ROOT / rel_path
        if full_path.exists():
            warnings.append(f"旧文件尚未清理: {rel_path}")
            print(f"  ⚠ {rel_path} (Execution 阶段需要删除)")
        else:
            print(f"  ✓ {rel_path} (已清理)")

    # 汇总
    print("\n" + "=" * 60)
    if errors:
        print(f"✗ 验证失败：{len(errors)} 个错误")
        for e in errors:
            print(f"  ERROR: {e}")
        if warnings:
            print(f"\n⚠ {len(warnings)} 个警告（不阻断 Execution）")
            for w in warnings:
                print(f"  WARN: {w}")
        sys.exit(1)
    else:
        print(f"✓ 验证通过！所有 {len(EXPECTED_FILES)} 个契约文件存在")
        if warnings:
            print(f"\n⚠ {len(warnings)} 个警告（Execution 阶段需处理）：")
            for w in warnings:
                print(f"  WARN: {w}")
        sys.exit(0)


def check_full():
    """完整验证"""
    check_planning_mode()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Claude ENV Installer 契约验证")
    parser.add_argument("--check-planning", action="store_true", help="仅检查 Planning 骨架完整性")
    args = parser.parse_args()

    if args.check_planning:
        check_planning_mode()
    else:
        check_full()
