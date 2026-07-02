---
文档编号：CORE-GOV-01
文档状态：A
负责模块：CORE
文档职责：ADU 派生检索索引
上游约束：CORE-GOV-00
直接承接：ADU / ADP / CLAUDE 的规则、链路、代码块检索
使用边界：不定义治理规则，不定义开发过程，不替代 adu.md、ADP.md 或 CLAUDE.md
变更要求：运行 npm run governance:generate 刷新自动生成区
---

# ADU 派生检索索引

本文件由 `governance:generate` 自动生成。
所有索引内容来自设计文档标注块、源代码 @GOV 标注、测试覆盖注释和 triage 报告。
运行 `npm run governance:generate` 刷新。

边界：
- adu.md 是静态治理规则源。
- ADP.md 是开发过程运行时协议。
- CLAUDE.md 是 AI 会话入口与执行路由协议。
- adus.md 只提供派生检索索引，不定义规则、不定义过程、不承载人类裁决。

自动生成边界：
- `## 【自动生成区】` 之前仅保留元信息头部，包括文档头、边界说明和变更记录。
- 元信息头部不得写入规则、链路、模块、约束、裁决或索引内容。
- `## 【自动生成区】` 及之后内容禁止人工编辑，应由 `npm run governance:generate` 刷新。

---

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-09 | v1.0 | 初始版本 |

---

## 【自动生成区】

<!-- 自动生成。禁止人工编辑。-->
<!-- 运行：npm run governance:generate -->
<!-- 最后生成时间：2026-07-02T16:20:42.164Z -->

### 模块编码表

| module_code | module_name | description                                            | 来源文档                                       |
| ----------- | ----------- | ------------------------------------------------------ | ------------------------------------------ |
| CORE        | 核心          | 项目核心与治理，跨模块产品级定义                                       | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| SYS         | 系统底座        | 整体架构、IPC 协议、跨模块契约与模块编码基线                               | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| WS          | 工作台         | workspace 生命周期、文件 I/O、缓存、pending_diffs 持久化、文档级时间轴与历史回溯 | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| ED          | 编辑器         | 编辑器运行时（TipTap，md/docx 富文本编辑）                           | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| DE          | 对话编辑        | diff 生成、审阅、接受/拒绝/过期终态                                  | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| AG          | AI Agent    | 单文档 chat/agent 与工具调用回合                                 | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| CBT         | 文档构建        | agent 文档项目构建（Chat Build）、构建中断、Chat↔Build 接管            | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| DOC         | 文档转换        | DOCX/Pandoc 格式转换                                       | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| PRV         | 预览输出        | Excel/PPT 预览、音视频预览与打印/导出（不含 Excel/PPT 编辑）              | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| REF         | 引用          | 引用（@mention）解析与上下文注入                                   | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| AST         | 记忆          | 本地记忆协同                                                 | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| KB          | 知识库         | 本地知识库                                                  | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| TPL         | 模板          | 模板与工作流                                                 | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| ACCT        | 账号          | 账号、认证、会话（云端认证客户端）                                      | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |
| ENT         | 订阅门控        | 订阅门控、用量与 AI 网关客户端                                      | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md |

### 规则域注册表

| domain_code | description      | 来源 |
| ----------- | ---------------- | -- |
| STATE       | 状态语义、生命周期、阶段闭合   | 内置 |
| DATA        | 数据结构、字段定义、schema | 内置 |
| PERSIST     | 持久化、恢复、重建        | 内置 |
| VERIFY      | 验证、审计、一致性检查      | 内置 |
| GOV         | 文档治理、命名、变更控制     | 内置 |
| OBS         | 可观测性、错误处理、恢复     | 内置 |

### 链路注册表

| chain_id      | 主责模块 | status | 集成测试 |
| ------------- | ---- | ------ | ---- |
| AG-TURN       | AG   | active |      |
| AG-TOOL       | AG   | active |      |
| AG-ASSIST     | AG   | active |      |
| AST-MEMORY    | AST  | active |      |
| CBT-BUILD     | CBT  | active |      |
| DE-DIFF       | DE   | active |      |
| DOC-CONVERT   | DOC  | active |      |
| ED-EDIT       | ED   | active |      |
| ED-DECOR      | ED   | active |      |
| KB-RETRIEVE   | KB   | active |      |
| PRV-VIEW      | PRV  | active |      |
| REF-INJECT    | REF  | active |      |
| DATA-MIGRATE  | SYS  | active |      |
| IPC-CALL      | SYS  | active |      |
| CLOUD-AUTH    | SYS  | active |      |
| CLOUD-GATE    | SYS  | active |      |
| CLOUD-INVOKE  | SYS  | active |      |
| TPL-FLOW      | TPL  | active |      |
| WS-FS         | WS   | active |      |
| WS-DIFF-APPLY | WS   | active |      |
| WS-TIMELINE   | WS   | active |      |
| WS-SAVE       | WS   | active |      |

### 规则注册表

| rule_id            | 域       | 主链路                                  | 来源文档                                           | registry_status | 测试覆盖    |
| ------------------ | ------- | ------------------------------------ | ---------------------------------------------- | --------------- | ------- |
| BR-ACCT-STATE-001  | STATE   | CLOUD-AUTH                           | docs/01_modules/ACCT/A-ACCT-M-T-01_账号会话技术设计.md | registered      | MISSING |
| BR-ACCT-DATA-001   | DATA    | CLOUD-AUTH                           | docs/01_modules/ACCT/A-ACCT-M-T-01_账号会话技术设计.md | registered      | MISSING |
| BR-AG-STATE-001    | STATE   | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md     | registered      | MISSING |
| BR-AG-DATA-003     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md     | registered      | MISSING |
| BR-AG-VERIFY-001   | VERIFY  | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md     | registered      | MISSING |
| BR-AG-PERSIST-001  | PERSIST | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md     | registered      | MISSING |
| BR-AG-STATE-002    | STATE   | AG-TOOL                              | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md     | registered      | MISSING |
| BR-AG-DATA-001     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md     | registered      | MISSING |
| BR-AG-DATA-002     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md     | registered      | MISSING |
| BR-AG-STATE-004    | STATE   | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md     | registered      | MISSING |
| BR-AG-STATE-003    | STATE   | AG-ASSIST                            | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md     | registered      | MISSING |
| BR-AG-DATA-004     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md      | registered      | MISSING |
| BR-AG-DATA-005     | DATA    | AG-TURN                              | docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md      | registered      | MISSING |
| BR-AST-DATA-001    | DATA    | AST-MEMORY                           | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md   | registered      | MISSING |
| BR-AST-STATE-001   | STATE   | AST-MEMORY                           | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md   | registered      | MISSING |
| BR-AST-DATA-002    | DATA    | AST-MEMORY                           | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md   | registered      | MISSING |
| BR-CBT-STATE-001   | STATE   | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md     | registered      | MISSING |
| BR-CBT-DATA-001    | DATA    | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md     | registered      | MISSING |
| BR-CBT-PERSIST-001 | PERSIST | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md     | registered      | MISSING |
| BR-CBT-STATE-002   | STATE   | CBT-BUILD                            | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md     | registered      | MISSING |
| BR-DE-DATA-001     | DATA    | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_对话编辑技术设计.md     | registered      | MISSING |
| BR-DE-DATA-002     | DATA    | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_对话编辑技术设计.md     | registered      | MISSING |
| BR-DE-STATE-001    | STATE   | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_对话编辑技术设计.md     | registered      | MISSING |
| BR-DE-STATE-002    | STATE   | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_对话编辑技术设计.md     | registered      | MISSING |
| BR-DE-DATA-003     | DATA    | DE-DIFF                              | docs/01_modules/DE/A-DE-M-T-01_对话编辑技术设计.md     | registered      | MISSING |
| BR-DOC-DATA-001    | DATA    | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md   | registered      | MISSING |
| BR-DOC-STATE-001   | STATE   | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md   | registered      | MISSING |
| BR-DOC-DATA-002    | DATA    | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md   | registered      | MISSING |
| BR-DOC-DATA-003    | DATA    | DOC-CONVERT                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md   | registered      | MISSING |
| BR-ED-STATE-001    | STATE   | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md      | registered      | MISSING |
| BR-ED-STATE-003    | STATE   | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md      | registered      | MISSING |
| BR-ED-DATA-001     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md      | registered      | MISSING |
| BR-ED-DATA-002     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md      | registered      | MISSING |
| BR-ED-DATA-003     | DATA    | ED-EDIT                              | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md      | registered      | MISSING |
| BR-ED-STATE-002    | STATE   | ED-DECOR                             | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md      | registered      | MISSING |
| BR-ENT-VERIFY-001  | VERIFY  | CLOUD-GATE                           | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md   | registered      | MISSING |
| BR-ENT-STATE-001   | STATE   | CLOUD-INVOKE                         | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md   | registered      | MISSING |
| BR-ENT-DATA-001    | DATA    | CLOUD-INVOKE                         | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md   | registered      | MISSING |
| BR-ENT-DATA-002    | DATA    | CLOUD-GATE                           | docs/01_modules/ENT/A-ENT-M-T-01_订阅门控技术设计.md   | registered      | MISSING |
| BR-KB-DATA-001     | DATA    | KB-RETRIEVE                          | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md      | registered      | MISSING |
| BR-KB-DATA-002     | DATA    | KB-RETRIEVE                          | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md      | registered      | MISSING |
| BR-PRV-DATA-002    | DATA    | PRV-VIEW                             | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md   | registered      | MISSING |
| BR-PRV-DATA-001    | DATA    | PRV-VIEW                             | docs/01_modules/PRV/A-PRV-M-T-01_预览输出技术设计.md   | registered      | MISSING |
| BR-REF-DATA-001    | DATA    | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md     | registered      | MISSING |
| BR-REF-DATA-002    | DATA    | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md     | registered      | MISSING |
| BR-REF-VERIFY-001  | VERIFY  | REF-INJECT                           | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md     | registered      | MISSING |
| BR-SYS-DATA-002    | DATA    | DATA-MIGRATE                         | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md     | registered      | MISSING |
| BR-SYS-PERSIST-001 | PERSIST | DATA-MIGRATE                         | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md     | registered      | MISSING |
| BR-SYS-DATA-003    | DATA    | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md      | registered      | MISSING |
| BR-SYS-OBS-002     | OBS     | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md      | registered      | MISSING |
| BR-SYS-STATE-002   | STATE   | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md      | registered      | MISSING |
| BR-SYS-STATE-003   | STATE   | IPC-CALL                             | docs/01_modules/SYS/A-SYS-I-P-02_IPC协议.md      | registered      | MISSING |
| BR-SYS-STATE-001   | STATE   | CLOUD-AUTH                           | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md      | registered      | MISSING |
| BR-SYS-VERIFY-001  | VERIFY  | CLOUD-GATE                           | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md      | registered      | MISSING |
| BR-SYS-DATA-001    | DATA    | CLOUD-INVOKE                         | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md      | registered      | MISSING |
| BR-SYS-DATA-004    | DATA    | CLOUD-INVOKE                         | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md      | registered      | MISSING |
| BR-SYS-OBS-001     | OBS     | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md      | registered      | MISSING |
| BR-TPL-DATA-001    | DATA    | TPL-FLOW                             | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md  | registered      | MISSING |
| BR-TPL-STATE-001   | STATE   | TPL-FLOW                             | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md  | registered      | MISSING |
| BR-WS-DATA-001     | DATA    | WS-FS                                | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md      | registered      | MISSING |
| BR-WS-OBS-001      | OBS     | WS-FS                                | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md      | registered      | MISSING |
| BR-WS-STATE-001    | STATE   | WS-DIFF-APPLY                        | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md      | registered      | MISSING |
| BR-WS-DATA-002     | DATA    | WS-TIMELINE                          | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md      | registered      | MISSING |
| BR-WS-STATE-002    | STATE   | WS-TIMELINE                          | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md      | registered      | MISSING |
| BR-WS-PERSIST-002  | PERSIST | DATA-MIGRATE                         | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md      | registered      | MISSING |
| BR-WS-PERSIST-001  | PERSIST | WS-SAVE                              | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md      | registered      | MISSING |

