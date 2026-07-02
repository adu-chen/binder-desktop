---
文档编号：ED-X-L-02
文档状态：A
负责模块：ED
文档职责：ED 编辑器专项优化计划（L3 承接型 Plan Item）——承接 2026-07-03 人类提出的 12 项优化需求 + 10 项发掘项与三项裁定（.html 富文本入范围 / 会话内字符级锚定 / 样式写路径 A 默认 + B 限内联）；定义文档先行路由（新建 vs 并入）、REQ/RULE/TERM 注册清单、PI-ED-06~13 与批次顺序
上游约束：A-CORE-C-P-01_APC.md（§5 门禁、§6 状态机阈值）、A-ED-M-D-01（REQ-ED-*，序号不复用）、A-ED-M-T-01（ED 规则 SSOT）、A-ED-I-P-01、A-ED-X-L-01（G3 建设计划，本计划为其后续专项）、A-CORE-X-L-03（PI-FE-31 先例：视图层不注册 REQ）、A-CORE-X-R-03（审计与补全上下文）、ADU.md（§3 REQ/RULE/TERM 规范）
直接承接：本计划驱动 ED 专项优化实现；契约批（批B）各 PI 的**文档修订动作先于代码**（文档先行）；@GOV 追溯 BR-ED-*（含本计划注册的新规则）
使用边界：L3 承接型 Plan Item；本文不定义 RULE/CHAIN/TERM 正文（注册动作在各 SSOT 文档执行，本文只列清单与去向）；代码不得追溯本文；视图层 PI 沿 PI-FE-31 先例不注册 REQ/@GOV 规则
变更要求：PI 增删保持承接可追溯；三项人类裁定（§2）变更须重审批B 全部 PI
治理成熟度：draft
---

# Binder 编辑器专项优化计划（ED · L3 专项）

## §1 输入与定位

**输入**：2026-07-03 人类逐条提出的 12 项需求（#1~#12）+ 分析发掘的 10 项（N1~N10）+ 三项裁定。
现状核实与逐条分析见会话记录（2026-07-03 需求分析轮）；实现完整度背景见 A-CORE-X-R-03。

**与既有计划的关系**：A-ED-X-L-01（PI-ED-01~05）为 G3 模块建设计划，其承接项已基本落地
（会话/blockId/统一模型/分页媒体/装饰层）；A-CORE-X-L-03 PI-FE-31 为编辑器核心优化第一批
（分页/图文/工具栏/布局/缩放）。本计划为**第二阶段专项**，PI 编号续 PI-ED-06。

## §2 人类裁定（2026-07-03，批B 前提，变更须重审）

| 裁定 | 结论 |
|---|---|
| D-1 需求#1 的 html 范围 | **.html 独立文件富文本编辑入范围**（新格式接入，REQ-ED-008 / PI-ED-13） |
| D-2 字符级锚定强度 | **本期锁会话内字符级锚定**；跨会话字符锚（内容哈希重建）显式延后登记债务 |
| D-3 agent 样式写路径 | **A 默认 + B 限内联**：chat/agent 回合样式修改走 DE diff 裁决（DiffPayload 扩样式 op + DiffCard 样式呈现）；用户显式触发的内联辅助样式操作视为用户编辑直改（BR-AG-STATE-003 语义扩展） |

## §3 文档先行路由总表

### 3.1 需求描述层（L1）——全部并入，不新建

| 文档 | 动作 | 内容 |
|---|---|---|
| **A-ED-M-D-01** | 修订（追加 REQ + 变更记录） | 新增 REQ-ED-006（字符级定位与选区引用）、REQ-ED-007（agent 可读可编辑的样式协议）、REQ-ED-008（.html 富文本编辑接入）；§2 职责表/§3 能力范围同步 |

> 并入理由：一模块一 M-D 惯例；REQ 序号不复用可追加（文档头变更要求明文）。
> **视图层项不注册 REQ**（#4/#5/#7/#9/#10/#11/#12、N3/N5/N7）：沿 PI-FE-31 先例与
> A-SYS-C-T-02 §9「UI 控件级贴 G4 just-in-time」，直接进本计划 PI；#8 为既有 BR-ED-DATA-003
> 范围内的缺陷修复，不需新需求。AG/DE/REF 的 L1 无需变更（消费端为 L2 契约细化）。

### 3.2 设计层（L2 / I-P）——全部并入，不新建

