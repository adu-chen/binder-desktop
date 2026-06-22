---
文档编号：REF-X-L-01
文档状态：A
负责模块：REF
文档职责：REF 引用 G3 开发计划——承接型 Plan Item，承接 REF-INJECT 链路与 BR-REF-* 规则，明确文件范围/@GOV/A 类债务/验收（REF 无状态机，豁免成立）
上游约束：A-CORE-X-L-02（P3）、APC §5/§6/§12、A-REF-M-T-01、A-REF-I-P-01、A-WS-X-L-01（read/search）、A-ED-X-L-01（locate_block 锚校验、扩展点）、A-AG-X-L-01（注入消费）、A-CORE-X-R-01
直接承接：本计划驱动 REF 实现；@GOV 追溯 BR-REF-*
使用边界：L3 承接型 Plan Item；读取经 WS、锚校验经 ED（不重复）；@GOV 在实现代码
变更要求：PI 增删保持承接可追溯
治理成熟度：draft
---

# Binder 引用开发计划（REF · L3）

## §1 文档定位

REF 是 @mention 引用解析与注入，无状态机（豁免成立，A-REF-M-T-01 §5）。承接 REF-INJECT 与
BR-REF-*，依赖 WS（读取）、ED（锚/扩展点）、AG（消费注入）。

## §2 Plan Item 清单

### PI-REF-01　@mention 触发与目标解析　✅ G4（resolve.rs @mention→ReferenceAnchor 限 workspace/越界拒绝/不可达失败不悬空 BR-REF-DATA-001）
- **承接**：REQ-REF-001/004 / CHAIN REF-INJECT / BR-REF-DATA-001。
- **文件范围**：`src/`（@ 触发浮层经 ED 扩展点）/`src-tauri/src/ref/resolve.rs`（候选经
  `ws.list_docs`/`ws.search` 限界）；`ref.list_candidates/resolve_anchor`。
- **@GOV**：触发与解析（限界/无悬空/经 ED 扩展点）`BR-REF-DATA-001`。
- **A 类债务**：候选排序与搜索匹配算法（实现期）；**KB/AST 纳入候选（REF-01，待 KB/AST 联合，
  本 PI v1 仅 workspace 目标）**。
- **验收**：候选限 workspace 内；目标不可达明确失败无悬空引用；触发/候选/标记经 ED 扩展点
  （X-FORBID-005，不绕过）。

### PI-REF-02　锚校验与注入载荷组装　✅ G4（inject.rs verify_anchor 保守失效不静默注入 BR-REF-VERIFY-001 + build_injection 读取时点内容可溯源 RefInjection BR-REF-DATA-002）
- **承接**：REQ-REF-002/003 / BR-REF-DATA-002（注入载荷）/ BR-REF-VERIFY-001（锚失效保守）。
- **文件范围**：`src-tauri/src/ref/inject.rs`（锚校验经 `ed.locate_block`；读取时点内容经
  `ws.read_doc`；组装 RefInjection 供 AG）；`ref.verify_anchor/build_injection`。
- **@GOV**：注入组装（读取时点/来源可溯/不私扩字段）`BR-REF-DATA-002`；锚失效校验（保守不注入）
  `BR-REF-VERIFY-001`。
- **A 类债务**：锚失效修复交互（REF-03，重选/自动重定位，实现期）。
- **验收**：注入内容与读取时点一致、含来源标识、结构同 A-SYS-I-P-01 RefInjection；锚失效发明确
  信号不静默注入空/过期/错位；检测不可得保守不注入。

## §3 豁免复核（承 A-REF-M-T-01 §5）

REF 无状态机（请求-响应式）。**G3/G4 复核失效触发条件**：若实现引入异步批量解析/索引重建/后台
失效扫描，则触发建机（X-SEQ-003），须补 M-S 并登记总图。本 PI v1 不引入，豁免维持。

## §4 横切验收

audit 绿；@GOV 追溯 BR-REF-*；`// covers: BR-REF-*` 覆盖 REF 全部规则（3 条）有断言；豁免复核
结论记录。

## §5 承接的 A 类债务（A-CORE-X-R-01 §6）

| 债务 | PI | 处置 |
|---|---|---|
| KB/AST 纳入候选（REF-01） | PI-REF-01 | 待 KB/AST 联合，v1 仅 workspace |
| 锚失效修复交互（REF-03） | PI-REF-02 | 实现期 |
| 候选排序/匹配算法 | PI-REF-01 | 实现期 |
| 豁免失效触发复核 | §3 | G3/G4 复核 |
| 候选/解析延迟 NFR | 全 PI | G5 标定 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3，P3）：REF 2 条 Plan Item（触发与解析 / 锚校验与注入组装），承接 REF-INJECT + 3 规则；无状态机（豁免维持，§3 复核失效条件）；只读注入（RefInjection 内容副本）；文件范围/@GOV/验收齐 |
