---
文档编号：DOC-M-T-01
文档状态：A
负责模块：DOC
文档职责：DOC 文档转换模块级 L2 技术设计（规则 SSOT）——docx 双向转换管线（编辑链 Pandoc / 预览链 LibreOffice，双链分离）、样式子集三级执行契约、blockId 转换层兜底、本地转换运行时边界；承接 REQ-DOC-001~005
上游约束：A-DOC-M-D-01_文档转换功能需求描述.md（REQ-DOC-001~005、DOC-01~05）、A-CORE-C-P-01_APC.md（§4.1、§6、§9）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-DATA-002 分工锁定、§11.1 样式子集与 blockId 选型记录）、A-WS-M-T-01_工作台技术设计.md（BR-WS-PERSIST-001 原子落盘、X-FORBID-004）、A-SYS-I-P-02_IPC协议.md（统一错误形状）、A-SYS-M-S-01_状态机总图.md（docx 开/存核心链、X-SEQ-003/X-INTER-001）、ADU.md（§3 格式）
直接承接：本文 RULE/CHAIN/CONSTRAINT/TERM 为 DOC 规则 SSOT，代码经 @GOV 映射；SM-DOC-01 在 A-DOC-M-S-01 定义并登记总图；样式子集契约被 ED（编辑模型承载）与 PRV（预览补偿）消费
使用边界：本文是 DOC 规则 SSOT；不定义编辑模型内部（归 ED）、不定义预览呈现（归 PRV）、不做文件落盘（经 WS）、不锁定 Pandoc/LibreOffice 具体版本与 Lua 过滤器实现（延实现期选型记录）、状态机内部归 A-DOC-M-S-01
变更要求：样式子集三级清单变更须同步 ED（编辑模型承载）与 PRV（预览补偿）复核；转换链分工变更须经架构评审；TERM 一经被 term_ref 引用其 en 不可更改
治理成熟度：draft
---

# Binder 文档转换技术设计（DOC · L2 技术设计）

## 0. Terminology Registry

<!-- TERM
term_id: TERM-DOC-001
chains: DOC-CONVERT
zh: 样式子集
en: StyleSubset
forbidden: 样式白名单, 格式子集, 样式范围, 支持样式表
-->
docx 进入编辑模式时编辑模型所承载的样式范围三级契约：必需子集（承诺往返保真）、部分支持（尽力
保留并明示）、明确不处理（不承诺，预览链全保真补偿）；进入编辑即子集化且对用户可知（REQ-DOC-003）。

## §1 文档定位

```
REQ-CORE-004 → REQ-DOC-001~005（DOC L1）→ 本文 RULE/CHAIN/CONSTRAINT/TERM（SSOT）→ 代码(@GOV)
本文 ←分工对偶← A-ED-M-T-01（BR-ED-DATA-002：转换归 DOC、编辑归 ED）
本文 →驱动→ A-DOC-M-S-01（SM-DOC-01 docx 开/存机）→登记→ A-SYS-M-S-01 总图
本文样式子集契约 →供→ ED（编辑模型承载）/ PRV（预览补偿）消费
```

技术方案采纳旧版实证（R-ED-M-R-01 v2.0 终案 + R-ED-M-R-23，台账 §4.5 P1 原料，按 APC §8 重新
声明）：**编辑链 Pandoc、预览链 LibreOffice→PDF、双链完全分离**。Pandoc/LibreOffice 具体版本与
Lua 过滤器实现延实现期（§9）。

## §2 转换管线架构（双链分离）

```
打开编辑   docx ──Pandoc(+Lua 过滤器)──→ HTML/统一编辑模型 ──注入缺失 data-block-id──→ ED 装载
保存往返   ED 编辑模型(HTML) ──Pandoc──→ docx 字节流 ──经 WS 原子落盘──→ 文件
只读预览   docx ──LibreOffice──→ PDF（全保真）──→ PRV 呈现
```

- 编辑链与预览链**完全分离**：无共享中间态、无功能黏连；预览链失败不影响编辑链，反之亦然。
- 转换全程**本地执行**（Pandoc/LibreOffice 本地依赖），离线可用，内容不上行（REQ-DOC-005）。
- 文件读写经 WS（X-FORBID-004）；转换中间产物在 workspace 管理边界内。

## §3 转换通道与生命周期（DOC-CONVERT）

<!-- CHAIN
chain_id: DOC-CONVERT
主责模块: DOC
status: active
risk_tier: P1
-->