### 跨模块约束表（X-INDEX）

| constraint_id | 摘要                                                                                              | 涉及模块                                                             | 链路影响                                                 | 来源文档                                            |
| ------------- | ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------- | ----------------------------------------------- |
| X-FORBID-006  | AG 及其工具调用不得绕过 DE diff 路径直接写入 workspace 文件；AI 修改落盘唯一路径为 AG 产出→DE 裁决→WS 落盘                        | AG, DE, WS, CBT                                                  | AG-TURN, AG-TOOL, WS-DIFF-APPLY                      | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md      |
| X-SEQ-004     | docx 等二进制文档格式的解析/生成必须经 DOC 转换管线，任何模块不得内置二进制文档转换库或另立转换路径                                         | DOC, ED, WS, PRV, CBT, AG                                        | DOC-CONVERT, ED-EDIT                                 | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md    |
| X-FORBID-005  | 上层模块（DE/AG/REF 等）不得绕过 ED 装饰层与编辑接口直接操纵编辑器内部文档状态；编辑器文档模型的修改必经 ED 受控接口                             | ED, DE, AG, REF                                                  | ED-EDIT, ED-DECOR                                    | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md       |
| X-SEQ-002     | 前端不得跨层直连文件系统/数据库/网络，必经 Service → Tauri command → Rust 后端                                        | SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE                 | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md      |
| X-FORBID-003  | 模块依赖必须无环；SYS 为架构底座不得反向依赖任何业务模块                                                                  | SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE                 | docs/01_modules/SYS/A-SYS-C-T-01_系统总体架构.md      |
| X-FORBID-007  | 前端 Store 不得另立权威业务状态，须投影后端状态机（SM-*）与控制态（workspace.db）；界面流转由后端状态决定，前端只呈现与转达意图，不得本地维护与后端可漂移的并行业务状态 | SYS, WS, ED, DE, AG, CBT, DOC, AST, KB, TPL, REF, PRV, ACCT, ENT | ED-EDIT, AG-TURN, DE-DIFF, WS-FS, WS-SAVE, CBT-BUILD | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md |
| X-SHARED-001  | 跨模块传递的实体/类型/枚举集合必须经 SYS 数据结构契约层（本文）定义，模块不得私自定义跨模块共享结构                                           | SYS, WS, ED, DE, AG, CBT, DOC, PRV, REF, AST, KB, TPL, ACCT, ENT | DATA-MIGRATE                                         | docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md      |
| X-FORBID-001  | 客户端不得持有或落地模型 provider 密钥，密钥仅服务端（AiGateway）持有                                                    | SYS, ENT, ACCT, AG, CBT, AST, KB, TPL                            | CLOUD-INVOKE, CLOUD-GATE                             | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| X-SEQ-001     | 任何模型调用前必须先过订阅门禁硬门（CLOUD-GATE），无有效订阅一律拒绝，禁止跳过门禁直连网关                                              | ENT, AG, CBT, AST, KB, TPL                                       | CLOUD-GATE, CLOUD-INVOKE                             | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| X-FORBID-002  | 订阅门禁不得存在客户端绕过路径，裁决以服务端为准，客户端不得本地缓存通过结论跳过校验                                                      | ENT, SYS                                                         | CLOUD-GATE                                           | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| X-SEQ-003     | 状态化核心链（APC §6 强制阈值）必须先建状态机并在总图登记 machine_id 方可进入实现，未注册者阻断进入代码                                   | SYS, WS, ED, DE, AG, CBT, DOC, AST                               | CLOUD-AUTH, CLOUD-GATE, CLOUD-INVOKE                 | docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md       |
| X-INTER-001   | 跨模块状态转移必须在总图登记转移契约，转移两侧 owner 状态机对接点须一致，不得各自假设状态/事件                                             | SYS, WS, DE, AG, CBT                                             | CLOUD-GATE, CLOUD-INVOKE                             | docs/01_modules/SYS/A-SYS-M-S-01_状态机总图.md       |
| X-FORBID-004  | 文件操作/搜索/diff/上下文读取的目标路径不得越出活动 workspace root；任一越界路径一律拒绝，不静默截断或回退到 workspace 外                   | WS, ED, DE, AG, CBT, REF, AST, KB, TPL, DOC, PRV                 | WS-FS, WS-SAVE, WS-DIFF-APPLY, WS-TIMELINE           | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md       |

### 术语注册表

| term_id        | 所属链路         | 正式中文名  | 正式英文名            | 禁用别名                                              | 来源文档                                            |
| -------------- | ------------ | ------ | ---------------- | ------------------------------------------------- | ----------------------------------------------- |
| TERM-AG-001    | AG-TURN      | 代理回合   | AgentTurn        | 对话轮, 轮次, 会话轮, round, 聊天回合                         | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md      |
| TERM-AG-002    | AG-TOOL      | 工具调用   | ToolCall         | 函数调用, function call, 工具执行, 能力调用                   | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md      |
| TERM-AG-003    | AG-ASSIST    | 内联辅助   | InlineAssist     | 行内辅助, 内联建议, inline建议, 行内AI                        | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md      |
| TERM-AG-004    | AG-ASSIST    | 自动补全   | Autocomplete     | 自动完成, 补全建议, 智能补全, 续写补全                            | docs/01_modules/AG/A-AG-M-T-01_对话代理技术设计.md      |
| TERM-AG-005    | AG-TURN      | 意图沿用判定 | IntentCarryover  | 历史复用, 意图继承, 上下文继承, 意图沿袭, carryover                | docs/01_modules/AG/A-AG-M-T-02_提示词架构设计.md       |
| TERM-AST-001   | AST-MEMORY   | 记忆条目   | MemoryEntry      | 记忆点, 记忆项, 记忆片段, memory条目                          | docs/01_modules/AST/A-AST-M-T-01_记忆协同技术设计.md    |
| TERM-CBT-001   | CBT-BUILD    | 构建回合   | BuildRun         | 构建会话, 构建过程, build流程, 项目构建轮                        | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md      |
| TERM-CBT-002   | CBT-BUILD    | 构建任务   | BuildTask        | 构建步骤, 子任务, build单元, 任务项                           | docs/01_modules/CBT/A-CBT-M-T-01_构建技术设计.md      |
| TERM-DE-001    | DE-DIFF      | 待裁决修改  | PendingDiff      | 修改提案, 变更请求, 待定diff, 修改单, 待审修改                     | docs/01_modules/DE/A-DE-M-T-01_对话编辑技术设计.md      |
| TERM-DE-002    | DE-DIFF      | 修改建议   | ChangeProposal   | AI建议, 修改意见, 编辑建议, 产出建议                            | docs/01_modules/DE/A-DE-M-T-01_对话编辑技术设计.md      |
| TERM-DE-004    | DE-DIFF      | 修改载荷   | DiffPayload      | diff载荷, 内容载荷, payload, diff内容, 落盘载荷, 提案载荷         | docs/01_modules/DE/A-DE-M-T-01_对话编辑技术设计.md      |
| TERM-DOC-001   | DOC-CONVERT  | 样式子集   | StyleSubset      | 样式白名单, 格式子集, 样式范围, 支持样式表                          | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md    |
| TERM-DOC-002   | DOC-CONVERT  | 编辑草稿   | EditDraft        | 草稿副本, draft文件, 临时编辑文件, 编辑副本                       | docs/01_modules/DOC/A-DOC-M-T-01_文档转换技术设计.md    |
| TERM-ED-001    | ED-EDIT      | 内容块    | Block            | 内容节点, 段落块, 文本块, 块节点, chunk                        | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md       |
| TERM-ED-002    | ED-DECOR     | 编辑器装饰层 | EditorDecoration | 高亮层, 标注层, 覆盖层, overlay层, 装饰器层                     | docs/01_modules/ED/A-ED-M-T-01_编辑器技术设计.md       |
| TERM-KB-001    | KB-RETRIEVE  | 知识条目   | KnowledgeEntry   | 知识点, 资料条目, 知识片段, kb条目                             | docs/01_modules/KB/A-KB-M-T-01_知识库技术设计.md       |
| TERM-REF-001   | REF-INJECT   | 引用锚    | ReferenceAnchor  | 引用链接, 引用指针, mention锚, 引用定位符                       | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md      |
| TERM-REF-002   | REF-INJECT   | 注入载荷   | InjectionPayload | 上下文载荷, 注入内容包, 引用载荷, 引用内容体                         | docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md      |
| TERM-SYS-001   | IPC-CALL     | 设计令牌   | DesignToken      | CSS变量, 设计变量, 视觉令牌, token系统, 视觉常量, design token    | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md |
| TERM-DE-003    | DE-DIFF      | 修改卡    | DiffCard         | diff卡片, AI建议卡, 修改面板, 修改组件, 变更卡, 审阅卡               | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md |
| TERM-ED-003    | ED-DECOR     | 绿色审阅层  | GreenReview      | 绿色高亮, 编辑器diff层, diff装饰, 正文高亮层, 修改高亮, green review | docs/01_modules/SYS/A-SYS-C-T-02_前端交互架构与流程总图.md |
| TERM-CLOUD-001 | CLOUD-GATE   | 订阅门禁   | SubscriptionGate | 订阅校验, 会员校验, 付费门槛, 鉴权门                             | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| TERM-CLOUD-002 | CLOUD-INVOKE | AI网关   | AiGateway        | 模型代理, API网关, 代理服务, provider代理                     | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| TERM-CLOUD-003 | CLOUD-AUTH   | 会话令牌   | SessionToken     | 登录态, token, 认证令牌, 鉴权码                             | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| TERM-CLOUD-004 | CLOUD-INVOKE | BYOK透传 | ByokPassthrough  | 自带密钥, 自定义key, 用户密钥直连                              | docs/01_modules/SYS/A-SYS-I-P-03_云接口契约.md       |
| TERM-TPL-001   | TPL-FLOW     | 写作模板   | Template         | 文档模板片, 提示模板项, 样板, template条目                      | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md   |
| TERM-TPL-002   | TPL-FLOW     | 写作工作流  | WritingWorkflow  | 流程, 工作流程, pipeline, 写作流程图                         | docs/01_modules/TPL/A-TPL-M-T-01_模板工作流技术设计.md   |
| TERM-WS-001    | WS-FS        | 工作区    | Workspace        | 工作空间, 项目根目录, 工作目录, root目录, 项目目录                   | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md       |
| TERM-WS-002    | WS-TIMELINE  | 文档时间轴  | DocumentTimeline | 历史记录, 版本历史, 历史栈, 版本栈, 历史链                         | docs/01_modules/WS/A-WS-M-T-01_工作台技术设计.md       |

