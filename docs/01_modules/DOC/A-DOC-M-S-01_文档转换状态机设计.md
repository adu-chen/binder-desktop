---
文档编号：DOC-M-S-01
文档状态：A
负责模块：DOC
文档职责：DOC 文档转换模块级状态机设计——docx 开/存机（SM-DOC-01），含 APC §6 必备字段；登记 A-SYS-M-S-01 总图（docx 行 planned→active），登记 DOC↔ED / DOC↔WS X-INTER-001 跨模块转移契约
上游约束：A-DOC-M-T-01_文档转换技术设计.md（DOC-CONVERT 链路、BR-DOC-STATE-001/DATA-001/003 规则）、A-CORE-C-P-01_APC.md（§6）、A-SYS-M-S-01_状态机总图.md（X-SEQ-003/X-INTER-001、docx 开/存核心链）、A-ED-M-S-01_编辑器状态机设计.md（SM-ED-01 Loading 对接）、A-WS-M-S-01_工作台状态机设计.md（SM-WS-01 落盘对接）、ADU.md（§3.1）
直接承接：SM-DOC-01 经 rules 字段承载 A-DOC-M-T-01 规则；machine_id 登记总图；开/存转移点对接 SM-ED-01 / SM-WS-01
使用边界：本文定义 DOC 状态机内部；不重定义规则（归 A-DOC-M-T-01）、不定义 SM-ED-01 / SM-WS-01 内部；预览链转换为单次无状态转换调用（无多步状态化生命周期，APC §6 豁免，§4 说明）
变更要求：transitions 变更须同步 A-DOC-M-T-01 规则并重跑测试矩阵；跨模块转移点变更须与 ED/WS 共同复核；machine_id 一经登记不可更改
治理成熟度：draft
---

# Binder 文档转换状态机设计（DOC · L2 技术设计）

## §1 文档定位

```
A-DOC-M-T-01（规则 SSOT：DOC-CONVERT）→ 本文 SM-DOC-01
本文 machine_id ──登记──→ A-SYS-M-S-01 §3（docx 开/存行 planned→active）
开转移点 ──对接──→ SM-ED-01 Loading（X-INTER-001）；存转移点 ──对接──→ SM-WS-01 Saving（X-INTER-001）
```

满足 X-SEQ-003：docx 开/存为状态化核心链（APC §6 列举，owner=DOC），在此建机。

## §2 SM-DOC-01 · docx 开/存机

<!-- STATE_MACHINE
machine_id: SM-DOC-01
owner_module: DOC
status: active
summary: 单个 docx 文档自打开转换到保存转换往返的生命周期（失败不破坏原文件、无半成品）
source_type: design
chain: DOC-CONVERT
rules: BR-DOC-STATE-001, BR-DOC-DATA-001, BR-DOC-DATA-003
initial_state: Idle
terminal_states: Idle
-->

- **machine_id**：SM-DOC-01　**owner_module**：DOC
- **scope**：单个 docx 文档一次编辑会话期内的打开转换与保存转换生命周期（预览链不在本机，§4）
- **context**：目标 DocRef、转换方向、编辑模型/docx 字节流、样式子集告知、失败原因
- **states**：Idle、OpenConverting、Ready、SaveConverting、OpenError、SaveError
- **events**：openDocx、convertOk、convertFail、saveDocx、saveConvertOk、saveConvertFail、retry、close
- **transitions（主表）**：

| from | event | guard | to | action |
|---|---|---|---|---|
| Idle | openDocx | 路径在 workspace root 内（WS 侧守护） | OpenConverting | 经 WS 读取，Pandoc 转换为编辑模型 |
| OpenConverting | convertOk | — | Ready | 注入缺失 data-block-id；样式子集告知；交 SM-ED-01 装载（X-INTER-001） |
| OpenConverting | convertFail | — | OpenError | 统一错误形状；编辑会话不建立（对接 SM-ED-01 loadFail/parseFail） |
| OpenError | retry | — | OpenConverting | 重试转换 |
| OpenError | close | — | Idle | 放弃打开 |
| Ready | saveDocx | — | SaveConverting | 编辑模型→Pandoc→docx 字节流 |
| SaveConverting | saveConvertOk | WS 原子落盘成功（SM-WS-01 Saved，X-INTER-001） | Ready | 往返完成，原文件被新内容原子替换 |
| SaveConverting | saveConvertFail | — | SaveError | 统一错误形状；原 docx 文件保持不变 |
| SaveError | retry | — | SaveConverting | 重试保存转换 |
| SaveError | close | 经用户确认（脏态处置归 SM-ED-01） | Idle | 放弃本次保存 |
| Ready | close | — | Idle | 会话结束 |