<!-- RULE
rule_id: BR-DOC-DATA-001
主链路: DOC-CONVERT
域: DATA
-->
docx 等二进制文档格式的解析与生成必经 DOC 转换管线（编辑链 Pandoc / 预览链 LibreOffice），
其他模块不得内置二进制格式处理路径（与 BR-ED-DATA-002 对偶；X-SEQ-004）；编辑链与预览链完全
分离，不得共享中间态或互相依赖；转换全程本地执行，文档内容不得因转换经网络上行；转换依赖缺失时
回统一错误形状明确报错，不静默降级。

<!-- RULE
rule_id: BR-DOC-STATE-001
主链路: DOC-CONVERT
域: STATE
-->
转换生命周期必须满足：打开转换失败回统一错误形状且不建立编辑会话（对接 SM-ED-01 的
parseFail/loadFail）；保存转换失败或落盘失败时原 docx 文件保持不变（原子性经 WS
BR-WS-PERSIST-001 保障），无半成品落盘；转换中断不残留破坏性中间产物；失败可重试。
状态转移归 SM-DOC-01（A-DOC-M-S-01）。

## §4 样式子集三级执行契约（DOC-CONVERT）

承 A-ED-M-T-01 §11.1 选型记录（旧版 R-ED-M-R-23 实证复现率），本文为执行 SSOT：

| 级别 | 清单 | 契约 |
|---|---|---|
| 必需子集 | 标题 H1-H6、段落对齐、粗体/斜体/下划线、有序/无序列表、基础表格、嵌入图片 | 承诺往返保真（实证 ≥95%），回归测试强制覆盖 |
| 部分支持 | 文本颜色、字体、字号 | 尽力保留（Pandoc+Lua 实证 30–50%），损失对用户明示 |
| 明确不处理 | 页眉页脚、文本框、SmartArt、VBA 宏、复杂样式继承链、合并单元格等复杂表格格式 | 不承诺；预览链（LibreOffice→PDF 全保真）补偿查看需求 |

<!-- RULE
rule_id: BR-DOC-DATA-002
主链路: DOC-CONVERT
域: DATA
-->
docx 进入编辑模式即按三级样式子集契约子集化：必需子集必须往返保真且回归测试强制覆盖；部分支持
项尽力保留、其损失必须对用户可知（不静默丢失）；明确不处理项清单公开且由预览链全保真补偿。
三级清单的变更必须回写本文并同步 ED（编辑模型承载范围）与 PRV（预览补偿）复核，不得在转换代码
中私自增减子集。

## §5 blockId 转换层兜底（DOC-CONVERT）

承 A-ED-M-T-01 §11.1：转换层在 docx→编辑模型方向为缺失 `data-block-id` 的块级元素注入
blockId（兜底，保障 DE/REF 锚定可用）；编辑模型→docx 方向尽力保留 `data-block-id`，
经 Pandoc 往返的保留度实现期实测（DOC-02），不达则按"往返后兜底重建锚"降级并回写契约。

<!-- RULE
rule_id: BR-DOC-DATA-003
主链路: DOC-CONVERT
域: DATA
-->
docx→编辑模型转换必须为缺失 data-block-id 的块级元素注入 blockId 兜底（生成规则与 ED blockId
机制一致，A-ED-M-T-01 §11.1），保障上层锚定不因转换缺锚；编辑模型→docx 转换尽力保留
data-block-id；往返保留度不达标时必须显式采用"重建锚"降级契约并回写本文与 A-ED-M-T-01，
不得静默让锚漂移。

## §6 跨模块约束（CONSTRAINT）

<!-- CONSTRAINT
constraint_id: X-SEQ-004
摘要: docx 等二进制文档格式的解析/生成必须经 DOC 转换管线，任何模块不得内置二进制文档转换库或另立转换路径
涉及模块: DOC, ED, WS, PRV, CBT, AG
链路影响: DOC-CONVERT, ED-EDIT
-->

**复用既有约束**：X-FORBID-004（文件路径限界）、X-SHARED-001（跨模块结构经 SYS）、
X-SEQ-003（SM-DOC-01 先建机）、X-INTER-001（DOC↔ED、DOC↔WS 转移点登记总图）。

## §7 需求到规则映射（REQ_MAP）