### 链路视图

#### AG-ASSIST

| codes                                   | type   | rules                                              | 文件                           | 行号   |
| --------------------------------------- | ------ | -------------------------------------------------- | ---------------------------- | ---- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-002     | RB     | BR-AG-STATE-003 term_ref: TERM-AG-003, TERM-AG-004 | src/components/ed/Editor.tsx | 199  |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-001     | RB     | BR-AG-STATE-003                                    | src-tauri/src/ag/assist.rs   | 39   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001 | EFFECT | BR-AG-STATE-003, BR-AG-VERIFY-001                  | src-tauri/src/lib.rs         | 1434 |

#### AG-TOOL

| codes                                | type  | rules                                 | 文件                       | 行号 |
| ------------------------------------ | ----- | ------------------------------------- | ------------------------ | -- |
| BR-AG-STATE-002-RB-AG-AG-TOOL-001    | RB    | BR-AG-STATE-002 term_ref: TERM-AG-002 | src-tauri/src/ag/tool.rs | 47 |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-001 | GUARD | BR-AG-STATE-002                       | src-tauri/src/ag/tool.rs | 81 |

#### AG-TURN

| codes                                   | type   | rules                                                  | 文件                              | 行号  |
| --------------------------------------- | ------ | ------------------------------------------------------ | ------------------------------- | --- |
| BR-AG-DATA-001-DATA-AG-AG-TURN-001      | DATA   | BR-AG-DATA-001                                         | src-tauri/src/ag/context.rs     | 45  |
| BR-AG-DATA-002-DATA-AG-AG-TURN-002      | DATA   | BR-AG-DATA-002                                         | src-tauri/src/ag/context.rs     | 99  |
| BR-AG-DATA-005-DATA-AG-AG-TURN-004      | DATA   | BR-AG-DATA-005                                         | src-tauri/src/ag/context.rs     | 144 |
| BR-AG-DATA-004-DATA-AG-AG-TURN-003      | DATA   | BR-AG-DATA-004 term_ref: TERM-AG-005                   | src-tauri/src/ag/context.rs     | 179 |
| BR-AG-DATA-003-GUARD-AG-AG-TURN-001     | GUARD  | BR-AG-DATA-003                                         | src-tauri/src/ag/context.rs     | 218 |
| BR-AG-STATE-004-GUARD-AG-AG-TURN-001    | GUARD  | BR-AG-STATE-004                                        | src-tauri/src/ag/memory_link.rs | 22  |
| BR-AG-STATE-004-EFFECT-AG-AG-TURN-002   | EFFECT | BR-AG-STATE-004                                        | src-tauri/src/ag/memory_link.rs | 49  |
| BR-AG-STATE-001-RB-AG-AG-TURN-001       | RB     | BR-AG-STATE-001                                        | src-tauri/src/ag/turn.rs        | 71  |
| BR-AG-VERIFY-001-GUARD-AG-AG-TURN-001   | GUARD  | BR-AG-VERIFY-001                                       | src-tauri/src/ag/turn.rs        | 105 |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-001 | EFFECT | BR-AG-PERSIST-001                                      | src-tauri/src/ag/turn.rs        | 134 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-001  | QUERY  | BR-AG-PERSIST-001                                      | src-tauri/src/ag/turn.rs        | 159 |
| BR-AG-VERIFY-001-GUARD-AG-AG-TURN-002   | GUARD  | BR-AG-VERIFY-001 term_ref: TERM-AG-001, TERM-CLOUD-001 | src-tauri/src/lib.rs            | 923 |

#### AST-MEMORY

| codes                                      | type   | rules            | 文件                           | 行号  |
| ------------------------------------------ | ------ | ---------------- | ---------------------------- | --- |
| BR-AST-STATE-001-RB-AST-AST-MEMORY-001     | RB     | BR-AST-STATE-001 | src-tauri/src/ast/capture.rs | 52  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-001 | EFFECT | BR-AST-STATE-001 | src-tauri/src/ast/capture.rs | 87  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-002 | EFFECT | BR-AST-STATE-001 | src-tauri/src/ast/capture.rs | 131 |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-001   | QUERY  | BR-AST-DATA-001  | src-tauri/src/ast/store.rs   | 12  |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-002   | QUERY  | BR-AST-DATA-001  | src-tauri/src/ast/store.rs   | 37  |
| BR-AST-DATA-002-QUERY-AST-AST-MEMORY-001   | QUERY  | BR-AST-DATA-002  | src-tauri/src/ast/supply.rs  | 23  |

#### CBT-BUILD

| codes                                       | type   | rules              | 文件                            | 行号  |
| ------------------------------------------- | ------ | ------------------ | ----------------------------- | --- |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-001      | DATA   | BR-CBT-DATA-001    | src-tauri/src/cbt/apply.rs    | 12  |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-002      | DATA   | BR-CBT-DATA-001    | src-tauri/src/cbt/apply.rs    | 36  |
| BR-CBT-STATE-001-RB-CBT-CBT-BUILD-001       | RB     | BR-CBT-STATE-001   | src-tauri/src/cbt/build.rs    | 59  |
| BR-CBT-PERSIST-001-EFFECT-CBT-CBT-BUILD-001 | EFFECT | BR-CBT-PERSIST-001 | src-tauri/src/cbt/build.rs    | 99  |
| BR-CBT-PERSIST-001-QUERY-CBT-CBT-BUILD-001  | QUERY  | BR-CBT-PERSIST-001 | src-tauri/src/cbt/build.rs    | 128 |
| BR-CBT-STATE-002-GUARD-CBT-CBT-BUILD-001    | GUARD  | BR-CBT-STATE-002   | src-tauri/src/cbt/takeover.rs | 35  |
| BR-CBT-STATE-002-RB-CBT-CBT-BUILD-001       | RB     | BR-CBT-STATE-002   | src-tauri/src/cbt/takeover.rs | 54  |

#### CLOUD-AUTH

| codes                                       | type   | rules             | 文件                            | 行号 |
| ------------------------------------------- | ------ | ----------------- | ----------------------------- | -- |
| BR-ACCT-STATE-001-RB-ACCT-CLOUD-AUTH-001    | RB     | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 24 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-001 | GUARD  | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 38 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-002 | GUARD  | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 52 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-003 | GUARD  | BR-ACCT-STATE-001 | src-tauri/src/acct/session.rs | 70 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-001 | EFFECT | BR-ACCT-DATA-001  | src-tauri/src/acct/token.rs   | 46 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-002 | EFFECT | BR-ACCT-DATA-001  | src-tauri/src/acct/token.rs   | 60 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-003 | EFFECT | BR-ACCT-DATA-001  | src-tauri/src/acct/token.rs   | 73 |
| BR-SYS-STATE-001-RB-SYS-CLOUD-AUTH-001      | RB     | BR-SYS-STATE-001  | src-tauri/src/session/mod.rs  | 40 |

#### CLOUD-GATE

| codes                                                                                  | type  | rules                              | 文件                           | 行号  |
| -------------------------------------------------------------------------------------- | ----- | ---------------------------------- | ---------------------------- | --- |
| BR-ENT-VERIFY-001-GUARD-ENT-CLOUD-GATE-001                                             | GUARD | BR-ENT-VERIFY-001                  | src-tauri/src/ent/gateway.rs | 119 |
| BR-ENT-DATA-002-GUARD-ENT-CLOUD-GATE-001                                               | GUARD | BR-ENT-DATA-002                    | src-tauri/src/ent/usage.rs   | 34  |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-INVOKE-001 | GUARD | BR-SYS-VERIFY-001, BR-SYS-DATA-001 | src-tauri/src/session/mod.rs | 76  |

