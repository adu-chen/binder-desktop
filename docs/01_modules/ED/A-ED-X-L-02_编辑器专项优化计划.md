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
| D-3 agent 样式写路径 | **A 默认 + B 限内联**：chat/agent 回合样式修改走 DE diff 裁决（DiffPayload 扩样式 op + DiffCard 样式呈现）；用户显式触发的局部修改样式操作视为用户编辑直改（BR-AG-STATE-003 语义扩展） |

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

#### PI-ED-06　分页页高修复 + 纸张选择（#8 / #9 / N8）　✅ G4 已实现（2026-07-03，b8eb89a；视觉/IME 真机验收待做）
- **承接**：BR-ED-DATA-003（既有）；根因已确认——分页缝紧跟页末块、未补足页尾留白，页高=内容累计高（≤预算）。
- **文件范围**：`ed/extensions/Pagination.ts(+test)`（breaks 扩为 `{index,fillPx}[]`，gap widget 加 filler；末页撑满整页）、纸张预设表（A4/A3/A5/Letter × 横竖，取材旧项目 constants）、纸张下拉 UI、`edViewPrefsStore`（per-doc localStorage 视图偏好，纸张项）。
- **验收**：非末页视觉高度恒等 A4（或所选纸张）比例（纯逻辑不变量单测：used+fill=预算）；纸张/方向切换即时重排、不改模型不产脏态；IME 组合输入期间分页重测量不打断输入（N8，真机验证）。

#### PI-ED-15　分页重测量与 BDocx 打开稳定性修复（SCROLL-JUMP-PAGINATION-01）　✅ 已闭合（2026-07-26，人类验收）
- **承接**：`BR-ED-DATA-003` / `ED-EDIT` / `X-FORBID-007`；直接 HTML 无分页路径无抖动，而 BDocx 在打开即抖动，故修复同时覆盖分页呈现反馈环及其打开控制面，不改变内容模型、保存或后端转换。
- **文件范围**：`ed/extensions/Pagination.ts(+test)`、`styles.css`、`Editor.tsx`、workspace 恢复、状态机监听、文档打开 service/store 及其测试、ED/SYS L2。重测量读取当前分页缝几何并扣除累计高度，禁止可见树临时隐藏分页缝；`lastFill` 与普通 DecorationSet 解耦；不以整个 ProseMirror ResizeObserver 驱动。启动恢复/监听注册单飞，异步打开按路径 latest-wins；非活动路径的迟到结果只能补全 session，内容载入把分页启用与纸张几何收敛为一次布局事务。
- **原验收计划（已由闭合裁定完成）**：自动回归覆盖分页重测量、恢复单飞、监听单飞、迟到内容不夺回
  活动编辑器；真机（DevTools关闭）覆盖 BDocx 输入与缩放。最终以本节“闭合裁定”的人类发布验收为准。
- **追加修复（2026-07-25）**：真机反馈剩余偶发单跳集中在内容底部、缩小时更易复现。修正
  `handleScrollToSelection` 的缩放坐标混算：只以光标与编辑滚动容器的视口 rect 判断，仍相交即接管默认滚动；回归覆盖 50% 缩放与底部边缘。
- **结构分页锚定（2026-07-25，历史方案，控制权已由下一段替代）**：审计确认剩余单跳发生在分页缝真实新增/移动后，而非默认选区滚动。
  `PaginationExtension` 在结构计划提交前捕获浏览器实际绘制的光标 range（降级为 DOM range/首内容块），并在**同一任务**内按 Y 差补偿一次滚动；不得延至下一帧形成先下推、再回拉的可见单跳。仅末页补白变化不调滚动。以编辑器 DOM 内联 `overflow-anchor:none` 禁用 ProseMirror 内置 preserve，与测量 epoch 共同防止竞争/迟到提交。
