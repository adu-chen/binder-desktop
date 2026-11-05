# Binder

**本地优先的 AI 原生写作工作台。** 文档、记忆、知识库都放在你自己选定的本地目录里；AI 的每次修改都以 diff 提案出现，写不写入由你裁决。

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB)](https://tauri.app)
[![Rust ≥ 1.77](https://img.shields.io/badge/Rust-%E2%89%A5%201.77-000000?logo=rust)](src-tauri/Cargo.toml)
[![Platform](https://img.shields.io/badge/platform-macOS%20arm64-lightgrey)](#运行要求)
[![Status](https://img.shields.io/badge/status-0.1.0%20pre--release-orange)](#当前状态)

## 是什么

Binder 面向**用 AI 协同完成严肃写作与多文件文档构建的个人专业写作者**。它把「AI 生成修改」变成可预览、可裁决的 diff，而不是让模型直接改你的文件。

- **桌面客户端**：Tauri 2 + React + TypeScript（前端）+ Rust（后端）。
- **本地优先**：workspace 是你选定的本地目录，是文件操作、搜索、diff、上下文读取的边界；用户内容不上云。
- **薄云端**：只有账号/认证、订阅/计费、AI 网关；不承载用户内容，也不做内容云同步。

## 核心交互

1. 在代理面板（Agent）里提要求，模型通过工具读文档、搜索、生成修改。
2. 修改以 **diff 提案**返回：已有编辑状态（LogicalState）时在编辑器内直接预览；否则挂载为待裁决卡片。
3. 你显式接受或拒绝；接受后写入文件，保存 pending 预览也等同接受。

此外支持多文件文档项目构建（CBT）、DOCX/Excel/PPT/PDF 与音视频预览、@mention 引用、本地记忆与知识库、模板/工作流。

## 设计原则

| 原则 | 含义 |
|---|---|
| 本地优先 | 内容、记忆、模板、聊天历史、知识库全部本地存储 |
| 人类把关 | AI 只提案，不直接落盘；接受/拒绝/保存都由人触发 |
| 薄云端 | 云端只做认证、计费、模型网关，不碰内容 |
| 规则驱动 | 文档先于代码：需求 → 技术设计 → 计划 → 实现 → 验证，代码用 `@GOV` 标注回链规则 |

## 明确不做（v1）

多人实时协同、内容云同步、插件市场、脱离写作场景的通用聊天助手、Excel/PPT 编辑（仅预览）。

## 模块地图

| 模块 | 职责 |
|---|---|
| CORE | 治理与项目核心 |
| SYS | 整体架构、IPC 协议、跨模块契约 |
| WS | workspace 生命周期、文件 I/O、pending diffs、文档级时间轴 |
| ED | 编辑器运行时（TipTap，Markdown / DOCX 富文本） |
| DE | Diff Edit：diff 生成、审阅、接受/拒绝/过期终态 |
| AG | 代理：回合基建、工具调用、续写与局部修改 |
| CBT | 项目蓝图与多文件文档项目构建 |
| DOC | DOCX/Pandoc 转换与内置工具运行时 |
| PRV | 预览与输出：Office/PDF/音视频预览、打印/导出 |
| REF | 引用（@mention）解析与注入 |
| AST / KB / TPL | 本地记忆协同 / 知识库 / 模板与工作流 |
| ACCT / ENT | 账号认证客户端 / 订阅门控与 AI 网关客户端 |

## 仓库结构

```
src/                    前端（React + TypeScript，TipTap 编辑器）
src-tauri/src/          Rust 后端（Tauri commands、workspace、DOC、AG、DE、PRV…）
src-tauri/resources/    运行期资源：工具 manifest、reference.docx、字体方案（Pandoc/LO 二进制不入库）
cloud/dev-gateway/      本地开发用 AI 网关（临时脚手架，非生产）
docs/                   需求 / 设计 / 计划 / 验证 + 治理索引
scripts/                治理、测试选择与构建脚本
ADU.md ADP.md AGENTS.md CLAUDE.md   治理控制文档
```

## 快速开始

### 运行要求

- **macOS 12+（arm64）**：当前仅支持 macOS（后端依赖 CoreText / Core Foundation，内置资源按 arm64 锁定）
- **Node.js 18+**（前端构建；开发环境使用 v23）
- **Rust 1.77+** 与 Xcode Command Line Tools

### 启动

```bash
npm install
npm run tauri dev        # 桌面应用（前端 1420 端口 + Rust 后端）
# 或者只起前端
npm run dev
```

### 接入 AI

客户端默认把模型请求发往本地网关 `http://localhost:8787`（见 `src-tauri/src/ent/gateway.rs`）：

```bash
cd cloud/dev-gateway
node server.mjs          # 需要 Node ≥ 22.5，以及你自己的 provider key
```

配置方式、端点与支持的 provider 见 [cloud/dev-gateway/README.md](cloud/dev-gateway/README.md)。

### 打包与内置工具

`npm run build` 会先执行 `scripts/verify_bundled_tools.sh`，校验 `src-tauri/resources/tools/manifest.json` 中锁定的 SHA-256：

```bash
PANDOC_ARCHIVE=/path/to/pandoc-3.10-arm64-macOS.zip bash scripts/prepare_bundled_tools.sh
bash scripts/deploy_lo.sh          # 部署精简版 LibreOffice（约 380MB，不入库）
npm run tauri build
```

内置工具不随仓库分发，遵循各自许可证：Pandoc（GPL-2.0-or-later）、LibreOffice（MPL-2.0）。

## 测试与治理

| 命令 | 作用 |
|---|---|
| `npm test` | 前端 Vitest |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust 单元 + 集成测试 |
| `npm run test:full` | 前端 + Rust 全量 |
| `npm run governance:audit` | 只读治理审计（@GOV、规则引用、术语、测试覆盖） |
| `npm run governance:generate` | 重新生成 ADUS 索引（自动生成区，禁止手改） |
| `npm run test:plan -- --files <改动文件>` | 计算受影响测试 |
| `npm run test:related -- --files <改动文件>` | 执行受影响测试 |

改动代码前请先读治理文档：本项目要求文档先行、规则可回链，`governance:audit` 是硬门。

## 文档

| 文档 | 内容 |
|---|---|
| [ADU.md](ADU.md) | 文档、规则、链路、模块、`@GOV`、Issue Trace 的静态规则 |
| [ADP.md](ADP.md) | AI 开发过程的 Gate、Scope Lock、Stop Conditions、运行时报告 |
| [AGENTS.md](AGENTS.md) | AI 会话入口、工作准则、执行路由 |
| [CLAUDE.md](CLAUDE.md) | 同上（Claude 入口版本） |
| [APC](docs/00_core/A-CORE-C-P-01_APC.md) | 项目身份、模块化架构、AI 执行边界 |
| [ADUS](docs/00_core/A-CORE-C-R-01_ADUS.md) | 派生检索索引（按链路 / 规则 / 模块定位代码与测试） |
| [docs/](docs) | 需求（L1）、技术设计（L2）、计划（L3）、验证卡 |

## 当前状态

`0.1.0`，**pre-release**：1.0 Experience Preview 阶段，仅有 macOS arm64 构建，签名与公证尚未完成，接口与数据格式仍可能变化。

## 许可证

代码以 [MIT](LICENSE) 许可。仓库不包含 Pandoc / LibreOffice 二进制，它们按各自许可证分发。