#### CLOUD-INVOKE

| codes                                                                                  | type   | rules                              | 文件                           | 行号 |
| -------------------------------------------------------------------------------------- | ------ | ---------------------------------- | ---------------------------- | -- |
| BR-ENT-DATA-001-EFFECT-ENT-CLOUD-INVOKE-001                                            | EFFECT | BR-ENT-DATA-001                    | src-tauri/src/ent/byok.rs    | 17 |
| BR-SYS-OBS-001-DATA-ENT-CLOUD-INVOKE-001                                               | DATA   | BR-SYS-OBS-001                     | src-tauri/src/ent/gateway.rs | 60 |
| BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-001                                               | RB     | BR-ENT-STATE-001                   | src-tauri/src/ent/gateway.rs | 89 |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-INVOKE-001 | GUARD  | BR-SYS-VERIFY-001, BR-SYS-DATA-001 | src-tauri/src/session/mod.rs | 76 |

#### DATA-MIGRATE

| codes                                          | type   | rules              | 文件                                   | 行号  |
| ---------------------------------------------- | ------ | ------------------ | ------------------------------------ | --- |
| BR-SYS-PERSIST-001-EFFECT-SYS-DATA-MIGRATE-001 | EFFECT | BR-SYS-PERSIST-001 | src-tauri/src/db/mod.rs              | 139 |
| BR-SYS-DATA-002-DATA-SYS-DATA-MIGRATE-001      | DATA   | BR-SYS-DATA-002    | src-tauri/src/types/mod.rs           | 9   |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-001   | EFFECT | BR-WS-PERSIST-002  | src-tauri/src/workspace/lifecycle.rs | 34  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-002   | EFFECT | BR-WS-PERSIST-002  | src-tauri/src/workspace/lifecycle.rs | 71  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-003   | EFFECT | BR-WS-PERSIST-002  | src-tauri/src/workspace/lifecycle.rs | 96  |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-001    | QUERY  | BR-WS-PERSIST-002  | src-tauri/src/workspace/lifecycle.rs | 113 |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-002    | QUERY  | BR-WS-PERSIST-002  | src-tauri/src/workspace/lifecycle.rs | 137 |

#### DE-DIFF

| codes                                 | type   | rules                                              | 文件                            | 行号  |
| ------------------------------------- | ------ | -------------------------------------------------- | ----------------------------- | --- |
| BR-DE-STATE-001-RB-DE-DE-DIFF-002     | RB     | BR-DE-STATE-001 term_ref: TERM-DE-001, TERM-DE-003 | src/store/deStore.ts          | 152 |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-001  | GUARD  | BR-DE-STATE-002                                    | src-tauri/src/de/expiry.rs    | 34  |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-001   | GUARD  | BR-DE-DATA-003                                     | src-tauri/src/de/expiry.rs    | 66  |
| BR-DE-DATA-002-DATA-DE-DE-DIFF-001    | DATA   | BR-DE-DATA-002                                     | src-tauri/src/de/generate.rs  | 35  |
| BR-DE-DATA-001-GUARD-DE-DE-DIFF-001   | GUARD  | BR-DE-DATA-001                                     | src-tauri/src/de/generate.rs  | 54  |
| BR-DE-STATE-002-QUERY-DE-DE-DIFF-001  | QUERY  | BR-DE-STATE-002                                    | src-tauri/src/de/lifecycle.rs | 31  |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-002  | GUARD  | BR-DE-STATE-002                                    | src-tauri/src/de/lifecycle.rs | 51  |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-002   | GUARD  | BR-DE-DATA-003                                     | src-tauri/src/de/lifecycle.rs | 79  |
| BR-DE-STATE-002-EFFECT-DE-DE-DIFF-001 | EFFECT | BR-DE-STATE-002                                    | src-tauri/src/de/lifecycle.rs | 99  |
| BR-DE-STATE-001-RB-DE-DE-DIFF-001     | RB     | BR-DE-STATE-001                                    | src-tauri/src/de/mod.rs       | 65  |
| BR-DE-DATA-002-IO-DE-DE-DIFF-002      | IO     | BR-DE-DATA-002 term_ref: TERM-DE-004               | src-tauri/src/de/store.rs     | 28  |
| BR-DE-DATA-002-QUERY-DE-DE-DIFF-001   | QUERY  | BR-DE-DATA-002 term_ref: TERM-DE-004               | src-tauri/src/de/store.rs     | 61  |

#### DOC-CONVERT

| codes                                     | type  | rules            | 文件                                       | 行号  |
| ----------------------------------------- | ----- | ---------------- | ---------------------------------------- | --- |
| BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-001  | DATA  | BR-DOC-DATA-003  | src-tauri/src/doc/block_backfill.rs      | 19  |
| BR-DOC-DATA-003-GUARD-DOC-DOC-CONVERT-002 | GUARD | BR-DOC-DATA-003  | src-tauri/src/doc/block_backfill.rs      | 63  |
| BR-DOC-STATE-001-RB-DOC-DOC-CONVERT-001   | RB    | BR-DOC-STATE-001 | src-tauri/src/doc/convert.rs             | 56  |
| BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-001 | GUARD | BR-DOC-DATA-001  | src-tauri/src/doc/convert.rs             | 89  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-003    | IO    | BR-DOC-DATA-002  | src-tauri/src/doc/edit_chain.rs          | 65  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004    | IO    | BR-DOC-DATA-002  | src-tauri/src/doc/edit_chain.rs          | 112 |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-003    | IO    | BR-DOC-DATA-001  | src-tauri/src/doc/libreoffice_runtime.rs | 97  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-004  | DATA  | BR-DOC-DATA-001  | src-tauri/src/doc/libreoffice_runtime.rs | 158 |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-005    | IO    | BR-DOC-DATA-001  | src-tauri/src/doc/libreoffice_runtime.rs | 294 |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-002  | DATA  | BR-DOC-DATA-001  | src-tauri/src/doc/pandoc_runtime.rs      | 35  |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-001    | IO    | BR-DOC-DATA-001  | src-tauri/src/doc/pandoc_runtime.rs      | 55  |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-002    | IO    | BR-DOC-DATA-001  | src-tauri/src/doc/pandoc_runtime.rs      | 83  |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-001  | DATA  | BR-DOC-DATA-002  | src-tauri/src/doc/style_subset.rs        | 30  |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-002  | DATA  | BR-DOC-DATA-002  | src-tauri/src/doc/style_subset.rs        | 52  |

#### ED-DECOR

| codes                              | type | rules                                              | 文件                                                      | 行号 |
| ---------------------------------- | ---- | -------------------------------------------------- | ------------------------------------------------------- | -- |
| BR-ED-STATE-002-RB-ED-ED-DECOR-001 | RB   | BR-ED-STATE-002 term_ref: TERM-ED-002, TERM-ED-003 | src/components/ed/extensions/DiffDecorationExtension.ts | 67 |

#### ED-EDIT

| codes                              | type | rules                                | 文件                                               | 行号  |
| ---------------------------------- | ---- | ------------------------------------ | ------------------------------------------------ | --- |
| BR-ED-DATA-001-DATA-ED-ED-EDIT-001 | DATA | BR-ED-DATA-001 term_ref: TERM-ED-001 | src/components/ed/extensions/BlockIdExtension.ts | 46  |
| BR-ED-DATA-003-RB-ED-ED-EDIT-005   | RB   | BR-ED-DATA-003                       | src/components/ed/extensions/Pagination.ts       | 151 |
| BR-ED-DATA-003-RB-ED-ED-EDIT-006   | RB   | BR-ED-DATA-003                       | src/services/docService.ts                       | 35  |
| BR-ED-DATA-002-RB-ED-ED-EDIT-004   | RB   | BR-ED-DATA-002                       | src/services/edService.ts                        | 16  |
| BR-ED-STATE-003-RB-ED-ED-EDIT-003  | RB   | BR-ED-STATE-003, BR-WS-OBS-001       | src/services/edService.ts                        | 98  |
| BR-ED-STATE-001-RB-ED-ED-EDIT-002  | RB   | BR-ED-STATE-001                      | src/store/edStore.ts                             | 120 |

#### IPC-CALL

| codes                                    | type   | rules                                 | 文件                         | 行号  |
| ---------------------------------------- | ------ | ------------------------------------- | -------------------------- | --- |
| BR-SYS-OBS-002-DATA-SYS-IPC-CALL-002     | DATA   | BR-SYS-OBS-002                        | src-tauri/src/ipc/mod.rs   | 11  |
| BR-SYS-STATE-002-DATA-SYS-IPC-CALL-003   | DATA   | BR-SYS-STATE-002                      | src-tauri/src/ipc/mod.rs   | 73  |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-001     | API    | BR-SYS-DATA-003                       | src-tauri/src/ipc/mod.rs   | 151 |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-002     | API    | BR-SYS-DATA-003 term_ref: TERM-WS-001 | src-tauri/src/lib.rs       | 3   |
| BR-SYS-STATE-003-EFFECT-SYS-IPC-CALL-001 | EFFECT | BR-SYS-STATE-003                      | src-tauri/src/sm_events.rs | 43  |

#### KB-RETRIEVE

| codes                                    | type   | rules          | 文件                           | 行号 |
| ---------------------------------------- | ------ | -------------- | ---------------------------- | -- |
| BR-KB-DATA-002-QUERY-KB-KB-RETRIEVE-001  | QUERY  | BR-KB-DATA-002 | src-tauri/src/kb/retrieve.rs | 12 |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-001 | EFFECT | BR-KB-DATA-001 | src-tauri/src/kb/store.rs    | 27 |
| BR-KB-DATA-001-QUERY-KB-KB-RETRIEVE-001  | QUERY  | BR-KB-DATA-001 | src-tauri/src/kb/store.rs    | 68 |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-002 | EFFECT | BR-KB-DATA-001 | src-tauri/src/kb/store.rs    | 90 |