- **控制权重构（2026-07-26）**：后续复测否定“分页层独占滚动补偿”的完整性假设。删除 ProseMirror DOM
  内联 `overflow-anchor:none`，恢复原生 preserve/scrollIntoView；浏览器锚定只在滚动容器禁用。输入期冻结
  break plan 450ms，静默后一次结算；break widget 身份与 fill CSS 更新解耦，fill 保留小数；分页事务先由
  ProseMirror 完成更新，再只补偿残差。BDocx 在当前 `loadSeq` 首轮分页 settled 前保持加载遮罩。
- **首输入预拦截（2026-07-26）**：重新编译后稳定复现“重新定位光标后的首字符同步单跳”，发生时间早于
  450ms 分页结算。保留 ProseMirror 更新期 preserve 和真实越界滚动；分页模式只在已揭示、非 composition
  的折叠文本光标完整位于 `.editor-paper` client viewport 时接管冗余 `scrollIntoView`。光标优先取原生
  Selection/Range，降级 DOM Range；两端统一屏幕坐标，不使用 scrollTop/zoom 混算。自动验证已完成，
  DevTools关闭的重新构建Tauri应用曾作为关闭门禁，最终结果见本节“闭合裁定”。
- **指针激活事务门控（2026-07-26，历史一次性方案，已由光标激活周期替代）**：上述 Range 门控真机复测仍跳，撤销其运行时可靠性假设。分页编辑器
  主键 `pointerup/mouseup` 记录当前 doc 引用与 scrollTop；仅在紧随其后的第一次 doc 变化且用户没有主动
  滚动时一次性吞掉 `scrollIntoView`，然后立即失效。composition首输入纳入该一次性路径；非内容导航、后续
  输入、真实越界和 blur 仍由原生控制。该分支若仍失败，将否定当前可见写入来自 `scrollIntoView`，不得再
  叠加同类门控。
- **光标激活周期（2026-07-26）**：真机稳定出现“首字符正常、第二字符单跳、后续稳定”，证明一次性门控只把
  原生归位延迟一个字符。取消字符计数；指针真实屏幕点作为锚，Range只提供相对位移，并按运行时检测的zoom/rect
  比例映射到屏幕。估算光标仍在client viewport且用户未滚动时持续接管每次doc变化；选择导航、真实越界、
  用户滚动、blur或新指针定位结束周期并恢复原生滚动，composition同样适用。
- **闭合前隐患收口（2026-07-26）**：分页层内部锚点补偿写入scrollTop后发专用事件更新现有周期基线，
  不再误判为用户滚动；周期随文档身份、格式、缩放、纸张和方向显式清理；pointerup/mouseup延后一帧等待
  ProseMirror最终选区，只允许主键单击后的折叠TextSelection建周期，排除双击、拖选和NodeSelection。
- **闭合裁定（2026-07-26）**：人类确认当前发布版本已满足“可编辑、无持续闪跳”的需求；本轮复跑定向
  测试40/40、TypeScript与Vite生产构建均通过。Block 内分页拆分迁移至 PI-ED-16，不再作为 PI-ED-15
  的未完成项；统一构建中的bundled LibreOffice校验不一致继续归独立DOC课题。

#### PI-ED-16　Block 内分页拆分（PAGINATION-INTRABLOCK-SPLIT-01）　✅ 已闭合（2026-07-26，人类接受当前边界）
- **承接**：`BR-ED-DATA-003` / `ED-EDIT`；不新增业务规则，不改变 LogicalState、DiskState、保存与脏态。
- **目标**：将分页输入由 Block 高度序列升级为可断视觉流片段；第一阶段只对普通段落按真实视觉行生成
  可映射的 ProseMirror position 断点，并以贪心填页保留当前页所有完整可容纳行；允许单行页首/页尾片段。
  标题、TaskList、引用、代码、表格和媒体保持原子降级；不可测量时同样降级，不猜测断点。
- **实现顺序**：正式WKWebView行间widget门禁 → `paginationFlow`纯逻辑 → plan position化与事务映射 →
  两级测量（先Block、仅溢出段落测行）→ `span`行间分页缝 → 输入冻结/视口锚点回归。
- **文件范围**：`Pagination.ts(+test)`、`paginationFlow.ts(+test)`、`paginationViewportAnchor.ts(+test)`、
  `styles.css`、ED技术设计与本计划；不进入Editor载入、store、DOC转换或后端路径。
