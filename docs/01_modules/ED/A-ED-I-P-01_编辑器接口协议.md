---
文档编号：ED-I-P-01
文档状态：A
负责模块：ED
文档职责：ED 编辑器模块级接口协议（L2 接口层）——按 A-SYS-I-P-02 骨架声明 ED 的编辑会话 command、装饰层挂载/复位接口、blockId 定位接口、外部变更订阅；前端编辑器运行时与后端/上层模块的契约面
上游约束：A-SYS-I-P-02_IPC协议.md（命名/信封/错误/流式骨架）、A-ED-M-T-01_编辑器技术设计.md（BR-ED-* 规则、ED-EDIT/ED-DECOR 链路、X-FORBID-005）、A-ED-M-S-01_编辑器状态机设计.md（SM-ED-01 含 Conflicted 态）、A-WS-I-P-01_工作台接口协议.md（read_doc/save_doc/external_change 复用）、A-SYS-I-P-01_数据结构定义.md（DocRef）、ADU.md
直接承接：实现期前端编辑器（TipTap）与扩展点按本文契约注册；DE（diff 呈现）/AG（内联辅助）/REF（@mention）按装饰层接口挂载（X-FORBID-005：唯一受控通道）
使用边界：本文声明 ED 接口的**语义级签名**；文件 I/O 经 WS（不在本文重复，ED 经 A-WS-I-P-01）；不新增 RULE/CHAIN/CONSTRAINT/TERM；装饰层接口为进程内前端契约（非 Tauri command）与跨模块 command 混合，分节标注
变更要求：装饰层接口变更须复核 DE/AG/REF 消费方；编辑会话 command 变更须复核 SM-ED-01 一致性；blockId 契约变更联动 DE 定位
治理成熟度：draft
---

# Binder 编辑器接口协议（ED · L2 接口层）

## §1 文档定位

```
A-SYS-I-P-02（骨架）+ A-WS-I-P-01（文件 I/O 复用）→ 本文（ED 会话/装饰层/定位接口）→ 实现 + @GOV(追溯 A-ED-M-T-01)
```

ED 接口分两类：**（a）编辑会话 command**（前端 Store/Service ↔ 后端协同，经 WS 落盘）；
**（b）装饰层与定位接口**（前端进程内契约，供 DE/AG/REF 在编辑器上挂载呈现，X-FORBID-005
唯一受控通道）。后者非跨进程 Tauri command，但作为模块间契约在此声明。

## §2 编辑会话接口（ED-EDIT / SM-ED-01）

> 内容读写经 WS（`ws.read_doc`/`ws.save_doc`，A-WS-I-P-01）；以下为 ED 会话编排接口
> （前端编辑器运行时对外暴露）。

| 接口 | 参数（语义） | 结果 | 特有错误/状态 | 归属 |
|---|---|---|---|---|
| `ed.open_session` | DocRef | 会话句柄 + 编辑模型（md 直载 / docx 经 DOC） | 解析失败→只读降级（SM-ED-01 ReadOnly）；读取失败→会话不建立 | BR-ED-STATE-001 |
| `ed.close_session` | 会话句柄 | 确认 | 有脏态未处置→拒绝（须先 save 或 confirmDiscard） | BR-ED-STATE-001 |
| `ed.get_dirty_state` | 会话句柄 | 是否脏 + 当前状态（SM-ED-01 态） | — | X-INTER-001（驱动 SM-WS-01） |
| `ed.confirm_discard` | 会话句柄 | 确认丢弃脏态并关闭 | — | 显式确认语义 |
| `ed.resolve_conflict` | 会话句柄、选择（keep_mine / reload） | 重置态（Editing/Loading） | 非 Conflicted 态调用→协议错误 | SM-ED-01 Conflicted（BR-ED-STATE-003，对接 ws.save_doc override / external_change） |

> `ed.resolve_conflict(keep_mine)` 后续 `ws.save_doc` 须带 override=true（用户显式覆盖语义链）；
> `reload` 经 `ws.read_doc` 重载并刷新基线。

## §3 装饰层接口（ED-DECOR / BR-ED-STATE-002 / X-FORBID-005）

> 前端进程内契约，DE/AG/REF 为消费方；ED 是编辑器内部状态的唯一受控通道，消费方不得绕过。

| 接口 | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| `ed.mount_decoration` | 消费方标识、blockId、装饰描述（呈现型，不含正文写入） | 装饰句柄 | **anchor_lost**（blockId 失效，BR-ED-DATA-001） | DE diff 呈现 / AG 内联候选 |
| `ed.update_decoration` | 装饰句柄、新呈现描述 | 确认 | 句柄失效 | — |
| `ed.dismiss_decoration` | 装饰句柄 | 复位确认（不残留） | — | 消费终态后复位 |
| `ed.on_decoration_action` | 装饰句柄（订阅） | 用户交互事件（如接受/拒绝点击）转达消费方 | — | ED 不代裁（转达 DE 裁决 / AG 采纳） |

## §4 blockId 定位接口（ED-EDIT / BR-ED-DATA-001）

| 接口 | 参数（语义） | 结果 | 特有错误 | 归属 |
|---|---|---|---|---|
| `ed.locate_block` | 会话句柄、blockId | 定位结果（范围/可视位置） | **anchor_lost**（明确失效信号，不静默错位） | DE 定位 / REF 引用锚校验 |
| `ed.apply_edit` | 会话句柄、编辑操作（用户编辑语义） | 新编辑模型态 + 脏态 | 越权（绕过受控接口）→拒绝 | 采纳内联候选的落点（AG 内联辅助经此，BR-AG-STATE-003）；X-FORBID-005 唯一通道 |

> `ed.apply_edit` 是 inline-assist/autocomplete 采纳「作为用户编辑操作」进入模型的接口——
> 经此驱动 SM-ED-01 edit→SM-WS-01 脏态，区别于 diff 落盘路径（后者经 DE→WS）。

## §5 外部变更订阅（承 A-WS-I-P-01 §6）

ED 会话订阅 `ws.external_change` 事件：无脏态自动重载（经 `ws.read_doc` 刷新基线），有脏态进
Conflicted 态待 `ed.resolve_conflict`（BR-ED-STATE-003）。本文不重复事件定义（归 WS I-P）。

## §6 测试与验收

- 会话生命周期 command 覆盖 SM-ED-01 全转移（含 Conflicted 路：external_change/saveConflict→
  resolve_conflict 两分支）；装饰层 mount→update→dismiss 复位不残留 + anchor_lost 失效信号；
  `ed.apply_edit` 越过受控接口的调用被拒（X-FORBID-005 静态+集成核验）；与 DE/AG/REF G5 联调。

## §7 债务与盲区

- 装饰描述的呈现 schema（diff 高亮样式 / 内联候选样式）延实现期与 DE/AG 联合定。
- 编辑操作（apply_edit 的 op 语义）的粒度表延实现期（依 TipTap/ProseMirror 事务模型）。
- 装饰层接口是否需要跨进程化（如后端驱动呈现）延实现期，当前定为前端进程内契约。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（实现前置件）：ED 接口协议——编辑会话 command 5 条（含 resolve_conflict 对接 SM-ED-01 Conflicted）、装饰层接口 4 条（mount/update/dismiss/on_action，X-FORBID-005 唯一通道）、blockId 定位 2 条（locate_block 失效信号 / apply_edit 内联采纳落点）、外部变更订阅承 WS I-P；零新增注册项；债务（呈现 schema/编辑 op 粒度/装饰层进程边界延实现期） |