#### PRV-VIEW

| codes                                  | type  | rules           | 文件                              | 行号 |
| -------------------------------------- | ----- | --------------- | ------------------------------- | -- |
| BR-PRV-DATA-001-GUARD-PRV-PRV-VIEW-003 | GUARD | BR-PRV-DATA-001 | src/components/prv/PrvPanel.tsx | 28 |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-001    | IO    | BR-PRV-DATA-002 | src-tauri/src/prv/mod.rs        | 23 |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-002    | IO    | BR-PRV-DATA-002 | src-tauri/src/prv/mod.rs        | 48 |

#### REF-INJECT

| codes                                      | type  | rules             | 文件                                 | 行号 |
| ------------------------------------------ | ----- | ----------------- | ---------------------------------- | -- |
| BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-001 | GUARD | BR-REF-VERIFY-001 | src-tauri/src/reference/inject.rs  | 14 |
| BR-REF-DATA-002-DATA-REF-REF-INJECT-001    | DATA  | BR-REF-DATA-002   | src-tauri/src/reference/inject.rs  | 47 |
| BR-REF-DATA-001-GUARD-REF-REF-INJECT-001   | GUARD | BR-REF-DATA-001   | src-tauri/src/reference/resolve.rs | 21 |

#### TPL-FLOW

| codes                                    | type   | rules            | 文件                            | 行号  |
| ---------------------------------------- | ------ | ---------------- | ----------------------------- | --- |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-001  | EFFECT | BR-TPL-DATA-001  | src-tauri/src/tpl/template.rs | 39  |
| BR-TPL-DATA-001-QUERY-TPL-TPL-FLOW-001   | QUERY  | BR-TPL-DATA-001  | src-tauri/src/tpl/template.rs | 67  |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-002  | EFFECT | BR-TPL-DATA-001  | src-tauri/src/tpl/template.rs | 87  |
| BR-TPL-STATE-001-RB-TPL-TPL-FLOW-001     | RB     | BR-TPL-STATE-001 | src-tauri/src/tpl/workflow.rs | 49  |
| BR-TPL-STATE-001-EFFECT-TPL-TPL-FLOW-001 | EFFECT | BR-TPL-STATE-001 | src-tauri/src/tpl/workflow.rs | 89  |
| BR-TPL-STATE-001-QUERY-TPL-TPL-FLOW-001  | QUERY  | BR-TPL-STATE-001 | src-tauri/src/tpl/workflow.rs | 118 |

#### WS-DIFF-APPLY

| codes                                       | type   | rules           | 文件                                    | 行号  |
| ------------------------------------------- | ------ | --------------- | ------------------------------------- | --- |
| BR-WS-STATE-001-RB-WS-WS-DIFF-APPLY-001     | RB     | BR-WS-STATE-001 | src-tauri/src/workspace/diff_apply.rs | 44  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-001 | EFFECT | BR-WS-STATE-001 | src-tauri/src/workspace/diff_apply.rs | 75  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-002 | EFFECT | BR-WS-STATE-001 | src-tauri/src/workspace/diff_apply.rs | 101 |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-003 | EFFECT | BR-WS-STATE-001 | src-tauri/src/workspace/diff_apply.rs | 136 |
| BR-WS-STATE-001-QUERY-WS-WS-DIFF-APPLY-001  | QUERY  | BR-WS-STATE-001 | src-tauri/src/workspace/diff_apply.rs | 188 |

#### WS-FS

| codes                              | type   | rules                                | 文件                                 | 行号  |
| ---------------------------------- | ------ | ------------------------------------ | ---------------------------------- | --- |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-001 | EFFECT | BR-WS-DATA-001 term_ref: TERM-WS-001 | src-tauri/src/workspace/fileops.rs | 16  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-002 | EFFECT | BR-WS-DATA-001 term_ref: TERM-WS-001 | src-tauri/src/workspace/fileops.rs | 38  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-003 | EFFECT | BR-WS-DATA-001 term_ref: TERM-WS-001 | src-tauri/src/workspace/fileops.rs | 63  |
| BR-WS-DATA-001-QUERY-WS-WS-FS-002  | QUERY  | BR-WS-DATA-001 term_ref: TERM-WS-001 | src-tauri/src/workspace/listing.rs | 35  |
| BR-WS-DATA-001-QUERY-WS-WS-FS-005  | QUERY  | BR-WS-DATA-001 term_ref: TERM-WS-001 | src-tauri/src/workspace/listing.rs | 141 |
| BR-WS-DATA-001-GUARD-WS-WS-FS-001  | GUARD  | BR-WS-DATA-001                       | src-tauri/src/workspace/mod.rs     | 40  |
| BR-WS-OBS-001-QUERY-WS-WS-FS-003   | QUERY  | BR-WS-OBS-001                        | src-tauri/src/workspace/save.rs    | 31  |
| BR-WS-OBS-001-QUERY-WS-WS-FS-004   | QUERY  | BR-WS-OBS-001                        | src-tauri/src/workspace/save.rs    | 77  |

#### WS-SAVE

| codes                                   | type   | rules             | 文件                              | 行号  |
| --------------------------------------- | ------ | ----------------- | ------------------------------- | --- |
| BR-WS-OBS-001-GUARD-WS-WS-SAVE-001      | GUARD  | BR-WS-OBS-001     | src-tauri/src/workspace/save.rs | 96  |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-001 | EFFECT | BR-WS-PERSIST-001 | src-tauri/src/workspace/save.rs | 129 |
| BR-WS-PERSIST-001-RB-WS-WS-SAVE-002     | RB     | BR-WS-PERSIST-001 | src-tauri/src/workspace/save.rs | 192 |

#### WS-TIMELINE

| codes                                    | type   | rules           | 文件                                  | 行号  |
| ---------------------------------------- | ------ | --------------- | ----------------------------------- | --- |
| BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-001 | EFFECT | BR-WS-DATA-002  | src-tauri/src/workspace/timeline.rs | 43  |
| BR-WS-DATA-002-QUERY-WS-WS-TIMELINE-001  | QUERY  | BR-WS-DATA-002  | src-tauri/src/workspace/timeline.rs | 87  |
| BR-WS-STATE-002-QUERY-WS-WS-TIMELINE-002 | QUERY  | BR-WS-STATE-002 | src-tauri/src/workspace/timeline.rs | 147 |
| BR-WS-STATE-002-RB-WS-WS-TIMELINE-001    | RB     | BR-WS-STATE-002 | src-tauri/src/workspace/timeline.rs | 187 |

### 规则视图

#### BR-ACCT-DATA-001

| codes                                       | type   | chain      | 文件                          | 行号 |
| ------------------------------------------- | ------ | ---------- | --------------------------- | -- |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-001 | EFFECT | CLOUD-AUTH | src-tauri/src/acct/token.rs | 46 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-002 | EFFECT | CLOUD-AUTH | src-tauri/src/acct/token.rs | 60 |
| BR-ACCT-DATA-001-EFFECT-ACCT-CLOUD-AUTH-003 | EFFECT | CLOUD-AUTH | src-tauri/src/acct/token.rs | 73 |

#### BR-ACCT-STATE-001

| codes                                       | type  | chain      | 文件                            | 行号 |
| ------------------------------------------- | ----- | ---------- | ----------------------------- | -- |
| BR-ACCT-STATE-001-RB-ACCT-CLOUD-AUTH-001    | RB    | CLOUD-AUTH | src-tauri/src/acct/session.rs | 24 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-001 | GUARD | CLOUD-AUTH | src-tauri/src/acct/session.rs | 38 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-002 | GUARD | CLOUD-AUTH | src-tauri/src/acct/session.rs | 52 |
| BR-ACCT-STATE-001-GUARD-ACCT-CLOUD-AUTH-003 | GUARD | CLOUD-AUTH | src-tauri/src/acct/session.rs | 70 |

#### BR-AG-DATA-001

| codes                              | type | chain   | 文件                          | 行号 |
| ---------------------------------- | ---- | ------- | --------------------------- | -- |
| BR-AG-DATA-001-DATA-AG-AG-TURN-001 | DATA | AG-TURN | src-tauri/src/ag/context.rs | 45 |

#### BR-AG-DATA-002

| codes                              | type | chain   | 文件                          | 行号 |
| ---------------------------------- | ---- | ------- | --------------------------- | -- |
| BR-AG-DATA-002-DATA-AG-AG-TURN-002 | DATA | AG-TURN | src-tauri/src/ag/context.rs | 99 |

#### BR-AG-DATA-003

| codes                               | type  | chain   | 文件                          | 行号  |
| ----------------------------------- | ----- | ------- | --------------------------- | --- |
| BR-AG-DATA-003-GUARD-AG-AG-TURN-001 | GUARD | AG-TURN | src-tauri/src/ag/context.rs | 218 |

#### BR-AG-DATA-004 term_ref: TERM-AG-005

| codes                              | type | chain   | 文件                          | 行号  |
| ---------------------------------- | ---- | ------- | --------------------------- | --- |
| BR-AG-DATA-004-DATA-AG-AG-TURN-003 | DATA | AG-TURN | src-tauri/src/ag/context.rs | 179 |

#### BR-AG-DATA-005

| codes                              | type | chain   | 文件                          | 行号  |
| ---------------------------------- | ---- | ------- | --------------------------- | --- |
| BR-AG-DATA-005-DATA-AG-AG-TURN-004 | DATA | AG-TURN | src-tauri/src/ag/context.rs | 144 |

#### BR-AG-PERSIST-001

| codes                                   | type   | chain   | 文件                       | 行号  |
| --------------------------------------- | ------ | ------- | ------------------------ | --- |
| BR-AG-PERSIST-001-EFFECT-AG-AG-TURN-001 | EFFECT | AG-TURN | src-tauri/src/ag/turn.rs | 134 |
| BR-AG-PERSIST-001-QUERY-AG-AG-TURN-001  | QUERY  | AG-TURN | src-tauri/src/ag/turn.rs | 159 |