- **硬门槛**：DevTools关闭，覆盖50%/75%/100%、中英文/混合字号、断点前后编辑、Enter/Delete/IME、
  多页长段落；分页事务不改doc/history/dirty，静默结算后可见锚点残差≤1 CSS px且不得重新出现持续或逐字跳动。
- **实现结果（2026-07-26）**：新增 `paginationFlow` 纯规划器；`PageBreakPlan` 改为
  `id+position+kind+fill` 并在冻结期以 `assoc=-1` 映射；既有 gap 按 position/bias 归一；两级测量只对
  实际溢出的顶层普通段落定位视觉行，采用 grapheme 合法位置的指数探测+二分查找；行间使用不可交互
  `span` widget。人工验收发现2+2防孤行会制造不符合需求的页尾空白，现改为“能放一行就保留一行、零行
  才换页”的贪心策略；后续人工验收又发现旧段内widget会污染自然换行并把行中position自锁为假行首，
  离屏克隆和DOMSerializer副本又都无法保证与当前可见纸张的真实行宽同源，现改为同一rAF内只暂时过滤
  `paragraph-line` widget，Block页缝、页码、纸张高度与末页补白始终保留，直接读取当前可见ProseMirror
  Range行盒并在回调返回前恢复最终完整分页。含hardBreak段落、复杂节点/inline atom/不可测几何均原子
  降级；既有450ms冻结和视口补偿未改。
- **自动验证**：分页定向55/55、TypeScript与Vite生产构建通过；全量前端349/353，4项失败仍均为既有
  `edLocate`选区偏移断言；治理审计passed、无finding。统一构建被独立DOC课题的bundled LibreOffice
  版本不一致阻断，不为本课题反向修改。
- **闭合裁定（2026-07-26）**：人类已在正式应用中多轮手工测试分页、拆分与跳跃，并在确认普通段落
  `paragraph-line` widget 仍可能形成行中强制断开后，明确决定不再优化并闭合。该限制与复杂节点原子
  降级一并作为当前版本接受边界；不将其写成已修复，也不再保留“正式人工矩阵待完成”的开放门禁。
  未来如要求 Word 级自然行分页必须另开架构课题，不得回退离屏近似或牺牲 PI-ED-15 防跳控制。

#### PI-ED-07　编辑器工作区体验批（#4 / #5 / #7 / #10 / #11 / #12 / N7）　✅ G4 已实现（2026-07-03，c056348；交互真机验收待做）
- **承接**：视图层（无规则注册）；BR-ED-STATE-003 呈现位置调整（语义不弱化）。
- **文件范围**：`EditorStatusBar.tsx`（新：字数/行数/光标位/脏态/保存态；接 @tiptap/extension-character-count）；冲突条重构为状态栏项+弹出双显式按钮；`tabsStore.reorder(+test)` + TabStrip 指针拖拽排序 + 滚动条隐藏 + 纵轮转横滚 + 激活标签 scrollIntoView；工具栏两排固定分组 + 溢出滚动；zoom 迁入 `edViewPrefsStore`（per-doc 持久）+ ctrl/pinch 滚轮缩放；SM-DOC-01 OpenConverting→ 会话 loading 呈现态 + 骨架/不确定进度；每标签滚动位置暂存。
- **验收**：状态栏各项实时正确；冲突裁决仍双向显式（BR-ED-STATE-003 不弱化）；标签拖动排序/无滚动条；工具栏窄窗口尾部可滚出；缩放跨切换保留；大 docx 打开有即时反馈。

#### PI-ED-08　表格操作 UI + 图片交互（N3 / #3 图片部分）　✅ G4 已实现（图片 Topic B 自动验证完成；WKWebView 人工验收待做）
- **承接**：视图+用户编辑（改模型 attrs 产脏态，合法，非装饰层）。
- **文件范围**：表格控件沿既有课题；图片使用自定义 NodeView document 捕获拖宽、ED 图片操作服务和
  直接挂载 `.ed-img-view` 的邻近上移/下移/删除控件。
