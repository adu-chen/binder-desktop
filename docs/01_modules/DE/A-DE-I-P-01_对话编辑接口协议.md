---
文档编号：DE-I-P-01
文档状态：A
负责模块：DE
文档职责：DE 对话编辑模块级接口协议（L2 接口层）——按 A-SYS-I-P-02 骨架声明 diff 生成接口（承接 AG ChangeProposal）、裁决接口（accept/reject）、过期与冲突重检接口、审阅呈现对接；产品核心交互「对话→diff→裁决→落盘」的 DE 侧 command 面
上游约束：A-SYS-I-P-02_IPC协议.md（骨架）、A-DE-M-T-01_对话编辑技术设计.md（BR-DE-* 规则、DE-DIFF、payload 形状、TERM-DE-001/002）、A-DE-M-S-01_对话编辑状态机设计.md（SM-DE-01）、A-WS-I-P-01_工作台接口协议.md（persist_diff/apply_diff/discard_diff 复用）、A-ED-I-P-01_编辑器接口协议.md（locate_block/装饰层呈现）、A-SYS-I-P-01_数据结构定义.md（PendingDiff、payload 字段）、ADU.md
直接承接：实现期 DE command 注册；AG/CBT 经 `de.generate_diff` 提交 ChangeProposal；前端审阅 UI 经裁决接口；落盘经 WS（apply_diff）、呈现经 ED（装饰层）
使用边界：本文声明 DE 接口语义级签名；持久化与落盘经 WS（A-WS-I-P-01，不重复）、呈现经 ED（A-ED-I-P-01，不重复）；不新增 RULE/CHAIN/CONSTRAINT/TERM；裁决 UI 形态（DE-04）属实现期，本文只定接口
变更要求：裁决接口语义变更须复核 SM-DE-01 与 SM-WS-02 双侧；payload 形状变更回写 A-SYS-I-P-01；ChangeProposal 接口变更复核 AG/CBT 调用方
治理成熟度：draft
---

# Binder 对话编辑接口协议（DE · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）→ 本文（DE 生成/裁决/过期接口）→ 实现 + @GOV(追溯 A-DE-M-T-01)
上游：AG/CBT(ChangeProposal) → de.generate_diff；下游：accept → ws.apply_diff、呈现 → ed.mount_decoration
```

DE 是「对话→diff→裁决→落盘」链的中枢：接 AG/CBT 的 ChangeProposal 生成 PendingDiff，经 ED
装饰层呈现审阅，裁决终态后驱动 WS 落盘（accepted）或丢弃（rejected/expired）。

## §2 diff 生成接口（DE-DIFF / SM-DE-01 / BR-DE-DATA-001）

| command | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `de.generate_diff` | ChangeProposal（target_blocks: DocRef+blockId、修改内容、origin: 模块/回合） | diff_id（SM-DE-01 Pending）+ 呈现就绪 | **anchor_lost**（blockId 失效→GenFailed，回报 AG handoffFail）；持久化失败 | 经 `ed.locate_block` 校验锚 + `ws.persist_diff` 持久化 + `ed.mount_decoration` 呈现 |
| `de.get_diff` | diff_id | PendingDiff + payload（含 before_snapshot 对比基准） | 不存在 | BR-DE-DATA-002（重启恢复可重建审阅） |
| `de.list_diffs` | DocRef（可空 → workspace 全量） | PendingDiff 列表（按创建序） | — | 多 diff 并存呈现（BR-DE-DATA-003） |

## §3 裁决接口（DE-DIFF / SM-DE-01 / BR-DE-STATE-001）

| command | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `de.accept_diff` | diff_id | Accepted（不可逆）→ 触发 `ws.apply_diff`，回 Applied + 装饰复位 | apply 失败 → 终态保持 Accepted、WS 侧重试（不回退） | SM-DE-01 Accepted → SM-WS-02（X-INTER-001，唯一落盘触发） |
| `de.reject_diff` | diff_id | Rejected（不可逆）→ `ws.discard_diff` + 装饰复位 | — | 不写文件 |

> 终态互斥不可逆：accept/reject/expire 后再裁决 → 协议错误（终态不可转移，BR-DE-STATE-001）。
> 用户裁决交互经 ED 装饰层 `ed.on_decoration_action` 转达至此（ED 不代裁，X-FORBID-005）。

## §4 过期与冲突重检接口（DE-DIFF / BR-DE-STATE-002 / BR-DE-DATA-003）

| command/事件 | 参数（语义） | 结果 | 归属 |
|---|---|---|---|
| `de.check_expiry` | diff_id（或定时/呈现时触发） | 维持 Pending / 转 Expired | 外部变更（目标块 ≠ before_snapshot，订阅 `ws.external_change`）或超时→Expired；检测不可得→保守维持 Pending（BR-DE-STATE-002） |
| `de.recheck_overlap` | 落盘事件触发（先序 diff Applied） | 重叠 pending diff 重检结果（陈旧者转 Expired） | BR-DE-DATA-003（不并发落盘同一块，不覆盖新内容） |

## §5 跨模块对接小结（不重复定义，仅索引）

| 环节 | 调用 | 文档 |
|---|---|---|
| 接 AG 产出 | AG HandingOff → `de.generate_diff` | A-AG-M-S-01 §3 |
| 锚校验/呈现 | `ed.locate_block` / `ed.mount_decoration` / `ed.dismiss_decoration` | A-ED-I-P-01 §3/§4 |
| 落盘/丢弃 | `ws.apply_diff` / `ws.discard_diff` | A-WS-I-P-01 §4 |
| 外部变更 | 订阅 `ws.external_change` → check_expiry | A-WS-I-P-01 §6 |

## §6 测试与验收

- `de.generate_diff` 锚有效→Pending、锚失效→GenFailed 回报 AG；`de.accept_diff` 唯一触发
  `ws.apply_diff`、ApplyError 不回退终态；reject/expire 不写文件 + 装饰复位；终态后再裁决被拒；
  过期保守性（检测不可得不误杀）；重叠重检（先序落盘后陈旧者 Expired）；全链与 AG/ED/WS G5 联调。

## §7 债务与盲区

- 裁决 UI 形态（逐条/批量，DE-04 待人类）影响是否需 `de.accept_batch`/`de.reject_batch` 批量
  接口——延决策后补；CBT 构建产物的批量裁决同此（CBT-02）。
- ~~payload 序列化与 before_snapshot 存储形态延实现期~~ → **已解除（PI-BE-02，2026-06-23）**：
  DiffPayload（TERM-DE-004）持久化于 workspace.db `diff_payloads` 表（payload_ref 主键，after_content
  BLOB、before_snapshot TEXT、is_new_doc），经 PendingDiff.payload_ref 关联；`de.accept_diff` 经此取回
  after_content 触发 `ws.apply_diff` 落盘。DE 拥有载荷形状、WS db 承载持久化（A-DE-M-T-01 §1）。
- `de.check_expiry` 的触发驱动（定时 vs 呈现时 vs 裁决时）延实现期，本文只定接口语义。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件，核心交互链 DE 侧）：DE 接口协议——生成 3 条（generate_diff 接 ChangeProposal+锚校验、get/list）、裁决 2 条（accept 唯一触发落盘不可逆、reject）、过期与重检 2 条（check_expiry 保守过期 / recheck_overlap 冲突）；§5 跨模块对接索引（AG/ED/WS，不重复定义）；零新增注册项；债务（批量裁决接口待 DE-04 决策、序列化/过期触发延实现期） |