#### BR-AG-STATE-001

| codes                             | type | chain   | 文件                       | 行号 |
| --------------------------------- | ---- | ------- | ------------------------ | -- |
| BR-AG-STATE-001-RB-AG-AG-TURN-001 | RB   | AG-TURN | src-tauri/src/ag/turn.rs | 71 |

#### BR-AG-STATE-002

| codes                                | type  | chain   | 文件                       | 行号 |
| ------------------------------------ | ----- | ------- | ------------------------ | -- |
| BR-AG-STATE-002-GUARD-AG-AG-TOOL-001 | GUARD | AG-TOOL | src-tauri/src/ag/tool.rs | 81 |

#### BR-AG-STATE-002 term_ref: TERM-AG-002

| codes                             | type | chain   | 文件                       | 行号 |
| --------------------------------- | ---- | ------- | ------------------------ | -- |
| BR-AG-STATE-002-RB-AG-AG-TOOL-001 | RB   | AG-TOOL | src-tauri/src/ag/tool.rs | 47 |

#### BR-AG-STATE-003

| codes                                   | type   | chain     | 文件                         | 行号   |
| --------------------------------------- | ------ | --------- | -------------------------- | ---- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-001     | RB     | AG-ASSIST | src-tauri/src/ag/assist.rs | 39   |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001 | EFFECT | AG-ASSIST | src-tauri/src/lib.rs       | 1434 |

#### BR-AG-STATE-003 term_ref: TERM-AG-003

| codes                               | type | chain     | 文件                           | 行号  |
| ----------------------------------- | ---- | --------- | ---------------------------- | --- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-002 | RB   | AG-ASSIST | src/components/ed/Editor.tsx | 199 |

#### BR-AG-STATE-004

| codes                                 | type   | chain   | 文件                              | 行号 |
| ------------------------------------- | ------ | ------- | ------------------------------- | -- |
| BR-AG-STATE-004-GUARD-AG-AG-TURN-001  | GUARD  | AG-TURN | src-tauri/src/ag/memory_link.rs | 22 |
| BR-AG-STATE-004-EFFECT-AG-AG-TURN-002 | EFFECT | AG-TURN | src-tauri/src/ag/memory_link.rs | 49 |

#### BR-AG-VERIFY-001

| codes                                   | type   | chain     | 文件                       | 行号   |
| --------------------------------------- | ------ | --------- | ------------------------ | ---- |
| BR-AG-VERIFY-001-GUARD-AG-AG-TURN-001   | GUARD  | AG-TURN   | src-tauri/src/ag/turn.rs | 105  |
| BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-001 | EFFECT | AG-ASSIST | src-tauri/src/lib.rs     | 1434 |

#### BR-AG-VERIFY-001 term_ref: TERM-AG-001

| codes                                 | type  | chain   | 文件                   | 行号  |
| ------------------------------------- | ----- | ------- | -------------------- | --- |
| BR-AG-VERIFY-001-GUARD-AG-AG-TURN-002 | GUARD | AG-TURN | src-tauri/src/lib.rs | 923 |

#### BR-AST-DATA-001

| codes                                    | type  | chain      | 文件                         | 行号 |
| ---------------------------------------- | ----- | ---------- | -------------------------- | -- |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-001 | QUERY | AST-MEMORY | src-tauri/src/ast/store.rs | 12 |
| BR-AST-DATA-001-QUERY-AST-AST-MEMORY-002 | QUERY | AST-MEMORY | src-tauri/src/ast/store.rs | 37 |

#### BR-AST-DATA-002

| codes                                    | type  | chain      | 文件                          | 行号 |
| ---------------------------------------- | ----- | ---------- | --------------------------- | -- |
| BR-AST-DATA-002-QUERY-AST-AST-MEMORY-001 | QUERY | AST-MEMORY | src-tauri/src/ast/supply.rs | 23 |

#### BR-AST-STATE-001

| codes                                      | type   | chain      | 文件                           | 行号  |
| ------------------------------------------ | ------ | ---------- | ---------------------------- | --- |
| BR-AST-STATE-001-RB-AST-AST-MEMORY-001     | RB     | AST-MEMORY | src-tauri/src/ast/capture.rs | 52  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-001 | EFFECT | AST-MEMORY | src-tauri/src/ast/capture.rs | 87  |
| BR-AST-STATE-001-EFFECT-AST-AST-MEMORY-002 | EFFECT | AST-MEMORY | src-tauri/src/ast/capture.rs | 131 |

#### BR-CBT-DATA-001

| codes                                  | type | chain     | 文件                         | 行号 |
| -------------------------------------- | ---- | --------- | -------------------------- | -- |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-001 | DATA | CBT-BUILD | src-tauri/src/cbt/apply.rs | 12 |
| BR-CBT-DATA-001-DATA-CBT-CBT-BUILD-002 | DATA | CBT-BUILD | src-tauri/src/cbt/apply.rs | 36 |

#### BR-CBT-PERSIST-001

| codes                                       | type   | chain     | 文件                         | 行号  |
| ------------------------------------------- | ------ | --------- | -------------------------- | --- |
| BR-CBT-PERSIST-001-EFFECT-CBT-CBT-BUILD-001 | EFFECT | CBT-BUILD | src-tauri/src/cbt/build.rs | 99  |
| BR-CBT-PERSIST-001-QUERY-CBT-CBT-BUILD-001  | QUERY  | CBT-BUILD | src-tauri/src/cbt/build.rs | 128 |

#### BR-CBT-STATE-001

| codes                                 | type | chain     | 文件                         | 行号 |
| ------------------------------------- | ---- | --------- | -------------------------- | -- |
| BR-CBT-STATE-001-RB-CBT-CBT-BUILD-001 | RB   | CBT-BUILD | src-tauri/src/cbt/build.rs | 59 |

#### BR-CBT-STATE-002

| codes                                    | type  | chain     | 文件                            | 行号 |
| ---------------------------------------- | ----- | --------- | ----------------------------- | -- |
| BR-CBT-STATE-002-GUARD-CBT-CBT-BUILD-001 | GUARD | CBT-BUILD | src-tauri/src/cbt/takeover.rs | 35 |
| BR-CBT-STATE-002-RB-CBT-CBT-BUILD-001    | RB    | CBT-BUILD | src-tauri/src/cbt/takeover.rs | 54 |

#### BR-DE-DATA-001

| codes                               | type  | chain   | 文件                           | 行号 |
| ----------------------------------- | ----- | ------- | ---------------------------- | -- |
| BR-DE-DATA-001-GUARD-DE-DE-DIFF-001 | GUARD | DE-DIFF | src-tauri/src/de/generate.rs | 54 |

#### BR-DE-DATA-002

| codes                              | type | chain   | 文件                           | 行号 |
| ---------------------------------- | ---- | ------- | ---------------------------- | -- |
| BR-DE-DATA-002-DATA-DE-DE-DIFF-001 | DATA | DE-DIFF | src-tauri/src/de/generate.rs | 35 |

#### BR-DE-DATA-002 term_ref: TERM-DE-004

| codes                               | type  | chain   | 文件                        | 行号 |
| ----------------------------------- | ----- | ------- | ------------------------- | -- |
| BR-DE-DATA-002-IO-DE-DE-DIFF-002    | IO    | DE-DIFF | src-tauri/src/de/store.rs | 28 |
| BR-DE-DATA-002-QUERY-DE-DE-DIFF-001 | QUERY | DE-DIFF | src-tauri/src/de/store.rs | 61 |

#### BR-DE-DATA-003

| codes                               | type  | chain   | 文件                            | 行号 |
| ----------------------------------- | ----- | ------- | ----------------------------- | -- |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-001 | GUARD | DE-DIFF | src-tauri/src/de/expiry.rs    | 66 |
| BR-DE-DATA-003-GUARD-DE-DE-DIFF-002 | GUARD | DE-DIFF | src-tauri/src/de/lifecycle.rs | 79 |

#### BR-DE-STATE-001

| codes                             | type | chain   | 文件                      | 行号 |
| --------------------------------- | ---- | ------- | ----------------------- | -- |
| BR-DE-STATE-001-RB-DE-DE-DIFF-001 | RB   | DE-DIFF | src-tauri/src/de/mod.rs | 65 |

#### BR-DE-STATE-001 term_ref: TERM-DE-001

| codes                             | type | chain   | 文件                   | 行号  |
| --------------------------------- | ---- | ------- | -------------------- | --- |
| BR-DE-STATE-001-RB-DE-DE-DIFF-002 | RB   | DE-DIFF | src/store/deStore.ts | 152 |

#### BR-DE-STATE-002

| codes                                 | type   | chain   | 文件                            | 行号 |
| ------------------------------------- | ------ | ------- | ----------------------------- | -- |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-001  | GUARD  | DE-DIFF | src-tauri/src/de/expiry.rs    | 34 |
| BR-DE-STATE-002-QUERY-DE-DE-DIFF-001  | QUERY  | DE-DIFF | src-tauri/src/de/lifecycle.rs | 31 |
| BR-DE-STATE-002-GUARD-DE-DE-DIFF-002  | GUARD  | DE-DIFF | src-tauri/src/de/lifecycle.rs | 51 |
| BR-DE-STATE-002-EFFECT-DE-DE-DIFF-001 | EFFECT | DE-DIFF | src-tauri/src/de/lifecycle.rs | 99 |

#### BR-DOC-DATA-001

| codes                                     | type  | chain       | 文件                                       | 行号  |
| ----------------------------------------- | ----- | ----------- | ---------------------------------------- | --- |
| BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-001 | GUARD | DOC-CONVERT | src-tauri/src/doc/convert.rs             | 89  |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-003    | IO    | DOC-CONVERT | src-tauri/src/doc/libreoffice_runtime.rs | 97  |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-004  | DATA  | DOC-CONVERT | src-tauri/src/doc/libreoffice_runtime.rs | 158 |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-005    | IO    | DOC-CONVERT | src-tauri/src/doc/libreoffice_runtime.rs | 294 |
| BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-002  | DATA  | DOC-CONVERT | src-tauri/src/doc/pandoc_runtime.rs      | 35  |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-001    | IO    | DOC-CONVERT | src-tauri/src/doc/pandoc_runtime.rs      | 55  |
| BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-002    | IO    | DOC-CONVERT | src-tauri/src/doc/pandoc_runtime.rs      | 83  |