- **验收**：图片真实拖宽产生单个 width 事务并在实际改宽时清除固定 height；空拖动不产生脏态；
  独立图片段落可在文档流内移动并保持 Block/媒体属性/选中态，混排图片禁止块移动但可删除；保存重开
  保持尺寸、顺序和删除结果。真实探针已证明 `data-display` 不经 DOCX 持久，因此撤销 inline/block
  按钮验收，不把失败能力留在 UI。

#### PI-ED-17　BDocx 项目图片代理编辑（BDOCX-IMAGE-AGENT-EDIT-01）

- **承接**：REQ-ED-004 / BR-ED-DATA-001/003 / TERM-ED-008；联合 SYS/AG/DE/DOC/WS。
- **范围**：当前活动 BDocx 的图片快照、workspace 项目图片来源、独立段落
  `insert_image`、当前实例 `delete_image`、整批原子草稿事务与结构化 Diff。
- **排除**：外部导入、远程/data URL、代理缩放/移动、浮动/绝对定位、任意拖放和源文件删除。
- **验收**：基线/Block/image/source hash 均须匹配；reject 恢复全文；保存重开保持受支持图片语义，
  缓存缺失时可凭 workspace 来源与哈希重新物化。

### 批B　契约批（文档修订先行 → 实现；D-1/2/3 为前提）

#### PI-ED-09　blockId 跨 docx 往返实测（N6，批B 地基）　✅ 已完成（2026-07-03，b431a31；保留率 0% → 兜底重建，锚有效期=会话内锁定）
- **承接**：A-ED-M-T-01 §11.2 既有债务；BR-ED-DATA-001。
- **动作**：实测 Pandoc 管线对 `data-block-id` 的保留度（含 preserve-styles.lua 是否需扩展）；不达 → 启用转换层兜底注入（旧项目 `inject_missing_data_block_ids` 模式，归 DOC）。结果回填 A-ED-M-T-01 §11.2。
- **验收**：docx 打开→编辑→保存→重开，既有块 id 保留率实测数据落文档；字符级契约（PI-ED-10）以此为地基。

#### PI-ED-10　字符级锚定与选区引用（#1 / #2 / #6）　✅ G4 已实现（2026-07-03，9ae444f 契约 + 9b3801a 实现；BR-ED-DATA-004 owner+covers 闭合）
- **文档先行**：§3.2 之 A-ED-M-D-01（REQ-ED-006）+ A-ED-M-T-01（BR-ED-DATA-004 + TERM-ED-004/005）+ A-ED-I-P-01 + A-REF-M-T-01 修订，**经人类确认后**进代码。
- **文件范围**：`edLocate.ts` 升级（SelectionRef 含 `{blockId,start,end,text,path}`；偏移↔PM position 换算；`getCursorRef` 按需查询）；选区 dragstart 携带 `application/x-binder-selection-ref`（拖出编辑器边界才转引用语义，器内保持原生移动）；Composer drop → 位置引用 chip（REF 协议格式）。
- **@GOV**：BR-ED-DATA-004（锚定纯逻辑 + 换算）。
- **验收**：agent 经 `{blockId,start,end}` 可精确取址目标字符区间（四格式统一纯文本平面）；锚失效回明确信号；选区拖入 Composer 生成含位置的引用；纯逻辑（偏移换算/失效判定）单测。

#### PI-ED-11　样式协议（#3 协议部分；DE/AG 联动）　✅ G4 已实现（2026-07-03 全件：ED 侧投影+StyleOp 原子写；后端 ag.list_blocks marks/blockAttrs 投影（extract_marks 栈式扫描）+ DiffPayload style_ops 持久化（迁移 v5）+ DiffCard 样式摘要呈现；人类授权后端推进。真实样式 diff 数据待 AG 解桩（G-1/G-2）填充）
- **文档先行**：A-ED-M-D-01（REQ-ED-007）+ A-ED-M-T-01（BR-ED-DATA-005 + TERM-ED-006）+ A-ED-I-P-01（op 粒度表）+ A-DE-*（样式载荷/呈现）+ A-AG-*（BR-AG-STATE-003 扩展，**协调 AG 并行窗口**），经人类确认后进代码。
- **文件范围**：读侧 `ag.list_blocks` 样式投影；写侧样式 op 集（set_mark/unset_mark/set_block_attr per CharAnchor 区间）经 `ed.apply_edit`；D-3 分流——回合产出样式 op 进 DiffPayload（DiffCard 扩样式呈现），局部修改样式直改。
- **@GOV**：BR-ED-DATA-005（ED 侧）；DE/AG 侧各归其规则。
- **验收**：agent 读到结构化样式；样式 diff 卡可裁决落盘；内联样式操作即改即脏；X-FORBID-006 语义核验（回合产出无直改路径）。

