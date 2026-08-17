# Issue Trace — AG-1.0-R4-CONVERSION-REQUIREMENTS-01：最小格式转换矩阵与复制新建生命周期

文档状态：A
任务类型：DOC_ONLY
治理阶段：REQUIREMENT
创建时间：2026-08-14

## 1. 用户可见问题

“文档格式转换”若只按现有转换函数或直接文件写入理解，会混淆 BDocx 的专用编辑副本链、通用格式能力、
来源文件保护和目标文件何时真正落盘。用户需要一个可预览、可继续编辑、可明确判断损失范围的最小转换能力，
而不是未经验证的“任何格式互转”承诺。

## 2. 本轮人类确认

1. `CAP-13` 在 Agent 1.0 的 P1 仅纳入 `TXT → Markdown` 与 `Markdown → TXT`。
2. 两个方向都创建新的目标副本，绝不原地改写来源；`TXT → Markdown` 不自动推断 Markdown 语义。
3. `Markdown → TXT` 是有损转换，必须在提案中告知不承诺样式、结构、表格、图片或可逆往返。
4. 打开的来源读取当前 LogicalState；未打开来源读取 DiskState。转换提案要展示来源、输入/输出格式、目标路径和保真/损失说明。
5. 预览成功后建立目标 LogicalState 与编辑器标签；接受仅裁决转换卡，用户显式保存当前目标才写入目标 DiskState；目标后续编辑使旧卡失效。
6. HTML 互转、通用 DOCX 与 TXT/Markdown/HTML 互转、批量/原地转换不纳入。BDocx 继续是 R3 的专用编辑副本处理路径，不是格式，也不变成通用转换。

## 3. 需求边界

- 转换是“新建目标副本”的一种受控提案，不把来源内容视为可被转换过程写回的目标。
- 转换预览前的格式、路径或转换失败不得产生目标 LogicalState 或文件；预览完成后的保存冲突/失败保留目标 LogicalState 和标签，并且不得覆盖既有目标。
- 来源是转换时刻的冻结输入：其后续变化不回写或自动刷新已预览的目标副本；目标自身的后续编辑按 D-NEW 使原转换提案失效。
- 具体转换算法、对象字段、卡片类型、状态枚举、错误码和 Markdown 语法降级规则属于后续 DESIGN Scope。

## 4. 直接消费者

- CORE：REQ-CORE-013 作为产品级最小格式转换与复制新建根要求。
- AG：REQ-AG-012 对明确转换请求只产生受控转换提案，不扩大工具授权。
- DE：REQ-DE-006 承接转换目标的新建提案预览、裁决和失效生命周期。
- DOC：REQ-DOC-006 承接文本与 Markdown 的本地转换语义、损失告知和 BDocx 边界。
- ED：REQ-ED-014 承接转换目标的未保存标签、可继续编辑和卡片失效呈现。
- WS：REQ-WS-009 承接目标副本的显式保存、来源不变和保存失败保留。

## 5. 验收计划

- 两个方向分别覆盖：来源状态选择、转换提案信息、预览创建目标标签、接受、拒绝、目标继续编辑、保存与重开。
- `TXT → Markdown` 不得把普通文本擅自转换为标题、列表或任务；`Markdown → TXT` 必须可见告知有损边界。
- 转换前失败不产生目标；保存冲突/失败不覆盖既有文件、不丢目标逻辑内容；来源内容和来源 DiskState 保持不变。
- HTML、通用 DOCX、批量和原地转换不得因这项能力获得可执行入口；BDocx 仍遵循 R3 专用路径。
- 本 Scope 不运行代码测试；实现、格式 Codec 与工具授权均属于后续 DESIGN/IMPLEMENTATION Scope。

## 6. 待设计项

- 转换 Proposal 的共享对象、源状态快照、目标未保存元数据、卡片/Trace 关联、格式检测、保存状态机和重启恢复。
- TXT/Markdown 的编码、换行、空白与具体 Markdown 语法降级/序列化规则。
- 目标路径建议、冲突检测时机、重命名交互和时间轴的关联。
- HTML 互转与通用 DOCX 转换的语义子集、资源和样式保真、安全净化策略、工具与验证门槛。

## 7. 文件范围结论

允许修改：CORE 产品根与需求索引、能力基线、AG/DE/DOC/ED/WS 的直接 L1 需求、本 Issue Trace 与 active scope。

禁止修改：REF、L2 技术设计/状态机/接口、代码、测试、脚本、APC、ADUS 和治理配置；现有 BDocx 处理链只作为边界
被引用，不能在本轮重新设计或扩展。

## 8. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`REQUIREMENT`
- final_state：`READY`
- changed_files：`A-CORE-C-D-01` 产品根、`A-CORE-X-D-01` 需求索引、`A-AG-M-D-02` 能力基线、AG/DE/DOC/ED/WS 五份直接 L1 需求文档、本 Issue Trace 与 active scope。
- scope：人类已确认 R4-C 的最小格式矩阵与复制新建生命周期；active scope 只列出直接 L1 消费者和治理留痕文件。
- phase_gate：G1 REQUIREMENT 进行中；未进入 DESIGN 或 IMPLEMENTATION。
- mapping_chain：不适用；本轮不修改 REQ_MAP、RULE、CHAIN、CONSTRAINT 或 TERM。
- test_impact：不适用；本轮无代码或测试改动。
- validation：已逐段复核 CAP-13、CORE/AG/DE/DOC/ED/WS 的目标副本、来源冻结、LogicalState、接受不写盘、显式保存、目标编辑失效、失败保留和 HTML/通用 DOCX 排除边界；Node 只读检查 Scope 的 10 个直接文件均在允许范围；CORE 13 条、AG 12 条、DE 6 条、DOC 6 条、ED 14 条、WS 9 条 REQ 均具备本层必填字段且 60 个 req_id 无重复；`git diff --check` 对已跟踪 Scope 文件无输出；包含未跟踪文档在内的尾随空白扫描无命中。
- automated_tests：不适用。
- unresolved_items：见 §6；R4-D、R5 与后续 DESIGN Scope 仍待完成。