- **error_states**：OpenError（recovery：retry→OpenConverting / close→Idle）；SaveError
  （recovery：retry→SaveConverting，原文件不变可重试）
- **recovery_paths**：OpenError→retry；SaveError→retry；任一失败态原文件均完好
- **forbidden_transitions**：
  - Idle → Ready（未经转换不得直接就绪）
  - OpenConverting → SaveConverting（未就绪不得保存）
  - SaveError → Ready 无 retry 成功（不得伪装已保存）
  - OpenError → Ready（转换失败不得伪装就绪）
- **acceptance_criteria**：唯一进入 Ready 的路径经 convertOk；保存失败后原 docx 与失败前一致
  （字节级核验）；无任一路径产生半成品落盘；失败均回统一错误形状且可重试
- **test_matrix**：见 §5

## §3 跨模块转移契约（X-INTER-001）

| 转移点 | 方向 | 契约 |
|---|---|---|
| docx 装载 | SM-DOC-01 convertOk → SM-ED-01 Loading→loadOk | DOC 转换成功交 ED 装载；convertFail 对接 SM-ED-01 loadFail（会话不建立）或 parseFail（降级只读），失败语义两侧一致 |
| docx 落盘 | SM-DOC-01 SaveConverting → SM-WS-01 Saving | 字节流经 WS 原子落盘；SM-WS-01 Saved 回执驱动 saveConvertOk，SaveError 驱动 saveConvertFail，原文件不破坏语义由 WS 保障（BR-WS-PERSIST-001） |

## §4 不独立建机的转换链（豁免说明，APC §6）

- **预览链转换**（docx→PDF，LibreOffice）：单次无状态转换调用（请求→成功/失败），无多步状态化
  生命周期与跨模块状态耦合，按 APC §6 豁免不建机；失败回统一错误形状由 PRV 呈现失败态
  （BR-DOC-DATA-001 约束其分离性）。若后续暴露排队/增量转换等状态化需求，再补机登记。

## §5 测试与验收（强制转移测试矩阵，APC §6/§10）

- **正路**：Idle→openDocx→OpenConverting→convertOk→Ready→saveDocx→SaveConverting→saveConvertOk→
  Ready→close→Idle（往返回归含必需样式子集保真断言）。
- **失败路**：convertFail→OpenError→retry / close；saveConvertFail→SaveError→retry，断言原 docx
  字节级不变。
- **禁止转移**：Idle/OpenConverting→Ready 旁路不可达；SaveError/OpenError 不得伪装就绪/已保存。
- **跨模块（G5 联调）**：convertOk↔SM-ED-01 装载、SaveConverting↔SM-WS-01 原子落盘、失败语义
  两侧一致。
- **离线**：断网下全链可用（REQ-DOC-005）。

## §6 总图登记（A-SYS-M-S-01）

| 核心链 | machine_id | 总图回填动作 |
|---|---|---|
| docx 开 / 存 | SM-DOC-01 | §3「docx 开 / 存｜DOC」行 planned→active，machine_id=SM-DOC-01 |
| DOC↔ED / DOC↔WS 转移 | — | §5 X-INTER-001 登记本文 §3 两个转移点 |

## §7 术语归属说明

状态机框架词不注册 TERM；样式子集（StyleSubset）已在 A-DOC-M-T-01 §0 注册；内容块/blockId 主责
归 ED，本文引用不抢注。

## §8 技术债务与盲区（诚实声明）

- **转换并发**：多 docx 同时打开/保存的并发实例隔离与转换器进程池策略延实现期。
- **大文件转换的进度与取消**：当前机无转换中取消事件；实现期如需取消（大文件长转换），补
  cancel 事件与中断终态（对齐 BR-SYS-STATE-002 中断语义）后回填本文。
- 预览链豁免依据基于"单次无状态调用"判断，实现期若引入排队/缓存状态则失效，须复核（§4）。

## §9 后续动作

1. 落地后同步编辑 A-SYS-M-S-01：§3 docx 行 planned→active（SM-DOC-01），§5 登记两个转移点。
2. 随 `npm run governance:generate` 复跑核对。
3. 待人类宣布 DOC G2 闭合（AI 不自行宣布晋级，APC §2）。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-11 | v1.0 | 初版（DOC G2 状态机件）：定义 SM-DOC-01 docx 开/存机（Idle/OpenConverting/Ready/SaveConverting/OpenError/SaveError，含 APC §6 全部必备字段；失败不破坏原文件、无半成品、可重试）；DOC↔ED 装载与 DOC↔WS 落盘 X-INTER-001 转移契约；预览链单次无状态调用豁免说明；满足 X-SEQ-003；测试矩阵与债务（并发/取消/豁免复核条件） |