#### PI-ED-12　大纲面板（N1）　✅ G4 已实现（2026-07-03：outlineStore 投影 + OutlinePanel + LeftRail 入口 + blockId 跳转；跳转/呈现真机验收待做）
- **承接**：视图层；吃 PI-ED-10 定位底座（heading 树 + blockId 跳转）。旧项目 OutlinePanel 参考。
- **验收**：大纲随文档实时更新、点击精确跳转、长文档可用。

### 批C　独立/低优批

#### PI-ED-13　.html 富文本编辑接入（D-1 / REQ-ED-008）　✅ G4 已实现（2026-07-03：editFormat=html_file 三分支路由 + HTML 往返 + blockId 落盘稳定；head/script 非正文处理策略为实现期债务）
- **文档先行**：A-ED-M-D-01（REQ-ED-008）+ A-ED-M-T-01（BR-ED-DATA-002 修订至三格式）。
- **文件范围**：`Editor.tsx`/`edService.ts` 格式路由（.html → 富文本 editFormat，读写仍经 ws.read_doc/save_doc 文本路径，**不经 DOC**——非二进制，X-SEQ-004 不涉及）；序列化 getHTML 往返；blockId 经 data-block-id 落 HTML（.html 源可承载，与 md 不注入策略区分声明）。
- **验收**：.html 打开为富文本、编辑保存往返保真、blockId 跨保存稳定（.html 可落盘 id，锚定强于 md）。

### PI-ED-14 · Markdown 终态富文本安全往返（ED-MARKDOWN-RICH-ROUNDTRIP-01）

- **状态**：in_progress（2026-07-18 人类确认，替代 2026-07-03 的 Markdown 纯文本裁决）。
- **规则来源**：REQ-ED-001 → BR-ED-DATA-002 / BR-ED-DATA-006 / BR-ED-VERIFY-001；复用 ED-EDIT 与 SM-ED-01。
- **范围**：官方 TipTap Markdown Codec、`editFormat=markdown` / `contentFormat`、终态样式、格式能力限制、AG/DE block projection、会话持久化迁移。
- **验收**：CommonMark+GFM 支持子集终态呈现；未编辑不改写；编辑后语义往返；非支持结构不静默丢失；`.txt`、b-docx、html 路径不回归。

**候补清单（不开 PI，待人类排期）**：N2 编辑器内查找/替换；N4 外部图片导入（需 A-WS-I-P-01 增
`ws.import_media`）；N5 链接编辑弹层（随 #11 顺手）；N9 编辑器分页 vs PRV 打印分页差异说明（文档
动作）；N10 自动保存（产品决策，与基线冲突模型有张力，默认不做）。

## §6 实施顺序

```
批A：PI-ED-06（bug 优先）→ PI-ED-07 → PI-ED-08          ——无契约依赖，可即刻推进
批B：PI-ED-09（地基实测）→ 文档修订（§3.2，人类确认）→ PI-ED-10 → PI-ED-11 → PI-ED-12
批C：PI-ED-13 → PI-ED-14（Markdown Codec 与跨模块投影）→ 候补项按人类排期
```

排序逻辑：批A 含已上线功能缺陷（#8）与纯视图快赢；批B 是「agent 精确编辑」主线，**契约先行**
（否则 ED/AG/DE/REF 各自拼装全线返工），PI-ED-09 实测不达则 PI-ED-10 锚定地基改走兜底注入。

