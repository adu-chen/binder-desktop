# Issue Trace — AG-1.0-R3-REQUIREMENTS-01：编辑格式、BDocx 路径与表格/图片边界

文档状态：A
任务类型：DOC_ONLY
治理阶段：REQUIREMENT
创建时间：2026-08-13

## 1. 用户可见问题

现有需求同时出现“原生 DOCX 往返编辑”“原生 DOCX 只读预览”和把 `BDocx` 当作文件格式的描述；表格和图片虽有候选能力，却没有锁定其仅适用于 HTML 与 BDocx 处理路径。继续以这些表述进入设计，会使 Agent 对原文件的写入目标和格式能力产生漂移。

## 2. 本轮人类确认

1. 原生 DOCX 只允许预览、读取与内容分析，不可在原文件上编辑或保存。
2. 对原生 DOCX 的编辑请求必须生成“创建 BDocx 编辑副本”的 Proposal；预览成功后该副本即成为新的可编辑 LogicalState。接受只裁决 Proposal 生命周期；仅显式保存才形成新的 DOCX 文件，原文件保持不变。
3. `BDocx` 是处理路径而不是文件格式：副本的磁盘保存格式仍为 DOCX；编辑时经 Pandoc 转为 HTML，由编辑器以模拟 Word 分页呈现；保存时再经 Pandoc 转回 DOCX。
4. 纯文本与 Markdown 的既有边界确认不变：纯文本不承诺富样式；Markdown 仅承诺其可表达的结构/样式语义，不把渲染效果表述为保存保真。
5. 表格与图片均为 P1 受控开放，支持对象限定为 HTML 编辑文档和 BDocx 处理路径；保留已列出的操作清单，逐项验证和开启。
6. HTML 与 BDocx 的样式子集尚未确认；本轮不得把任何具体样式操作写为新增 P0 保证。

## 3. 需求边界

- 原生 DOCX 的预览链与 BDocx 编辑链是不同用户路径；前者不能因 Agent 产生 Proposal 而变成可编辑标签。
- “创建副本”遵循已确认的新建与 Proposal 生命周期：预览成功后才有可继续编辑的 LogicalState；接受只裁决 Proposal；显式保存才创建副本 DOCX。
- 本轮只同步 L1 的用户可见行为、P0/P1 档位和格式/处理路径边界；不改 L2 转换接口、状态机、代码或工具清单。

## 4. 直接消费者

- CORE：产品级 DOCX 只读与副本编辑承诺。
- AG / DE：编辑意图与 Proposal 的目标必须从原生 DOCX 转为新 BDocx 编辑副本。
- ED：HTML 编辑器和模拟 Word 分页呈现 BDocx 处理副本；原生 DOCX 不进入编辑会话。
- DOC / PRV：分别承接 BDocx 编辑转换路径与原生 DOCX 只读预览路径。
- WS：副本作为新文档的保存与原文件保护。

## 5. 验收计划

- L1 文档逐项检查：原生 DOCX 不再被描述为可直接编辑或原地往返保存；BDocx 不再被描述为文件格式。
- 表格与图片在能力基线和直接模块 L1 中均标明 P1，且仅对 HTML/BDocx 处理路径开放。
- 纯文本/Markdown 边界与 R3 未确认的样式子集不被扩大。
- 本 Scope 不运行实现测试；后续 DESIGN Scope 才对 Pandoc、编辑器分页、复制命名/元数据、转换错误和格式保真建立 RULE/CHAIN/接口与验证。

## 6. 待确认项

- HTML 与 BDocx 处理路径的具体样式子集、每项保真等级及失败告知。
- BDocx 编辑副本的默认命名、来源关联与再次打开识别策略。

## 7. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`REQUIREMENT`
- final_state：`READY`（R3 已确认部分）；HTML/BDocx 样式子集是下一轮独立确认项，不视为本 Scope 未完成。
- changed_files：CORE 产品概述与需求索引；AG 能力基线和功能需求；DE、ED、WS、DOC、PRV 的直接 L1 消费者；本 Issue Trace 与 active scope。
- validation：逐文件检查原生 DOCX 的命中上下文均为只读或“创建 BDocx 编辑副本”；检查 BDocx 的格式表述均明确为处理路径；HTML/BDocx 表格、图片均标注为 P1；样式条目均为待 R3 确认；受 Git 跟踪的 Scope 文件经 `git diff --check` 无输出，包含未跟踪 Scope 文档在内的尾随空白扫描亦无命中。
- automated_tests：不适用。本 Scope 无代码、接口、状态机或验证资产改动。
- unresolved_items：HTML/BDocx 样式操作、保真等级和失败告知；BDocx 副本的命名、来源关联与再次打开识别策略；R4~R5 需求确认。
