---
文档编号：ED-X-L-01
文档状态：A
负责模块：ED
文档职责：ED 编辑器 G3 开发计划——承接型 Plan Item，承接 ED-EDIT/ED-DECOR 链路与 BR-ED-* 规则、X-FORBID-005、SM-ED-01（含 Conflicted），明确文件范围/@GOV/A 类债务（含 TipTap 选型）/验收
上游约束：A-CORE-X-L-02（P2）、APC §5/§6/§12、A-ED-M-T-01（含 §11.1 选型记录）/A-ED-M-S-01、A-ED-I-P-01、A-WS-X-L-01（经 WS 文件 I/O）、A-DOC-X-L-01（docx 转换）、A-CORE-X-R-01
直接承接：本计划驱动 ED 实现；@GOV 追溯 BR-ED-*；SM-ED-01 转移矩阵
使用边界：L3 承接型 Plan Item；@GOV 在实现代码；NFR 延 G5
变更要求：PI 增删保持承接可追溯
治理成熟度：draft
---

# Binder 编辑器开发计划（ED · L3）

## §1 文档定位

ED 是 P2 富文本编辑运行时，依赖 WS（文件 I/O）与 DOC（docx 转换）。本计划承接 A-ED-M-T-01 §11.1
已定的选型记录（TipTap v3 / blockId 机制 / 样式子集）——这些是已锁方向的 A 类，实现期落地验证。

## §2 Plan Item 清单

### PI-ED-01　编辑器运行时与文档编辑会话（含冲突态）
- **承接**：REQ-ED-001 / CHAIN ED-EDIT / SM-ED-01（Closed/Loading/Ready/Editing/ReadOnly/
  **Conflicted**）/ BR-ED-STATE-001 / **BR-ED-STATE-003**（外部变更会话处理）。
- **文件范围**：`src/components/Editor/`（TipTap 运行时、会话生命周期、冲突裁决 UI 对接
  `ed.resolve_conflict`）；订阅 `ws.external_change`。
- **@GOV**：会话生命周期 `BR-ED-STATE-001`；外部变更处理 `BR-ED-STATE-003`。
- **A 类债务（已定方向，实现期 lock）**：TipTap v3 系次版本 lock + 扩展集 + StarterKit（ED-01，
  A-ED-M-T-01 §11.1）；多会话并发隔离（实现期）；confirmDiscard UI 形态（实现期）。
- **验收（强制，APC §6）**：SM-ED-01 矩阵全覆盖含 Conflicted（externalChange/saveConflict→
  conflictKeepMine/reload）；脏态不静默关闭/丢失；解析失败降级只读；离线 open/edit/close 可用；
  ED 无直接文件 I/O 路径（静态核验，经 WS）。

### PI-ED-02　block 模型与稳定 blockId
- **承接**：REQ-ED-002 / BR-ED-DATA-001。
- **文件范围**：`src/components/Editor/extensions/BlockId*`（blockId 扩展、appendTransaction 补缺/
  分裂去重）；`ed.locate_block`。
- **@GOV**：blockId 稳定性与失效信号 `BR-ED-DATA-001`。
- **A 类债务（已定机制，实现期验证）**：blockId = `block_`+UUID + appendTransaction 补缺、分裂
  首块保 id（ED-02，A-ED-M-T-01 §11.1）；md 源不注入、跨会话哈希重建（**推荐待实现期与 DE 联合
  验证转锁定**）；分裂/合并继承边角。
- **验收**：普通编辑后 blockId 不变；删除块后定位收到 anchor_lost 失效信号；无静默错位（与 DE 联调）。

### PI-ED-03　md/docx 统一编辑模型
- **承接**：REQ-ED-003 / BR-ED-DATA-002。
- **文件范围**：`src/components/Editor/`（统一编辑模型；docx 经 DOC `doc.open_for_edit`/
  `doc.save_from_edit`，ED 无二进制代码）。