## §7 A 类债务与显式延后登记

| 项 | 处置 |
|---|---|
| 跨会话字符级锚定（内容哈希+diff 重建） | D-2 显式延后；登记 A-ED-M-T-01 §11（PI-ED-10 文档动作含此） |
| 图片宽度/嵌入样式的 docx 往返保留度 | 图片宽度、资源、文档流顺序、删除结果与重开尺寸已实测；`data-display` 探针确认丢失，已撤下 inline/block UI，不再作为待验证能力 |
| 分页视觉 vs PRV 打印分页差异 | N9 候补（说明性文档动作） |
| 大文档虚拟化/15MB 上限 | 既有债务不变，G5 NFR |
| **分页大文档增量测量**（2026-07-04 登记，2026-07-26口径同步）：2682块/123页文档每次静默结算仍全量测2k+ rect；当前内容输入等待450ms，且已删除ProseMirror整体ResizeObserver自反馈，只保留文档事务/字体/媒体/窗口等外部失效源和末块单读。卡顿风险降低但全量测量成本未消除——增量测量（仅测变更块邻域 + 缓存未变块高）为下一档。 | A 类，专项排期（G5 NFR 联动） |
| **分页重排滚动锚定 + 光标保持**（2026-07-04 登记，2026-07-26 控制权重构）：已恢复 ProseMirror 普通输入原生滚动保持，分页输入期冻结并仅在结算事务后做一次残差校正；`lastFill`/普通 widget 已解耦。自动验证完成且人类确认本发布版本达到可编辑、无持续闪跳。 | ✅ PI-ED-15 已闭合；作为 PI-ED-16 硬回归门槛 |
| **Block 内分页拆分**（2026-07-26审计登记并转实施）：顶层普通段落第一阶段已按视觉行拆分；普通段落在WKWebView仍可能行中强制断开，复杂节点仍原子降级。 | ✅ PI-ED-16 已按人类接受边界闭合；当前版本不继续优化，不扩展复杂容器 |

## §8 协调点（并发窗口纪律）

- **AG 文档/代码修订（PI-ED-11）**：PI-AG-WIRE-01 由并行窗口推进中——AG 侧文档修订与
  `ag.list_blocks` 扩展**须与该窗口协调时序**，本计划 ED 侧不单方面改 AG 文件。
