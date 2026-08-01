---
文档编号：DE-I-P-01
文档状态：A
负责模块：DE
文档职责：DE Diff Edit 模块级接口协议（L2 接口层）——按 A-SYS-I-P-02 骨架声明 diff 生成接口（承接 AG ChangeProposal）、裁决/自动应用接口（accept/reject/expire）、过期与冲突重检接口、审阅呈现对接；产品核心交互「代理回合→diff→LogicalState/落盘」的 DE 侧 command 面
上游约束：A-SYS-I-P-02_IPC协议.md（骨架）、A-DE-M-T-01_DiffEdit技术设计.md（BR-DE-* 规则、DE-DIFF、payload 形状、TERM-DE-001/002）、A-DE-M-S-01_DiffEdit状态机设计.md（SM-DE-01）、A-WS-I-P-01_工作台接口协议.md（persist_diff/apply_diff/discard_diff 复用）、A-ED-I-P-01_编辑器接口协议.md（locate_block/装饰层呈现）、A-SYS-I-P-01_数据结构定义.md（PendingDiff、payload 字段）、ADU.md
直接承接：实现期 DE command 注册；AG/CBT 经 `de.generate_diff` 提交 ChangeProposal；前端审阅 UI 经裁决接口；落盘经 WS（apply_diff）、呈现经 ED（装饰层）
使用边界：本文声明 DE 接口语义级签名；持久化与落盘经 WS（A-WS-I-P-01，不重复）、呈现经 ED（A-ED-I-P-01，不重复）；不新增 RULE/CHAIN/CONSTRAINT/TERM；裁决 UI 形态（DE-04）属实现期，本文只定接口
变更要求：裁决接口语义变更须复核 SM-DE-01 与 SM-WS-02 双侧；payload 形状变更回写 A-SYS-I-P-01；ChangeProposal 接口变更复核 AG/CBT 调用方
治理成熟度：draft
---

# Binder Diff Edit 接口协议（DE · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）→ 本文（DE 生成/裁决/过期接口）→ 实现 + @GOV(追溯 A-DE-M-T-01)
上游：AG/CBT(ChangeProposal) → de.generate_diff；下游：current/open session → ED LogicalState，未打开文档 accept → ws.apply_diff、呈现 → ed.mount_decoration
```

DE 是「代理回合→diff→LogicalState/落盘」链的中枢：接 AG/CBT 的 ChangeProposal 生成 PendingDiff，经 ED
装饰层或 DiffCard 呈现审阅；激活文档和已打开非激活文档进入 ED LogicalState 后等待保存，未打开文档
accepted 后才驱动 WS 落盘，rejected/expired 不写文件。

## §2 diff 生成接口（DE-DIFF / SM-DE-01 / BR-DE-DATA-001）

| command | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `de.generate_diff` | ChangeProposal（target_blocks: DocRef+blockId、修改内容、origin: 模块/回合、DiffProjection 基线元数据） | diff_id（SM-DE-01 Pending）+ 呈现就绪 | **anchor_lost**（blockId 失效→GenFailed，回报 AG handoffFail）；已打开文档缺 session 基线；持久化失败 | 经 `ed.locate_block`/AgentReadContext 校验锚 + `ws.persist_diff` 持久化 + `ed.mount_decoration` 或 DiffCard 呈现 |
| `de.get_diff` | diff_id | PendingDiff + payload（含 before_snapshot 对比基准） | 不存在 | BR-DE-DATA-002（重启恢复可重建审阅） |
| `de.list_diffs` | DocRef（可空 → workspace 全量） | PendingDiff 列表（按创建序） | — | 多 diff 并存呈现（BR-DE-DATA-003） |
| `de.update_image_diff_snapshots` | diff_id、before/after ImageSnapshot、affectedBlockIds | 将 ED 实际应用结果补入既有 ImageOperation DiffPayload；首次写入后只接受同值幂等重试 | workspace/diff 不匹配、非 image_ops、快照字段/路径/hash 无效、已写值冲突 | BR-DE-DATA-002；只补审阅载荷，不修改文档内容或 diff 终态 |

`patch_kind=block_ops` 时 DiffProjection 必须携带版本化 StructuredBlockEditPayload。每个目标块的
beforeNode/afterNode、blockId、expectedNodeType 与 fingerprint 是执行、详细审阅和精确拒绝的共同
真值；before/after 文本摘要只供紧凑卡显示，不得反向重建执行载荷。该 kind 仅允许
`draft_mode=current_editor`，不得进入 open_session HTML 字符串或 background raw apply。

`patch_kind=table_ops` 时 DiffProjection 必须携带受限 TableOperation 及 before/after TableSnapshot，
包括每个逻辑单元格的 `backgroundColor`。DiffCard 直接消费该投影：`set_cell_background` 按 `RnCn`
呈现修改前后色块与 `#RRGGBB`，`null` 明确呈现为无底纹；不得从原始 HTML 或工具请求参数猜测差异。