| 文档 | 动作 | 内容 |
|---|---|---|
| **A-ED-M-T-01**（ED 规则 SSOT） | 修订 | ① §4 扩展字符级锚定语义 → 注册 **BR-ED-DATA-004**；② 新节样式投影与操作 → 注册 **BR-ED-DATA-005**；③ **BR-ED-DATA-002 规则文本修订**（统一模型 md/docx → md/docx/**html**）；④ Terminology Registry 注册 TERM-ED-004/005/006；⑤ REQ_MAP 补三条；⑥ §11 登记跨会话字符锚延后债务 |
| **A-ED-I-P-01** | 修订 | `ed.locate_block` 字符级扩展（回 `{blockId,start,end}` 定位结果）；新增 `ed.get_cursor_ref` / `ed.get_selection_ref`（按需查询，非常驻投影）；`ed.apply_edit` **op 语义粒度表**（正式承接 §7 既有债务：文本 op + 样式 op）；样式投影读接口 |
| **A-AG-M-T-01 / A-AG-I-P-01** | 修订（⚠️ 协调并行 AG 窗口） | BR-AG-STATE-003 语义扩展（D-3：内联样式操作=用户编辑直改）；`ag.list_blocks` 扩结构化样式投影（`{blockId,type,text,marks[],blockAttrs}`）；选区引用消费契约 |
| **A-DE-M-T-01 / A-DE-I-P-01** | 修订 | D-3 方案 A：DiffPayload（TERM-DE-004）扩样式 op 载荷；DiffCard 样式 diff 呈现语义；锚定粒度从块级扩展至可含字符区间 |
| **A-REF-M-T-01** | 修订 | 位置引用（选区拖拽产物）作为 ReferenceAnchor（TERM-REF-001）子类型进入注入协议——不允许 AG 私有引用格式（X-SHARED-001 精神） |
| **A-WS-I-P-01** | 修订（仅当 N4 排期） | `ws.import_media`（本地图片经 WS 写入 workspace，X-FORBID-004 守界） |
| A-ED-M-S-01 / A-DOC-M-T-01 | **无需变更** | #7 加载态投影既有 SM-DOC-01 广播（X-FORBID-007 正统）；.html 为文本格式不经 DOC 二进制管线（X-SEQ-004 不涉及） |

### 3.3 计划层（L3）——新建本文档

新建理由：跨 ED/AG/DE/REF 四模块、三批次、含契约修订的专项，独立于已履约的 A-ED-X-L-01
与单批体例的 A-CORE-X-L-03。

## §4 注册项清单（在各 SSOT 执行，此处为清单）

**REQ（A-ED-M-D-01）**：
- `REQ-ED-006` 字符级定位与选区引用——编辑器提供会话内字符级内容锚（blockId + 块内纯文本偏移）与光标/选区位置查询，供 agent 精确编辑定位与位置引用生成；锚失效可感知
- `REQ-ED-007` agent 可读可编辑的样式协议——文档样式（字符 marks / 块属性）以机器可读结构投影，样式修改经受控操作面进入（写路径按 D-3 分流）
- `REQ-ED-008` .html 富文本编辑接入——独立 .html 文件加入统一编辑模型（富文本编辑 + blockId + 序列化往返）

**RULE（A-ED-M-T-01，REQ_MAP 同步）**：
- `BR-ED-DATA-004`（新）：字符级锚定契约——偏移定义在**块内纯文本平面**（跨格式统一）；会话内有效（D-2）；锚失效回明确信号不静默错位；跨模块传递结构经 SYS 契约层（X-SHARED-001）。REQ_MAP：REQ-ED-006
- `BR-ED-DATA-005`（新）：样式协议——样式投影机器可读；样式修改必经 ED 受控接口（X-FORBID-005）；agent 样式落盘按 D-3 分流（回合产出→DE diff，显式内联→用户编辑）。REQ_MAP：REQ-ED-007
- `BR-ED-DATA-002`（修订）：统一编辑模型扩展至 md/docx/**html** 三格式。REQ_MAP 补：REQ-ED-008
- DE/AG 侧：BR-AG-STATE-003 修订（AG SSOT）、DE 载荷规则语义扩展（DE SSOT）——各归其主责文档，不在 ED 注册

**TERM（A-ED-M-T-01 Terminology Registry）**：
- `TERM-ED-004` 字符锚 / **CharAnchor**（禁用：字符偏移锚, 文本锚, 偏移量锚, inline锚）——基础锚
- `TERM-ED-005` 选区引用 / **SelectionRef**（禁用：选区信息, 拖拽引用, selection数据）——派生于字符锚
- `TERM-ED-006` 样式操作 / **StyleOp**（禁用：样式指令, 格式操作, style命令, 样式patch）

**不注册**：视图层批全部（#4/#5/#7/#8/#9/#10/#11/#12、N3/N5/N7/N8）——无新规则语义，
#8/#9 在 BR-ED-DATA-003 既有语义内，@GOV 沿用既有块。

## §5 Plan Item 清单

### 批A　视图与修复批（无契约依赖，可立即开工）

#### PI-ED-06　分页页高修复 + 纸张选择（#8 / #9 / N8）
- **承接**：BR-ED-DATA-003（既有）；根因已确认——分页缝紧跟页末块、未补足页尾留白，页高=内容累计高（≤预算）。
- **文件范围**：`ed/extensions/Pagination.ts(+test)`（breaks 扩为 `{index,fillPx}[]`，gap widget 加 filler；末页撑满整页）、纸张预设表（A4/A3/A5/Letter × 横竖，取材旧项目 constants）、纸张下拉 UI、`edViewPrefsStore`（per-doc localStorage 视图偏好，纸张项）。
- **验收**：非末页视觉高度恒等 A4（或所选纸张）比例（纯逻辑不变量单测：used+fill=预算）；纸张/方向切换即时重排、不改模型不产脏态；IME 组合输入期间分页重测量不打断输入（N8，真机验证）。

#### PI-ED-07　编辑器工作区体验批（#4 / #5 / #7 / #10 / #11 / #12 / N7）
- **承接**：视图层（无规则注册）；BR-ED-STATE-003 呈现位置调整（语义不弱化）。
- **文件范围**：`EditorStatusBar.tsx`（新：字数/行数/光标位/脏态/保存态；接 @tiptap/extension-character-count）；冲突条重构为状态栏项+弹出双显式按钮；`tabsStore.reorder(+test)` + TabStrip 指针拖拽排序 + 滚动条隐藏 + 纵轮转横滚 + 激活标签 scrollIntoView；工具栏两排固定分组 + 溢出滚动；zoom 迁入 `edViewPrefsStore`（per-doc 持久）+ ctrl/pinch 滚轮缩放；SM-DOC-01 OpenConverting→ 会话 loading 呈现态 + 骨架/不确定进度；每标签滚动位置暂存。
- **验收**：状态栏各项实时正确；冲突裁决仍双向显式（BR-ED-STATE-003 不弱化）；标签拖动排序/无滚动条；工具栏窄窗口尾部可滚出；缩放跨切换保留；大 docx 打开有即时反馈。

#### PI-ED-08　表格操作 UI + 图片交互（N3 / #3 图片部分）
- **承接**：视图+用户编辑（改模型 attrs 产脏态，合法，非装饰层）。
- **文件范围**：表格浮动控件/二级菜单（addRow/Column/delete/toggleHeader，TipTap 命令现成）；图片自定义 NodeView（选中手柄拖拽调宽改 `width` attr、inline/block 嵌入切换）。
- **验收**：表格行列增删可用；图片可拖宽、可切嵌入样式，改动产生脏态且 ⌘S 往返（宽度经 docx 往返保留度与 DOC 联合实测，不达登记降级）。

### 批B　契约批（文档修订先行 → 实现；D-1/2/3 为前提）

#### PI-ED-09　blockId 跨 docx 往返实测（N6，批B 地基）
- **承接**：A-ED-M-T-01 §11.2 既有债务；BR-ED-DATA-001。
- **动作**：实测 Pandoc 管线对 `data-block-id` 的保留度（含 preserve-styles.lua 是否需扩展）；不达 → 启用转换层兜底注入（旧项目 `inject_missing_data_block_ids` 模式，归 DOC）。结果回填 A-ED-M-T-01 §11.2。
- **验收**：docx 打开→编辑→保存→重开，既有块 id 保留率实测数据落文档；字符级契约（PI-ED-10）以此为地基。

#### PI-ED-10　字符级锚定与选区引用（#1 / #2 / #6）
- **文档先行**：§3.2 之 A-ED-M-D-01（REQ-ED-006）+ A-ED-M-T-01（BR-ED-DATA-004 + TERM-ED-004/005）+ A-ED-I-P-01 + A-REF-M-T-01 修订，**经人类确认后**进代码。
- **文件范围**：`edLocate.ts` 升级（SelectionRef 含 `{blockId,start,end,text,path}`；偏移↔PM position 换算；`getCursorRef` 按需查询）；选区 dragstart 携带 `application/x-binder-selection-ref`（拖出编辑器边界才转引用语义，器内保持原生移动）；Composer drop → 位置引用 chip（REF 协议格式）。
- **@GOV**：BR-ED-DATA-004（锚定纯逻辑 + 换算）。
- **验收**：agent 经 `{blockId,start,end}` 可精确取址目标字符区间（四格式统一纯文本平面）；锚失效回明确信号；选区拖入 Composer 生成含位置的引用；纯逻辑（偏移换算/失效判定）单测。

#### PI-ED-11　样式协议（#3 协议部分；DE/AG 联动）
- **文档先行**：A-ED-M-D-01（REQ-ED-007）+ A-ED-M-T-01（BR-ED-DATA-005 + TERM-ED-006）+ A-ED-I-P-01（op 粒度表）+ A-DE-*（样式载荷/呈现）+ A-AG-*（BR-AG-STATE-003 扩展，**协调 AG 并行窗口**），经人类确认后进代码。
- **文件范围**：读侧 `ag.list_blocks` 样式投影；写侧样式 op 集（set_mark/unset_mark/set_block_attr per CharAnchor 区间）经 `ed.apply_edit`；D-3 分流——回合产出样式 op 进 DiffPayload（DiffCard 扩样式呈现），内联辅助样式直改。
- **@GOV**：BR-ED-DATA-005（ED 侧）；DE/AG 侧各归其规则。
- **验收**：agent 读到结构化样式；样式 diff 卡可裁决落盘；内联样式操作即改即脏；X-FORBID-006 语义核验（回合产出无直改路径）。

#### PI-ED-12　大纲面板（N1）
- **承接**：视图层；吃 PI-ED-10 定位底座（heading 树 + blockId 跳转）。旧项目 OutlinePanel 参考。
- **验收**：大纲随文档实时更新、点击精确跳转、长文档可用。

### 批C　独立/低优批

#### PI-ED-13　.html 富文本编辑接入（D-1 / REQ-ED-008）
- **文档先行**：A-ED-M-D-01（REQ-ED-008）+ A-ED-M-T-01（BR-ED-DATA-002 修订至三格式）。
- **文件范围**：`Editor.tsx`/`edService.ts` 格式路由（.html → 富文本 editFormat，读写仍经 ws.read_doc/save_doc 文本路径，**不经 DOC**——非二进制，X-SEQ-004 不涉及）；序列化 getHTML 往返；blockId 经 data-block-id 落 HTML（.html 源可承载，与 md 不注入策略区分声明）。
- **验收**：.html 打开为富文本、编辑保存往返保真、blockId 跨保存稳定（.html 可落盘 id，锚定强于 md）。

**候补清单（不开 PI，待人类排期）**：N2 编辑器内查找/替换；N4 本地图片插入（需 A-WS-I-P-01 增
`ws.import_media`）；N5 链接编辑弹层（随 #11 顺手）；N9 编辑器分页 vs PRV 打印分页差异说明（文档
动作）；N10 自动保存（产品决策，与基线冲突模型有张力，默认不做）。

## §6 实施顺序

```
批A：PI-ED-06（bug 优先）→ PI-ED-07 → PI-ED-08          ——无契约依赖，可即刻推进
批B：PI-ED-09（地基实测）→ 文档修订（§3.2，人类确认）→ PI-ED-10 → PI-ED-11 → PI-ED-12
批C：PI-ED-13 → 候补项按人类排期
```

排序逻辑：批A 含已上线功能缺陷（#8）与纯视图快赢；批B 是「agent 精确编辑」主线，**契约先行**
（否则 ED/AG/DE/REF 各自拼装全线返工），PI-ED-09 实测不达则 PI-ED-10 锚定地基改走兜底注入。

## §7 A 类债务与显式延后登记

| 项 | 处置 |
|---|---|
| 跨会话字符级锚定（内容哈希+diff 重建） | D-2 显式延后；登记 A-ED-M-T-01 §11（PI-ED-10 文档动作含此） |
| 图片宽度/嵌入样式的 docx 往返保留度 | PI-ED-08 与 DOC 联合实测，不达登记降级 |
| 分页视觉 vs PRV 打印分页差异 | N9 候补（说明性文档动作） |
| 大文档虚拟化/15MB 上限 | 既有债务不变，G5 NFR |

## §8 协调点（并发窗口纪律）

- **AG 文档/代码修订（PI-ED-11）**：PI-AG-WIRE-01 由并行窗口推进中——AG 侧文档修订与
  `ag.list_blocks` 扩展**须与该窗口协调时序**，本计划 ED 侧不单方面改 AG 文件。
- `src-tauri/src/lib.rs` 为最高冲突点：批B 涉及 command 扩展时先与人类确认提交时机。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-03 | v1.0 | 初版：承接 12 项优化需求 + 10 项发掘项 + 三裁定（D-1 .html 入范围 / D-2 会话内字符锚 / D-3 样式写路径 A 默认+B 限内联）；文档路由（L1/L2 全并入既有文档、L3 新建本计划）；注册清单（REQ-ED-006~008、BR-ED-DATA-004/005 + 002 修订、TERM-ED-004~006）；PI-ED-06~13 三批次（批A 视图修复/批B 契约先行/批C 独立），候补 5 项；协调点（AG 并行窗口、lib.rs 冲突点） |