#### BR-DOC-DATA-002

| codes                                    | type | chain       | 文件                                | 行号  |
| ---------------------------------------- | ---- | ----------- | --------------------------------- | --- |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-003   | IO   | DOC-CONVERT | src-tauri/src/doc/edit_chain.rs   | 65  |
| BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004   | IO   | DOC-CONVERT | src-tauri/src/doc/edit_chain.rs   | 112 |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-001 | DATA | DOC-CONVERT | src-tauri/src/doc/style_subset.rs | 30  |
| BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-002 | DATA | DOC-CONVERT | src-tauri/src/doc/style_subset.rs | 52  |

#### BR-DOC-DATA-003

| codes                                     | type  | chain       | 文件                                  | 行号 |
| ----------------------------------------- | ----- | ----------- | ----------------------------------- | -- |
| BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-001  | DATA  | DOC-CONVERT | src-tauri/src/doc/block_backfill.rs | 19 |
| BR-DOC-DATA-003-GUARD-DOC-DOC-CONVERT-002 | GUARD | DOC-CONVERT | src-tauri/src/doc/block_backfill.rs | 63 |

#### BR-DOC-STATE-001

| codes                                   | type | chain       | 文件                           | 行号 |
| --------------------------------------- | ---- | ----------- | ---------------------------- | -- |
| BR-DOC-STATE-001-RB-DOC-DOC-CONVERT-001 | RB   | DOC-CONVERT | src-tauri/src/doc/convert.rs | 56 |

#### BR-ED-DATA-001 term_ref: TERM-ED-001

| codes                              | type | chain   | 文件                                               | 行号 |
| ---------------------------------- | ---- | ------- | ------------------------------------------------ | -- |
| BR-ED-DATA-001-DATA-ED-ED-EDIT-001 | DATA | ED-EDIT | src/components/ed/extensions/BlockIdExtension.ts | 46 |

#### BR-ED-DATA-002

| codes                            | type | chain   | 文件                        | 行号 |
| -------------------------------- | ---- | ------- | ------------------------- | -- |
| BR-ED-DATA-002-RB-ED-ED-EDIT-004 | RB   | ED-EDIT | src/services/edService.ts | 16 |

#### BR-ED-DATA-003

| codes                            | type | chain   | 文件                                         | 行号  |
| -------------------------------- | ---- | ------- | ------------------------------------------ | --- |
| BR-ED-DATA-003-RB-ED-ED-EDIT-005 | RB   | ED-EDIT | src/components/ed/extensions/Pagination.ts | 151 |
| BR-ED-DATA-003-RB-ED-ED-EDIT-006 | RB   | ED-EDIT | src/services/docService.ts                 | 35  |

#### BR-ED-STATE-001

| codes                             | type | chain   | 文件                   | 行号  |
| --------------------------------- | ---- | ------- | -------------------- | --- |
| BR-ED-STATE-001-RB-ED-ED-EDIT-002 | RB   | ED-EDIT | src/store/edStore.ts | 120 |

#### BR-ED-STATE-002 term_ref: TERM-ED-002

| codes                              | type | chain    | 文件                                                      | 行号 |
| ---------------------------------- | ---- | -------- | ------------------------------------------------------- | -- |
| BR-ED-STATE-002-RB-ED-ED-DECOR-001 | RB   | ED-DECOR | src/components/ed/extensions/DiffDecorationExtension.ts | 67 |

#### BR-ED-STATE-003

| codes                             | type | chain   | 文件                        | 行号 |
| --------------------------------- | ---- | ------- | ------------------------- | -- |
| BR-ED-STATE-003-RB-ED-ED-EDIT-003 | RB   | ED-EDIT | src/services/edService.ts | 98 |

#### BR-ENT-DATA-001

| codes                                       | type   | chain        | 文件                        | 行号 |
| ------------------------------------------- | ------ | ------------ | ------------------------- | -- |
| BR-ENT-DATA-001-EFFECT-ENT-CLOUD-INVOKE-001 | EFFECT | CLOUD-INVOKE | src-tauri/src/ent/byok.rs | 17 |

#### BR-ENT-DATA-002

| codes                                    | type  | chain      | 文件                         | 行号 |
| ---------------------------------------- | ----- | ---------- | -------------------------- | -- |
| BR-ENT-DATA-002-GUARD-ENT-CLOUD-GATE-001 | GUARD | CLOUD-GATE | src-tauri/src/ent/usage.rs | 34 |

#### BR-ENT-STATE-001

| codes                                    | type | chain        | 文件                           | 行号 |
| ---------------------------------------- | ---- | ------------ | ---------------------------- | -- |
| BR-ENT-STATE-001-RB-ENT-CLOUD-INVOKE-001 | RB   | CLOUD-INVOKE | src-tauri/src/ent/gateway.rs | 89 |

#### BR-ENT-VERIFY-001

| codes                                      | type  | chain      | 文件                           | 行号  |
| ------------------------------------------ | ----- | ---------- | ---------------------------- | --- |
| BR-ENT-VERIFY-001-GUARD-ENT-CLOUD-GATE-001 | GUARD | CLOUD-GATE | src-tauri/src/ent/gateway.rs | 119 |

#### BR-KB-DATA-001

| codes                                    | type   | chain       | 文件                        | 行号 |
| ---------------------------------------- | ------ | ----------- | ------------------------- | -- |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-001 | EFFECT | KB-RETRIEVE | src-tauri/src/kb/store.rs | 27 |
| BR-KB-DATA-001-QUERY-KB-KB-RETRIEVE-001  | QUERY  | KB-RETRIEVE | src-tauri/src/kb/store.rs | 68 |
| BR-KB-DATA-001-EFFECT-KB-KB-RETRIEVE-002 | EFFECT | KB-RETRIEVE | src-tauri/src/kb/store.rs | 90 |

#### BR-KB-DATA-002

| codes                                   | type  | chain       | 文件                           | 行号 |
| --------------------------------------- | ----- | ----------- | ---------------------------- | -- |
| BR-KB-DATA-002-QUERY-KB-KB-RETRIEVE-001 | QUERY | KB-RETRIEVE | src-tauri/src/kb/retrieve.rs | 12 |

#### BR-PRV-DATA-001

| codes                                  | type  | chain    | 文件                              | 行号 |
| -------------------------------------- | ----- | -------- | ------------------------------- | -- |
| BR-PRV-DATA-001-GUARD-PRV-PRV-VIEW-003 | GUARD | PRV-VIEW | src/components/prv/PrvPanel.tsx | 28 |

#### BR-PRV-DATA-002

| codes                               | type | chain    | 文件                       | 行号 |
| ----------------------------------- | ---- | -------- | ------------------------ | -- |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-001 | IO   | PRV-VIEW | src-tauri/src/prv/mod.rs | 23 |
| BR-PRV-DATA-002-IO-PRV-PRV-VIEW-002 | IO   | PRV-VIEW | src-tauri/src/prv/mod.rs | 48 |

#### BR-REF-DATA-001

| codes                                    | type  | chain      | 文件                                 | 行号 |
| ---------------------------------------- | ----- | ---------- | ---------------------------------- | -- |
| BR-REF-DATA-001-GUARD-REF-REF-INJECT-001 | GUARD | REF-INJECT | src-tauri/src/reference/resolve.rs | 21 |

#### BR-REF-DATA-002

| codes                                   | type | chain      | 文件                                | 行号 |
| --------------------------------------- | ---- | ---------- | --------------------------------- | -- |
| BR-REF-DATA-002-DATA-REF-REF-INJECT-001 | DATA | REF-INJECT | src-tauri/src/reference/inject.rs | 47 |

#### BR-REF-VERIFY-001

| codes                                      | type  | chain      | 文件                                | 行号 |
| ------------------------------------------ | ----- | ---------- | --------------------------------- | -- |
| BR-REF-VERIFY-001-GUARD-REF-REF-INJECT-001 | GUARD | REF-INJECT | src-tauri/src/reference/inject.rs | 14 |

#### BR-SYS-DATA-001

| codes                                                                                  | type  | chain                    | 文件                           | 行号 |
| -------------------------------------------------------------------------------------- | ----- | ------------------------ | ---------------------------- | -- |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-INVOKE-001 | GUARD | CLOUD-GATE, CLOUD-INVOKE | src-tauri/src/session/mod.rs | 76 |

#### BR-SYS-DATA-002

| codes                                     | type | chain        | 文件                         | 行号 |
| ----------------------------------------- | ---- | ------------ | -------------------------- | -- |
| BR-SYS-DATA-002-DATA-SYS-DATA-MIGRATE-001 | DATA | DATA-MIGRATE | src-tauri/src/types/mod.rs | 9  |

#### BR-SYS-DATA-003

| codes                                | type | chain    | 文件                       | 行号  |
| ------------------------------------ | ---- | -------- | ------------------------ | --- |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-001 | API  | IPC-CALL | src-tauri/src/ipc/mod.rs | 151 |

#### BR-SYS-DATA-003 term_ref: TERM-WS-001

| codes                                | type | chain    | 文件                   | 行号 |
| ------------------------------------ | ---- | -------- | -------------------- | -- |
| BR-SYS-DATA-003-API-SYS-IPC-CALL-002 | API  | IPC-CALL | src-tauri/src/lib.rs | 3  |

#### BR-SYS-OBS-001

| codes                                    | type | chain        | 文件                           | 行号 |
| ---------------------------------------- | ---- | ------------ | ---------------------------- | -- |
| BR-SYS-OBS-001-DATA-ENT-CLOUD-INVOKE-001 | DATA | CLOUD-INVOKE | src-tauri/src/ent/gateway.rs | 60 |