`patch_kind=image_ops` 时初始 DiffProjection 必须携带受限 ImageOperation、生成基线与项目来源/锚；
持久载荷 envelope 固定 `payloadVersion=1`，该版本允许快照字段在 ED 应用前缺省、应用后完整固化。
ED 在当前 LogicalState 单事务应用后立即调用 `de.update_image_diff_snapshots` 固化真实 before/after
ImageSnapshot 与受影响 Block。后续 `de.list_diffs(diffIds)` 必须从持久载荷返回
`image_before/image_after/image_affected_block_ids`，DiffCard 直接消费该投影细分插入与删除；该 kind 仅允许
`draft_mode=current_editor`，不得进入后台 raw apply 或非活动会话字符串替换。同一图片单批可各含一次
resize/layout/move 并按固定顺序执行，delete 独占；DiffCard 必须逐项呈现插入、删除、缩放、移动与
对齐的真实前后快照，不得只细分插入/删除。

## §3 裁决接口（DE-DIFF / SM-DE-01 / BR-DE-STATE-001）

| command | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `de.accept_draft_diff` | diff_id、apply_context(current_editor/open_session)、base_session_version?、base_content_hash? | Accepted（不可逆）；仅确认已进入 LogicalState，不触发 `ws.apply_diff`。可由 DiffCard 的“接受”或当前文档的任意保存动作调用 | open_session 基线不匹配 → 拒绝，调用方应转 `de.expire_diff`；终态后再调用 → 协议错误 | SM-DE-01 Accepted → ED/WS save 路径 |
| `de.accept_diff` | diff_id | Accepted（不可逆）→ 仅当目标文档未打开时触发 `ws.apply_diff`，回 Applied + 装饰复位 | 目标文档已打开 → `DE_OPEN_DOC_REQUIRES_LOGICAL_STATE`；apply 失败 → 终态保持 Accepted、WS 侧重试（不回退） | SM-DE-01 Accepted → SM-WS-02（X-INTER-001，未打开文档后台落盘触发） |
| `de.reject_diff` | diff_id | Rejected（不可逆）→ `ws.discard_diff` + 装饰复位 | `draft_mode=open_session` 已自动应用/失效 → 协议错误（卡片无拒绝入口） | 不写文件 |
| `de.expire_diff` | diff_id、reason | Expired（不可逆）→ `ws.discard_diff` + 装饰复位 | 终态后再调用 → 协议错误 | 自动应用失败、超时或外部变更关闭 |

> 终态互斥不可逆：accept/reject/expire 后再裁决 → 协议错误（终态不可转移，BR-DE-STATE-001）。
> 用户裁决交互经 ED 装饰层 `ed.on_decoration_action` 或 DiffCard 转达至此（ED 不代裁，X-FORBID-005）。
> `draft_mode=open_session` 不提供用户裁决入口：前端必须在 DiffProjection 到达后先执行 ED session
> 自动应用事务，成功调用 `de.accept_draft_diff`，失败调用 `de.expire_diff` 或投错误卡；不得调用
> `de.accept_diff` 触发后台落盘。

## §4 过期与冲突重检接口（DE-DIFF / BR-DE-STATE-002 / BR-DE-DATA-003）

| command/事件 | 参数（语义） | 结果 | 归属 |
|---|---|---|---|
| `de.check_expiry` | diff_id（或定时/呈现时触发） | 维持 Pending / 转 Expired | 外部变更（目标块 ≠ before_snapshot，订阅 `ws.external_change`）或超时→Expired；检测不可得→保守维持 Pending（BR-DE-STATE-002） |
| `de.recheck_overlap` | 落盘事件或 ED session 自动应用事件触发（先序 diff Applied/autoApplied） | 重叠 pending diff 重检结果（陈旧者转 Expired） | BR-DE-DATA-003（不并发应用/落盘同一块，不覆盖新内容） |

## §5 跨模块对接小结（不重复定义，仅索引）

