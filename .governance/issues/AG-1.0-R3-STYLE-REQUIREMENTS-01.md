# Issue Trace — AG-1.0-R3-STYLE-REQUIREMENTS-01：HTML/BDocx 样式子集与字体族边界

文档状态：A
任务类型：DOC_ONLY
治理阶段：REQUIREMENT
创建时间：2026-08-14

## 1. 用户可见问题

R3 的样式矩阵此前保留为“待确认”，会使 1.0 的编辑范围无法稳定：用户无法知道哪些样式应实现，开发也无法区分“未支持”和“尚未裁决”。其中，字体族切换此前只建议 HTML 支持，未纳入 BDocx 编辑副本。

## 2. 本轮人类确认

1. 上轮样式矩阵中，凡未明确写为“不纳入 1.0”的操作，均是 1.0 要实现的受控编辑需求。
2. “需验证”只表示代码实现后必须使用真实样例验证，不表示在验证前仍未决定是否支持实现。
3. 字体族切换同时属于 HTML 与 BDocx 处理路径的 1.0 实现范围。
4. 其余上轮矩阵项按原建议档位确认：字符基础样式为 P0；颜色/突出显示、段落对齐、间距与缩进、字号、字体族切换为 P1；页面级布局、页眉页脚、复杂样式继承、任意定位不纳入 1.0。

## 3. 需求边界

- 本轮只确认用户可见操作、格式适用范围、P0/P1 档位和验证含义；不定义 CSS、OOXML、Pandoc、TipTap、工具参数或能力开关的技术实现。
- BDocx 是处理路径，不是格式。字体族切换发生在 BDocx 编辑副本的 HTML 编辑模型上，保存目标仍是新 DOCX 副本。
- 受控编辑只允许结构化样式操作；不允许 Agent 直接提交原始 HTML、CSS 或 OOXML。
- 真实验证是上线前置门槛：每项已确认能力实现后，必须以对应格式执行保存、重开和结果核对；未验证不得在产品中宣称已可用。

## 4. 直接消费者

- AG：能力基线和编辑意图只生成已确认样式的受控 Proposal。
- DE：样式 Proposal 的 Diff 必须可呈现、裁决和失效。
- ED：HTML 与 BDocx 编辑副本暴露相同的已确认操作边界，字体族切换不因处理路径而被排除。
- DOC：BDocx 副本的样式保存往返按操作逐项验证；原生 DOCX 继续只读。
- CORE 需求索引：记录 R3 样式项不再待确认，并保持 R3 之外的需求不变。

## 5. 验收计划

- CAP-06、格式矩阵与 AG/DE/ED/DOC 的直接 L1 描述对 P0/P1、处理路径和排除项一致。
- “需验证”被表述为实现后的真实验证门槛，不能被解释为撤销已确认的实现范围。
- 字体族切换在 HTML 与 BDocx 两列均为 P1 受控实现，且不承诺原生 DOCX 编辑。
- 每份变更的 A 状态文档保留文档头和新增变更记录。
- 本 Scope 不运行代码测试；技术映射和测试矩阵进入后续 DESIGN Scope。

## 6. 待设计项

- 每个 StyleOp 的操作码、目标范围、参数值域、能力开关及其 HTML/OOXML 映射。
- BDocx 字体可用性、缺失字体替代、保存重开保真等级与用户告知。
- 样式 Diff 的可读表达、失败形状和状态机/测试矩阵。
- 原始来源中未进入已确认子集的样式在创建 BDocx 编辑副本时的展示与告知策略。

## 7. 文件范围结论

允许修改：能力基线、需求索引与 AG/DE/ED/DOC 的直接 L1 需求文件，以及本 Issue Trace、active scope。它们分别定义能力档位、用户可见编辑、提案审阅、编辑器边界和 BDocx 转换承诺。

禁止修改：CORE 产品概述、WS、PRV、所有 L2 设计/状态机/接口、代码、测试、脚本、APC、ADUS 和治理配置；它们不需要为了本轮需求裁决而改变，或属于后续设计/实现阶段。

## 8. Task Runtime Report

- task_type：`DOC_ONLY`
- governance_phase：`REQUIREMENT`
- final_state：`READY`
- changed_files：`A-CORE-X-D-01` 需求索引、`A-AG-M-D-02` 能力基线、AG/DE/ED/DOC 的四份直接 L1 需求文档、本 Issue Trace 和 active scope。
- scope：隐式 DOC_ONLY；为延续工作区的范围审计，active scope 仅列出本轮文件，且人类已明确确认样式矩阵。
- phase_gate：已在 G1 REQUIREMENT 内完成需求确认；未进入 DESIGN 或 IMPLEMENTATION。
- mapping_chain：不适用；本轮不修改 REQ_MAP、RULE、CHAIN、CONSTRAINT 或 TERM。
- test_impact：不适用；本轮无代码或测试改动。
- validation：逐文件检查 P0/P1、HTML/BDocx 字体族、样式操作集合与“真实验证”语义；检查 REQ 边界和变更记录；`git diff --check` 对已跟踪 Scope 文件无输出；包含未跟踪文档在内的尾随空白扫描无命中；`jq` 不可用后以 Node 只读解析 active scope JSON 成功；当前正文无“待 R3 样式确认”遗留。
- automated_tests：不适用。样式功能尚未实现；本轮不把需求确认表述为代码或系统验证已通过。
- unresolved_items：StyleOp 的技术映射、字体可用性/替代、BDocx 保存重开保真、用户告知、能力开关、异常形状与测试矩阵，均待后续 DESIGN Scope；R4~R5 的产品需求仍待确认。