#### BR-SYS-OBS-002

| codes                                | type | chain    | 文件                       | 行号 |
| ------------------------------------ | ---- | -------- | ------------------------ | -- |
| BR-SYS-OBS-002-DATA-SYS-IPC-CALL-002 | DATA | IPC-CALL | src-tauri/src/ipc/mod.rs | 11 |

#### BR-SYS-PERSIST-001

| codes                                          | type   | chain        | 文件                      | 行号  |
| ---------------------------------------------- | ------ | ------------ | ----------------------- | --- |
| BR-SYS-PERSIST-001-EFFECT-SYS-DATA-MIGRATE-001 | EFFECT | DATA-MIGRATE | src-tauri/src/db/mod.rs | 139 |

#### BR-SYS-STATE-001

| codes                                  | type | chain      | 文件                           | 行号 |
| -------------------------------------- | ---- | ---------- | ---------------------------- | -- |
| BR-SYS-STATE-001-RB-SYS-CLOUD-AUTH-001 | RB   | CLOUD-AUTH | src-tauri/src/session/mod.rs | 40 |

#### BR-SYS-STATE-002

| codes                                  | type | chain    | 文件                       | 行号 |
| -------------------------------------- | ---- | -------- | ------------------------ | -- |
| BR-SYS-STATE-002-DATA-SYS-IPC-CALL-003 | DATA | IPC-CALL | src-tauri/src/ipc/mod.rs | 73 |

#### BR-SYS-STATE-003

| codes                                    | type   | chain    | 文件                         | 行号 |
| ---------------------------------------- | ------ | -------- | -------------------------- | -- |
| BR-SYS-STATE-003-EFFECT-SYS-IPC-CALL-001 | EFFECT | IPC-CALL | src-tauri/src/sm_events.rs | 43 |

#### BR-SYS-VERIFY-001

| codes                                                                                  | type  | chain                    | 文件                           | 行号 |
| -------------------------------------------------------------------------------------- | ----- | ------------------------ | ---------------------------- | -- |
| BR-SYS-VERIFY-001-GUARD-SYS-CLOUD-GATE-001, BR-SYS-DATA-001-GUARD-SYS-CLOUD-INVOKE-001 | GUARD | CLOUD-GATE, CLOUD-INVOKE | src-tauri/src/session/mod.rs | 76 |

#### BR-TPL-DATA-001

| codes                                   | type   | chain    | 文件                            | 行号 |
| --------------------------------------- | ------ | -------- | ----------------------------- | -- |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-001 | EFFECT | TPL-FLOW | src-tauri/src/tpl/template.rs | 39 |
| BR-TPL-DATA-001-QUERY-TPL-TPL-FLOW-001  | QUERY  | TPL-FLOW | src-tauri/src/tpl/template.rs | 67 |
| BR-TPL-DATA-001-EFFECT-TPL-TPL-FLOW-002 | EFFECT | TPL-FLOW | src-tauri/src/tpl/template.rs | 87 |

#### BR-TPL-STATE-001

| codes                                    | type   | chain    | 文件                            | 行号  |
| ---------------------------------------- | ------ | -------- | ----------------------------- | --- |
| BR-TPL-STATE-001-RB-TPL-TPL-FLOW-001     | RB     | TPL-FLOW | src-tauri/src/tpl/workflow.rs | 49  |
| BR-TPL-STATE-001-EFFECT-TPL-TPL-FLOW-001 | EFFECT | TPL-FLOW | src-tauri/src/tpl/workflow.rs | 89  |
| BR-TPL-STATE-001-QUERY-TPL-TPL-FLOW-001  | QUERY  | TPL-FLOW | src-tauri/src/tpl/workflow.rs | 118 |

#### BR-WS-DATA-001

| codes                             | type  | chain | 文件                             | 行号 |
| --------------------------------- | ----- | ----- | ------------------------------ | -- |
| BR-WS-DATA-001-GUARD-WS-WS-FS-001 | GUARD | WS-FS | src-tauri/src/workspace/mod.rs | 40 |

#### BR-WS-DATA-001 term_ref: TERM-WS-001

| codes                              | type   | chain | 文件                                 | 行号  |
| ---------------------------------- | ------ | ----- | ---------------------------------- | --- |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-001 | EFFECT | WS-FS | src-tauri/src/workspace/fileops.rs | 16  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-002 | EFFECT | WS-FS | src-tauri/src/workspace/fileops.rs | 38  |
| BR-WS-DATA-001-EFFECT-WS-WS-FS-003 | EFFECT | WS-FS | src-tauri/src/workspace/fileops.rs | 63  |
| BR-WS-DATA-001-QUERY-WS-WS-FS-002  | QUERY  | WS-FS | src-tauri/src/workspace/listing.rs | 35  |
| BR-WS-DATA-001-QUERY-WS-WS-FS-005  | QUERY  | WS-FS | src-tauri/src/workspace/listing.rs | 141 |

#### BR-WS-DATA-002

| codes                                    | type   | chain       | 文件                                  | 行号 |
| ---------------------------------------- | ------ | ----------- | ----------------------------------- | -- |
| BR-WS-DATA-002-EFFECT-WS-WS-TIMELINE-001 | EFFECT | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 43 |
| BR-WS-DATA-002-QUERY-WS-WS-TIMELINE-001  | QUERY  | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 87 |

#### BR-WS-OBS-001

| codes                              | type  | chain   | 文件                              | 行号 |
| ---------------------------------- | ----- | ------- | ------------------------------- | -- |
| BR-ED-STATE-003-RB-ED-ED-EDIT-003  | RB    | ED-EDIT | src/services/edService.ts       | 98 |
| BR-WS-OBS-001-QUERY-WS-WS-FS-003   | QUERY | WS-FS   | src-tauri/src/workspace/save.rs | 31 |
| BR-WS-OBS-001-QUERY-WS-WS-FS-004   | QUERY | WS-FS   | src-tauri/src/workspace/save.rs | 77 |
| BR-WS-OBS-001-GUARD-WS-WS-SAVE-001 | GUARD | WS-SAVE | src-tauri/src/workspace/save.rs | 96 |

#### BR-WS-PERSIST-001

| codes                                   | type   | chain   | 文件                              | 行号  |
| --------------------------------------- | ------ | ------- | ------------------------------- | --- |
| BR-WS-PERSIST-001-EFFECT-WS-WS-SAVE-001 | EFFECT | WS-SAVE | src-tauri/src/workspace/save.rs | 129 |
| BR-WS-PERSIST-001-RB-WS-WS-SAVE-002     | RB     | WS-SAVE | src-tauri/src/workspace/save.rs | 192 |

#### BR-WS-PERSIST-002

| codes                                        | type   | chain        | 文件                                   | 行号  |
| -------------------------------------------- | ------ | ------------ | ------------------------------------ | --- |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-001 | EFFECT | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 34  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-002 | EFFECT | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 71  |
| BR-WS-PERSIST-002-EFFECT-WS-DATA-MIGRATE-003 | EFFECT | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 96  |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-001  | QUERY  | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 113 |
| BR-WS-PERSIST-002-QUERY-WS-DATA-MIGRATE-002  | QUERY  | DATA-MIGRATE | src-tauri/src/workspace/lifecycle.rs | 137 |

#### BR-WS-STATE-001

| codes                                       | type   | chain         | 文件                                    | 行号  |
| ------------------------------------------- | ------ | ------------- | ------------------------------------- | --- |
| BR-WS-STATE-001-RB-WS-WS-DIFF-APPLY-001     | RB     | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 44  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-001 | EFFECT | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 75  |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-002 | EFFECT | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 101 |
| BR-WS-STATE-001-EFFECT-WS-WS-DIFF-APPLY-003 | EFFECT | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 136 |
| BR-WS-STATE-001-QUERY-WS-WS-DIFF-APPLY-001  | QUERY  | WS-DIFF-APPLY | src-tauri/src/workspace/diff_apply.rs | 188 |

#### BR-WS-STATE-002

| codes                                    | type  | chain       | 文件                                  | 行号  |
| ---------------------------------------- | ----- | ----------- | ----------------------------------- | --- |
| BR-WS-STATE-002-QUERY-WS-WS-TIMELINE-002 | QUERY | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 147 |
| BR-WS-STATE-002-RB-WS-WS-TIMELINE-001    | RB    | WS-TIMELINE | src-tauri/src/workspace/timeline.rs | 187 |

#### TERM-AG-004

| codes                               | type | chain     | 文件                           | 行号  |
| ----------------------------------- | ---- | --------- | ---------------------------- | --- |
| BR-AG-STATE-003-RB-AG-AG-ASSIST-002 | RB   | AG-ASSIST | src/components/ed/Editor.tsx | 199 |

#### TERM-CLOUD-001

| codes                                 | type  | chain   | 文件                   | 行号  |
| ------------------------------------- | ----- | ------- | -------------------- | --- |
| BR-AG-VERIFY-001-GUARD-AG-AG-TURN-002 | GUARD | AG-TURN | src-tauri/src/lib.rs | 923 |

#### TERM-DE-003

| codes                             | type | chain   | 文件                   | 行号  |
| --------------------------------- | ---- | ------- | -------------------- | --- |
| BR-DE-STATE-001-RB-DE-DE-DIFF-002 | RB   | DE-DIFF | src/store/deStore.ts | 152 |

#### TERM-ED-003

| codes                              | type | chain    | 文件                                                      | 行号 |
| ---------------------------------- | ---- | -------- | ------------------------------------------------------- | -- |
| BR-ED-STATE-002-RB-ED-ED-DECOR-001 | RB   | ED-DECOR | src/components/ed/extensions/DiffDecorationExtension.ts | 67 |

### triage 汇总

（triage 报告存在后自动生成）

| codes | 文件 | 分类 | 已解决 |
|-------|------|------|--------|
| （待生成） | | | |

注：triage 汇总为精简视图，只包含 codes、文件、分类、已解决四列。
完整的七字段记录（含原因、已采取动作、需要人类介入）
保存在各模块的 [模块]_triage.md 报告文件中。
