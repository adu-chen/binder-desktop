---
文档编号：DE-X-L-01
文档状态：A
负责模块：DE
文档职责：DE Diff Edit G3 开发计划——承接型 Plan Item，承接 DE-DIFF 链路与 BR-DE-* 规则、SM-DE-01（diff 终态机），明确文件范围/@GOV/A 类债务/验收；核心交互「对话→diff→裁决→落盘」的 DE 侧实现
上游约束：A-CORE-X-L-02（P3）、APC §5/§6/§12、A-DE-M-T-01/A-DE-M-S-01、A-DE-I-P-01、A-WS-X-L-01（apply_diff 落盘）、A-ED-X-L-01（装饰层呈现）、A-AG（产出消费）、A-CORE-X-R-01
直接承接：本计划驱动 DE 实现；@GOV 追溯 BR-DE-*；SM-DE-01 转移矩阵
使用边界：L3 承接型 Plan Item；持久化/落盘经 WS、呈现经 ED（不重复）；@GOV 在实现代码；NFR 延 G5
变更要求：PI 增删保持承接可追溯；裁决接口变更复核 SM-WS-02/SM-AG-01 对接
治理成熟度：draft
---

# Binder Diff Edit 开发计划（DE · L3）

## §1 文档定位

DE 是核心交互中枢（接 AG 产出→生成 diff→裁决→驱动 WS 落盘）。本计划承接 SM-DE-01 与 BR-DE-*，
依赖 WS（apply_diff）、ED（装饰层）、AG（ChangeProposal 来源）。

## §2 Plan Item 清单

### PI-DE-01　diff 生成与载荷　✅ G4 已实现（2026-06-22，generate.rs build_payload 统一形状新建空基准 + generate_diff 锚失效 GenFailed 回报，ChangeProposal DE 单一拥有，+3 covers，audit 绿）
- **承接**：REQ-DE-001/004 / CHAIN DE-DIFF / BR-DE-DATA-001（生成与锚）/ BR-DE-DATA-002（payload
  载荷形状）。
- **文件范围**：`src-tauri/src/de/generate.rs`（ChangeProposal→PendingDiff，blockId 锚定经
  `ed.locate_block`，持久化经 `ws.persist_diff`）；`de.generate_diff/get_diff/list_diffs`。
- **@GOV**：生成与锚失效回报 `BR-DE-DATA-001`；载荷形状 `BR-DE-DATA-002`。
- **A 类债务**：diff 粒度（DE-01，块级裁决基线已定，行/字符级对比为呈现增强，实现期）；payload
  序列化与 before_snapshot 存储（实现期，承 A-SYS-I-P-01）；diff 拆分策略（与 AG 生成侧联合调优）。
- **验收**：建议生成持久化 PendingDiff 定位准确；锚失效→GenFailed 回报 AG（handoffFail）；
  持久化→重启恢复后审阅可重建（before_snapshot 基准）。

### PI-DE-02　diff 终态机与裁决　✅ G4 状态机已实现（2026-06-22，cargo test SM-DE-01 矩阵 + @GOV + audit 绿；diff 生成/载荷 PI-DE-01 与过期冲突 PI-DE-03 待续）
- **承接**：REQ-DE-002/003 / SM-DE-01（Generating/Pending/Accepted/Rejected/Expired/GenFailed）/
  BR-DE-STATE-001（终态裁决）。
- **文件范围**：`src-tauri/src/de/adjudicate.rs`（终态转移；accepted→`ws.apply_diff`、回执处理；
  rejected/expired→`ws.discard_diff`；装饰复位经 `ed.dismiss_decoration`）；`de.accept_diff/
  reject_diff`；用户裁决经 `ed.on_decoration_action` 转达。
- **@GOV**：终态裁决与唯一落盘触发 `BR-DE-STATE-001`。
- **A 类债务**：**审阅 UI 形态（DE-04，C 类待人类）**——逐条/批量、侧栏/内联，实现期定（接口已留
  `de.accept_batch` 扩展位）。
- **验收（强制，APC §6）**：SM-DE-01 矩阵——三终态互斥不可逆；accepted 唯一触发 `ws.apply_diff`；
  ApplyError 不回退终态；rejected/expired 文件不变 + 装饰复位；终态后再裁决被拒；X-INTER-001
  （DE↔WS 落盘、AG→DE 交接）G5 联调。