<!-- REQ_MAP
req_id: REQ-DOC-001
target_type: CHAIN
target_id: DOC-CONVERT
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-001
target_type: RULE
target_id: BR-DOC-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-002
target_type: RULE
target_id: BR-DOC-STATE-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-002
target_type: RULE
target_id: BR-DOC-DATA-003
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-003
target_type: RULE
target_id: BR-DOC-DATA-002
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-004
target_type: RULE
target_id: BR-DOC-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-005
target_type: RULE
target_id: BR-DOC-DATA-001
status: mapped
-->
<!-- REQ_MAP
req_id: REQ-DOC-005
target_type: CONSTRAINT
target_id: X-SEQ-004
status: mapped
-->

## §8 测试与验收

- **唯一通道与双链分离**（BR-DOC-DATA-001 / X-SEQ-004）：除 DOC 外无二进制文档处理代码（静态可查）；
  预览链失败不影响编辑链；断网时开/存/预览转换可用；无文档内容网络上行（可经网络核验）。
- **必需子集往返保真**（BR-DOC-DATA-002，REQ-DOC-003 验收）：标题/对齐/粗斜下/列表/基础表格/图片
  的 docx→编辑→docx 往返回归测试强制覆盖；部分支持项损失有用户可见明示。
- **转换原子性**（BR-DOC-STATE-001 / SM-DOC-01）：注入打开/保存转换失败，会话不建立/原文件不变、
  无半成品、可重试。
- **blockId 兜底**（BR-DOC-DATA-003）：转换后所有块级元素有 blockId；往返保留度实测并记录
  （DOC-02）。
- 量化 NFR（首开 <12s / 缓存后 <2s / 准确率 ≥90%，旧版目标）延 G5 标定复核（DOC-05）。

## §9 技术债务与盲区（诚实声明）

- **Pandoc / LibreOffice 版本与 Lua 过滤器集未锁定**（DOC-01）：旧版实证方向已锁（双链分离），
  具体版本、过滤器实现、打包体积与按需加载延实现期选型记录。
- **data-block-id 往返保留度未实测**（DOC-02）：本文已定降级契约（重建锚），实测后回写。
- **Excel/PPT/音视频预览转换归属**（DOC-03）：**已随 PRV G2 联合解除（2026-06-11，A-PRV-M-T-01
  §2）**——Excel/PPT 预览转换归 DOC 预览链扩展（LibreOffice→PDF，受 X-SEQ-004 约束）；音视频
  系统解码归 PRV（非文档转换职责）。Excel/PPT 保真度实现期实测，不达另议须回写两文。
- **样式子集告知交互形态**（DOC-04）：延实现期人类确认，本文只锁"用户可知"语义。
- 转换缓存策略（旧版 file_cache + content_hash 模式可取材）延实现期，与 WS 控制态缓存协同设计。

## §10 后续动作

1. 落地后随 `npm run governance:generate` 注册：DOC-CONVERT 链路、BR-DOC-DATA-001/002/003、
   BR-DOC-STATE-001 规则、X-SEQ-004 约束、TERM-DOC-001、REQ_MAP 进 ADUS。
2. 续 A-DOC-M-S-01（SM-DOC-01 docx 开/存机），登记总图（docx 行 planned→active）与
   X-INTER-001 转移点（DOC↔ED 装载、DOC↔WS 落盘）。
3. 待人类宣布 DOC G2 闭合；PRV G1 时联合解除 DOC-03。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（DOC G2 首件）：DOC 规则 SSOT——双链分离转换管线（编辑链 Pandoc / 预览链 LibreOffice→PDF，旧版 R-ED-M-R-01 v2.0 实证采纳）、转换生命周期与原子性、样式子集三级执行契约（R-ED-M-R-23 实证清单）、blockId 转换层兜底与往返降级契约；注册链路 DOC-CONVERT（P1）、规则 BR-DOC-DATA-001/002/003·BR-DOC-STATE-001、约束 X-SEQ-004、术语 TERM-DOC-001 StyleSubset；REQ_MAP（REQ-DOC-001~005）；债务（版本/过滤器/往返实测/PRV 归属/告知形态延后记明） |
| 2026-06-11 | v1.1 | DOC-03 解除（随 PRV G2 联合，A-PRV-M-T-01 §2）：Excel/PPT 预览转换归 DOC 预览链扩展（LibreOffice→PDF，X-SEQ-004 覆盖）、音视频系统解码归 PRV；§9 债务更新；保真度实测留实现期 |