- **@GOV**：统一模型与转换分工 `BR-ED-DATA-002`。
- **A 类债务**：docx 样式子集承载范围（ED-04/05，已采纳三级契约，与 DOC PI 联合实测）；blockId
  跨 docx 往返保持（DOC-02，DOC 侧实测，不达走重建锚）。
- **验收**：md/docx 共用编辑模型与交互；ED 代码无 docx 二进制处理（静态核验）。

### PI-ED-04　分页与图文媒体（呈现层纯化）
- **承接**：REQ-ED-003/004 / BR-ED-DATA-003。
- **文件范围**：`src/components/Editor/pagination/`（移植 `tiptap-pagination-plus` 适配）；媒体
  内容块（资源经 WS）。
- **@GOV**：分页/媒体呈现层纯化（不改模型不产生脏态）`BR-ED-DATA-003`。
- **A 类债务**：分页光标/遮罩实现（ED-03，移植适配，实现期）；媒体格式支持范围（实现期）。
- **验收**：分页开/关与重排不改文档模型、不产生脏态；媒体资源越界被拒（X-FORBID-004）。

### PI-ED-05　编辑器装饰层接口
- **承接**：REQ-ED-005 / CHAIN ED-DECOR / BR-ED-STATE-002 / X-FORBID-005。
- **文件范围**：`src/components/Editor/decorations/`（装饰层挂载/复位；供 DE diff、AG 局部修改）；
  `ed.mount_decoration/update/dismiss/on_decoration_action` + `ed.apply_edit`（内联采纳落点）。
- **@GOV**：装饰层契约 `BR-ED-STATE-002`；唯一受控通道 `X-FORBID-005`。
- **A 类债务**：装饰呈现 schema（diff 高亮/内联候选样式，与 DE/AG 联合，实现期）；apply_edit op
  粒度（依 ProseMirror 事务，实现期）。
- **验收**：decoration 按 blockId 挂载、按消费方信号复位不残留；不存在绕过 ED 接口直改编辑器状态
  的路径（X-FORBID-005 静态+与 DE/AG 集成核验）。

## §3 实施顺序

```
PI-WS-01（文件 IO，前置）→ PI-ED-01（运行时/会话）→ PI-ED-02（blockId）→ PI-ED-05（装饰层）
PI-ED-03（docx 统一模型）依赖 PI-DOC-01；PI-ED-04（分页/媒体）可并行
```

## §4 横切验收

audit 绿；@GOV 追溯 BR-ED-*；SM-ED-01 矩阵（含 Conflicted）全覆盖；`// covers: BR-ED-*` 覆盖
ED 全部规则（6 条，含 BR-ED-STATE-003）有断言；X-FORBID-005 静态审查。

## §5 承接的 A 类债务（A-CORE-X-R-01 §6）

| 债务 | PI | 处置 |
|---|---|---|
| TipTap v3 次版本 lock + 扩展集（ED-01） | PI-ED-01 | 实现期 lock，选型记录回填 L2 |
| blockId 算法/分裂合并/md 哈希重建（ED-02） | PI-ED-02 | 实现期验证（与 DE 联合）转锁定 |
| 分页光标遮罩实现（ED-03） | PI-ED-04 | 移植适配，实现期 |
| 装饰呈现 schema / apply_edit op 粒度 | PI-ED-05 | 实现期与 DE/AG 联合 |
| 编辑延迟/大文档分页性能 NFR | 全 PI | G5 标定 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3）：ED 5 条 Plan Item（运行时与会话含 Conflicted / blockId / md·docx 统一模型 / 分页媒体 / 装饰层），承接 ED-EDIT/ED-DECOR + 6 规则 + X-FORBID-005 + SM-ED-01；承接 A-ED-M-T-01 §11.1 已定选型（TipTap v3/blockId/样式子集）作为 A 类实现期 lock；文件范围/@GOV/验收齐 |
| 2026-07-04 | v1.1 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