### PI-DE-03　过期与冲突重检　✅ G4 已实现（2026-06-22，cargo test 保守过期/重叠重检 + 2 @GOV + audit 绿）
- **承接**：REQ-DE-005 / BR-DE-STATE-002（过期保守）/ BR-DE-DATA-003（多 diff 冲突重检）。
- **文件范围**：`src-tauri/src/de/expiry.rs`（超时 + 外部变更检测，订阅 `ws.external_change`；先序
  落盘后重叠 diff 重检）；`de.check_expiry/recheck_overlap`。
- **@GOV**：过期保守语义 `BR-DE-STATE-002`；冲突重检不覆盖新内容 `BR-DE-DATA-003`。
- **A 类债务**：超时时长 + 检测触发时机（DE-03，定时/呈现时/裁决时，实现期调度）。
- **验收**：超时/外部变更（块内容≠before_snapshot）转 Expired；检测不可得保守维持 Pending（不误杀）；
  先序 diff 落盘后重叠 pending diff 重检转 Expired（不覆盖新内容、不并发落盘同一块）。

### PI-DE-04　DiffCard 紧凑审阅卡　🚧 G4 实现中（2026-07-08，DE-04 局部闭合）
- **承接**：A-DE-M-T-01 §8.1 / TERM-DE-003 / BR-DE-DATA-002（before/after 审阅重建）/
  BR-DE-STATE-001（状态与裁决操作互斥）。
- **文件范围**：`src/components/de/DiffCard.tsx`（标题/位置/红删/绿增/操作五行结构 + 展开详情区）；
  `src/styles.css`（默认态单行省略、受控行高、详情区样式）；`src/components/de/DiffCard.test.tsx`（结构、
  长文本、状态详情、样式摘要断言）。
- **@GOV**：沿用现有 DiffCard 呈现规则链，不新增运行时规则；测试继续以 `BR-DE-DATA-002` 与
  `BR-DE-STATE-001` 覆盖审阅重建和状态互斥。
- **A 类债务**：批量裁决、侧栏/内联整体布局仍属 DE-04 剩余项；本 PI 只闭合逐条默认卡片形态。
- **验收**：默认态固定为标题行、位置行、红删行、绿增行、操作按钮行；长 before/after 默认单行省略，
  展开后显示完整内容；状态提示、进度和样式摘要不无界撑高默认卡片；组件测试、build、audit 绿。

## §3 实施顺序

```
PI-DE-01（生成，依赖 ED locate_block + WS persist_diff）→ PI-DE-02（终态机，接 WS apply_diff）
→ PI-DE-03（过期/冲突，订阅 ws.external_change）→ PI-DE-04（逐条 DiffCard 默认形态）
```

## §4 横切验收

audit 绿；@GOV 追溯 BR-DE-*；SM-DE-01 矩阵全覆盖；`// covers: BR-DE-*` 覆盖 DE 全部规则（5 条）
有断言；核心交互链 AG→DE→WS→ED G5 联调（对话→diff→裁决→落盘端到端）。

## §5 承接的 A 类债务（A-CORE-X-R-01 §6）

| 债务 | PI | 处置 |
|---|---|---|
| diff 粒度/拆分策略（DE-01） | PI-DE-01 | 实现期（与 AG 联合） |
| payload 序列化/before_snapshot 存储 | PI-DE-01 | 实现期 |
| 审阅 UI 形态（DE-04，C 类） | PI-DE-02/04 | 逐条默认卡片形态由 PI-DE-04 局部闭合；批量/侧栏/内联仍待实现期 |
| 超时时长/检测触发时机（DE-03） | PI-DE-03 | 实现期调度 |
| diff 生成/呈现延迟 NFR | 全 PI | G5 标定 |

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-06-12 | v1.0 | 初版（G3，P3 核心交互 DE 侧）：DE 3 条 Plan Item（生成与载荷 / 终态机与裁决 / 过期与冲突重检），承接 DE-DIFF + 5 规则 + SM-DE-01；accept 唯一触发 ws.apply_diff 不可逆、ApplyError 不回退；DE-04 审阅 UI 为 C 类（接口留 batch 扩展位）；文件范围/@GOV/验收齐 |
| 2026-07-04 | v1.1 | 术语统一清扫（承 2026-07-04 人类三轮裁决）：正文禁用别名替换——内联辅助→局部修改、自动补全→辅助续写、对话编辑→Diff Edit、Chat↔Build 接管→构建移交/回落、chat/chat-agent 回合→代理回合；文件名与变更记录历史行不改写 |
| 2026-07-08 | v1.2 | 新增 PI-DE-04 DiffCard 紧凑审阅卡：承接 A-DE-M-T-01 §8.1，锁定逐条默认卡片五行结构、长内容单行省略与展开详情区；DE-04 剩余项收敛为批量裁决、侧栏/内联整体布局 |