| 环节 | 调用 | 文档 |
|---|---|---|
| 接 AG 产出 | AG HandingOff → `de.generate_diff` | A-AG-M-S-01 §3 |
| 锚校验/呈现/自动应用 | `ed.locate_block` / `ed.apply_session_diff` / `ed.mount_decoration` / `ed.dismiss_decoration` | A-ED-I-P-01 §3/§4 |
| 未打开文档落盘/丢弃 | `ws.apply_diff` / `ws.discard_diff` | A-WS-I-P-01 §4 |
| 外部变更 | 订阅 `ws.external_change` → check_expiry | A-WS-I-P-01 §6 |

## §6 测试与验收

- `de.generate_diff` 锚有效→Pending、锚失效→GenFailed 回报 AG；`de.accept_diff` 只对未打开文档触发
  `ws.apply_diff`、ApplyError 不回退终态；`de.accept_draft_diff(open_session)` 不触发 `ws.apply_diff`；
  reject/expire 不写文件 + 装饰复位；终态后再裁决被拒；
  过期保守性（检测不可得不误杀）；重叠重检（先序落盘后陈旧者 Expired）；全链与 AG/ED/WS G5 联调。

## §7 债务与盲区

- 裁决 UI 形态（逐条/批量，DE-04 待人类）影响是否需 `de.accept_batch`/`de.reject_batch` 批量
  接口——延决策后补；CBT 构建产物的批量裁决同此（CBT-02）。
- ~~payload 序列化与 before_snapshot 存储形态延实现期~~ → **已解除（PI-BE-02，2026-06-23）**：
  DiffPayload（TERM-DE-004）持久化于 workspace.db `diff_payloads` 表（payload_ref 主键，after_content
  BLOB、before_snapshot TEXT、is_new_doc），经 PendingDiff.payload_ref 关联；`de.accept_diff` 经此取回
  after_content 触发 `ws.apply_diff` 落盘。DE 拥有载荷形状、WS db 承载持久化（A-DE-M-T-01 §1）。
- `de.check_expiry` 的触发驱动（定时 vs 呈现时 vs 裁决时）延实现期，本文只定接口语义。
- `open_session` 自动应用后的 DiffCard 当前不提供独立撤销接口；撤销能力需后续设计 `de.undo_applied_diff`
  或文档级回滚协议，本轮只保证禁用二次裁决和不触发后台落盘。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件，核心交互链 DE 侧）：DE 接口协议——生成 3 条（generate_diff 接 ChangeProposal+锚校验、get/list）、裁决 2 条（accept 唯一触发落盘不可逆、reject）、过期与重检 2 条（check_expiry 保守过期 / recheck_overlap 冲突）；§5 跨模块对接索引（AG/ED/WS，不重复定义）；零新增注册项；债务（批量裁决接口待 DE-04 决策、序列化/过期触发延实现期） |
| 2026-07-04 | v1.1 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-18 | v1.2 | 补齐已打开文档接口边界：`de.accept_draft_diff` 确认 current/open session LogicalState，不触发后台落盘；`de.accept_diff` 限未打开文档；新增 `de.expire_diff` 承接 open_session 自动应用失败 |
| 2026-07-27 | v1.3 | DE-SAVE-IMPLICIT-ACCEPT-01：确定任意保存动作是已应用草稿 Diff 的接受入口；保存编排先调用既有 `de.accept_draft_diff`，再走 ED/WS 保存链，以既有 effect_status 回调确认 saved。 |
| 2026-07-28 | v1.4 | BDocx TableCellShading：`patch_kind=table_ops` 的 DiffProjection 明确携前后 TableSnapshot；DiffCard 按逻辑坐标显示前后色块、色值与无底纹状态。 |
| 2026-07-29 | v1.5 | BDOCX-IMAGE-AGENT-EDIT-01：新增 patch_kind=image_ops 的 ImageOperation、项目来源/锚、current_editor 与 structured DiffCard 契约；补充 `de.update_image_diff_snapshots` 一次性幂等持久化以及历史查询返回前后 ImageSnapshot 的闭环。 |
| 2026-07-30 | v1.6 | 图片代理编辑闭合审计：image_ops 接口同步同图 resize/layout/move 组合、delete 独占，并明确五类图片操作均须由真实前后快照结构化呈现。 |
| 2026-07-30 | v1.7 | AG-RICH-BLOCK-STRUCTURED-EDIT-01：新增 patch_kind=block_ops 的 exact node before/after、活动草稿限定与逐块结构化审阅；文本摘要不再承担执行或回滚。 |