- `src-tauri/src/lib.rs` 为最高冲突点：批B 涉及 command 扩展时先与人类确认提交时机。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-03 | v1.0 | 初版：承接 12 项优化需求 + 10 项发掘项 + 三裁定（D-1 .html 入范围 / D-2 会话内字符锚 / D-3 样式写路径 A 默认+B 限内联）；文档路由（L1/L2 全并入既有文档、L3 新建本计划）；注册清单（REQ-ED-006~008、BR-ED-DATA-004/005 + 002 修订、TERM-ED-004~006）；PI-ED-06~13 三批次（批A 视图修复/批B 契约先行/批C 独立），候补 5 项；协调点（AG 并行窗口、lib.rs 冲突点） |
| 2026-07-03 | v1.1 | 执行状态更新：批A 全完成（PI-ED-06/07/08）+ PI-ED-09 实测完成（0% → 兜底重建）+ PI-ED-10 完成（契约 9ae444f + 实现 9b3801a）+ PI-ED-11 ED 侧完成（后端联动件留 §8 协调）；TERM 编号配对修正（004=CharAnchor/005=SelectionRef） |
| 2026-07-03 | v1.2 | 执行状态更新：PI-ED-12 大纲面板完成（视图层）+ PI-ED-13 .html 接入完成（html_file 格式路由，blockId 跨保存稳定）；计划内客户端可独立项全部落地，余 PI-ED-11 后端三件（§8 协调）+ 全批真机验收 |
| 2026-07-03 | v1.3 | PI-ED-11 后端三件完成（人类授权「推进后端」）：ag.list_blocks 样式投影（blocks.rs extract_marks/extract_block_attrs，@GOV BR-ED-DATA-005 读侧后端承载）、DiffPayload style_ops 持久化（db 迁移 v5 + generate/store/route 透传，AG/CBT 构造点 None 占位待解桩填充）、DiffCard 样式摘要呈现（DiffView.styleOps）；计划全部 PI 收口 |
| 2026-07-04 | v1.4 | 分页页高三轮收敛结案（位置差分+无缝测量+zoom 自校准+fill 补足，真机 123 页确认）；§7 登记两条 A 类债务：大文档增量测量（卡顿残余）、**重排滚动锚定+光标保持（较严重，含根治方案指引，按人类 10 行阈值纪律记债）** |
| 2026-07-18 | v1.5 | 新增 PI-ED-14 Markdown 终态富文本安全往返：复用 ED-EDIT/SM-ED-01，接 BR-ED-DATA-006/BR-ED-VERIFY-001，覆盖 Codec、contentFormat、样式能力、AG/DE 与会话迁移；显式替代 2026-07-03 纯文本实现裁决但保留历史记录 |
| 2026-07-25 | v1.6 | 新增 PI-ED-15：直接 HTML 无分页路径验证无抖动后，将分页闪跳范围锁定在分页呈现层；先修可见树临时隐藏/恢复和末页补白触发普通分页缝重建两条路径，手动滚动锚定降为后备。 |
| 2026-07-25 | v1.7 | PI-ED-15 追加打开即振荡根因：整个 ProseMirror 的 ResizeObserver 将分页缝/末页补白自身造成的高度变化当作外部输入，形成重排正反馈；改由文档事务与字体/媒体/窗口外部事件触发重测量。 |
| 2026-07-25 | v1.8 | PI-ED-15 扩展并落地 BDocx 打开控制面：恢复与监听单飞、异步打开路径版本守卫、非活动会话后台补全、分页启用/几何单事务；自动测试完成，DevTools 关闭的真机验收仍待执行。 |
| 2026-07-25 | v1.9 | PI-ED-15 落地结构分页视口锚定：分页计划变化前后保持可见锚点，禁用 ProseMirror preserve 竞争、淘汰过期测量、修正末页底边距；自动回归完成，真机跨页/缩放验收待执行。 |
| 2026-07-25 | v1.10 | PI-ED-15 修正剩余首帧单跳：补偿同步于分页提交，锚点以真实光标 DOM range 优先，避开 widget 边界 `coordsAtPos` 的 side 歧义；真机跨页/缩放验收待执行。 |
| 2026-07-26 | v1.11 | PI-ED-15 控制权重构：恢复 ProseMirror 原生滚动保持；输入期冻结分页、静默后一次结算；break/fill DOM 解耦与浮点精度；分页事务仅做残差补偿；BDocx 首轮 settled 后揭示。 |
| 2026-07-26 | v1.12 | PI-ED-15 控制权重构已实现并通过定向单测、TypeScript 与生产构建；全量前端测试的 4 个既有 `edLocate` 选区偏移失败仍在当前 Scope 外，DevTools 关闭的 BDocx 真机矩阵仍是最终视觉验收门槛。 |
| 2026-07-26 | v1.13 | PI-ED-15 发布构建复测发现残余首输入单跳；新增分页可见光标的冗余 `scrollIntoView` 预拦截，保留原生 preserve、真实越界滚动和分页结算残差边界。相关37项测试、TypeScript与Vite构建通过；统一构建受独立bundled LibreOffice版本校验阻断，发布真机视觉验收仍待执行。 |
| 2026-07-26 | v1.14 | PI-ED-15 Range门控真机失败后新增指针激活事务门控：doc引用变化证明首输入、scrollTop未变证明用户无主动滚动，仅消费一次并覆盖composition首输入；相关38项测试、TypeScript与Vite构建通过，全量329/333仍仅4项既有edLocate失败。 |
| 2026-07-26 | v1.15 | PI-ED-15 第二字符单跳证明一次性门控仅延迟原生归位；升级光标激活周期，以指针屏幕锚+Range相对位移持续覆盖可见区内doc变化，运行时适配zoom rect差异并在真实越界/导航/用户滚动/blur时释放。相关39项测试、TypeScript与Vite构建通过，全量330/334仍仅4项既有edLocate失败。 |
| 2026-07-26 | v1.16 | PI-ED-15闭合前隐患收口：分页内部补偿同步激活周期滚动基线；文档/格式/缩放/纸张身份变化强制清理；指针守卫延至最终选区同步后建立并排除双击/拖选。相关40项、TypeScript与Vite构建通过；全量331/335仍仅4项既有edLocate失败，发布真机闭合验收待做。 |
| 2026-07-26 | v1.17 | 实现一致性同步：标明已被替代的结构独占/一次性门控为历史方案；大文档债务由旧150ms/RO口径更新为当前450ms且无整体ResizeObserver；新增块内分页拆分A类债务，明确当前仅顶层块/listItem边界切页。 |
| 2026-07-26 | v1.18 | 人类确认闭合 PI-ED-15 / SCROLL-JUMP-PAGINATION-01；新增 PI-ED-16 / PAGINATION-INTRABLOCK-SPLIT-01，将 Block 内分页拆分由 A 类债务转为独立实施课题，锁定普通段落先行、复杂类型原子降级和滚动稳定性硬门槛。 |
| 2026-07-26 | v1.19 | PI-ED-16 第一阶段实现：position型分页计划和冻结期映射、普通段落视觉行两级测量、2+2分页流规则、position/bias gap归一与span行间widget落地；相关54项、TypeScript和Vite通过，全量仅保留4项既有edLocate失败，正式WKWebView验收待做。 |
| 2026-07-26 | v1.20 | PI-ED-16人工验收确认分页、块拆分与跳跃基本正常，同时发现2+2规则撤回本可容纳行；按人类决策改为贪心填页，允许单行页首/页尾片段，测量与滚动控制链不变。 |
| 2026-07-26 | v1.21 | PI-ED-16行尾断点收敛：可见DOM中的旧段内widget会制造假行首并在结算重测时自锁；改用无分页缝离屏段落副本测量自然行，断点须通过纵向换行+横向回行双验证，不确定时整段原子降级；Decoration移除即恢复自然排版。 |
| 2026-07-26 | v1.22 | PI-ED-16二次人工失败纠偏：确认过滤分页缝但继续克隆可见DOM仍会复制WebKit辅助BR/残留结构；离屏段落改由ProseMirror正文模型经schema DOMSerializer生成，含hardBreak段落不再参与段内分页，避免短行或旧widget污染成为断点。 |
| 2026-07-26 | v1.23 | PI-ED-16物理分页边界纠正：恢复并保留完整页缝/页码/纸张高度/防跳实现，仅替换普通段落自然行真值路径；同一rAF内暂时过滤paragraph-line widget但保留Block页缝，读取当前可见ProseMirror真实行盒后立即恢复最终完整分页。 |
| 2026-07-26 | v1.24 | 人类确认闭合PI-ED-16及整个分页课题：普通段落行中强制断开与复杂节点原子降级作为当前版本显式接受边界；保留物理分页、打开收敛和PI-ED-15防跳控制，不再保留人工矩阵开放门禁或后继优化Issue。 |
| 2026-07-28 | v1.25 | BDOCX-IMAGE-MEDIA-FOUNDATION-01：PI-ED-08 的图片宽度/资源/alt/重开尺寸完成 DOCX XML 与 HTML 联合实测；持久媒体恢复链已落地。NodeView 人工拖拽与 inline/block 样式不在本课题内，继续保持原验收边界。 |
| 2026-07-28 | v1.26 | BDOCX-IMAGE-INTERACTION-01：PI-ED-08 图片 Topic B 实现 document 捕获拖宽、独立图片段落邻近移动/删除、混排保护及 DOCX 顺序/删除往返测试；撤下真实探针证明不可持久的 inline/block 按钮。 |
| 2026-07-29 | v1.27 | 新增 PI-ED-17：以 ImageOperation 承接 workspace 项目图片代理插入/删除、动态锚与来源哈希、批原子草稿、结构化 Diff、缓存重建和保存重开验收。 |
