# ADP.md — AI Development Process 运行时协议

## 0. 文档边界

ADP 定义单次开发任务的过程约束。

ADP 只回答：
- 当前任务是什么类型
- 当前任务属于哪个治理阶段
- 当前任务处于什么状态
- 是否需要 Issue Trace
- Issue Trace 后如何形成 Scope Lock
- 是否满足 APC 定义的阶段门禁
- 何时停止、何时恢复、何时可交付
- 完成时输出什么最小运行报告

ADP 不定义：
- RULE / CHAIN / MODULE / CONSTRAINT / @GOV / triage 的静态规则（见 ADU.md）
- 需求、设计、计划文档的项目级完整性标准（见 APC）
- Issue Trace 十一项内容和触发判断（见 ADU.md §6）
- AI 会话启动和文档读取入口（见 CLAUDE.md）
- ADUS.md 自动生成区内容

CLAUDE.md 负责调用 ADP。ADU 是 ADP 的静态规则上游。

---

## 1. 任务类型

任务类型用于决定是否需要 Issue Trace 和 Scope Lock。

```
CODE_CHANGE          修改业务代码或已有 @GOV 覆盖的代码
CODE_CHANGE_MINOR    满足 §1.2 全部客观判据的小型代码修改（轻量路径）
GOVERNANCE_CHANGE    修改治理标注、triage、规则引用或治理文档
DOC_ONLY             只修改普通文档，不修改代码
TEST_ONLY            只修改测试文件
INDEX_ONLY           只刷新 ADUS.md 或运行生成/审计脚本
```

判定规则：
- 任务实际内容超出当前 task_type 时，必须停止并重新判定 task_type。
- 同时涉及多类任务时，按约束更强的类型执行：`CODE_CHANGE` > `CODE_CHANGE_MINOR` > `GOVERNANCE_CHANGE` > `TEST_ONLY` / `DOC_ONLY` / `INDEX_ONLY`。
- `CODE_CHANGE` 默认需要 Issue Trace 和 Scope Lock。
- `CODE_CHANGE_MINOR` 走 §1.2 轻量路径：不要求完整 Issue Trace 与人类 confirmation，但仍受轻量 Scope 声明与最小验证约束。
- `DOC_ONLY`、`TEST_ONLY`、`INDEX_ONLY` 使用隐式 Scope，不需要 Issue Trace。

### 1.1 治理阶段

governance_phase 用于判断当前任务处于哪个治理阶段。
它不替代 task_type；同一个 task_type 可以发生在不同治理阶段。

```
REQUIREMENT      需求文档治理
DESIGN           技术设计文档治理
PLAN             开发计划治理
IMPLEMENTATION   代码实现
VALIDATION       验证、审计、回归
NONE             无法判断或不适用
```

判定规则：
- `DOC_ONLY` 可以对应 REQUIREMENT、DESIGN 或 PLAN。
- `CODE_CHANGE` 通常对应 IMPLEMENTATION。
- `TEST_ONLY` 通常对应 IMPLEMENTATION 或 VALIDATION。
- `INDEX_ONLY` 通常对应 VALIDATION。
- 无法判断治理阶段且阶段会影响门禁时，触发 `SC-05 AMBIGUOUS_TASK`。
- `CODE_CHANGE_MINOR` 通常对应 IMPLEMENTATION 或 VALIDATION（修复完善阶段）。

### 1.2 变更比例原则

治理开销必须与变更的风险和规模成比例。
小型修复套用完整 `CODE_CHANGE` 仪式（完整 Issue Trace + 人类 confirmation）会产生
与收益不匹配的开销，反而诱发对治理流程的整体绕过。为此定义轻量路径与会话内快速通道。
轻量不等于无治理：判据、轻量 Scope 声明和最小验证仍然强制。

**CODE_CHANGE_MINOR 客观判据（必须全部满足，任一不满足即升级为 CODE_CHANGE）：**

1. 改动落在单一已存在文件，或一组已属于同一已确认 Scope 的文件内
2. 不新增导出符号（exported function / class / component），不新增 @GOV 块
3. 不新增或修改任一 RULE / CHAIN / CONSTRAINT / TERM 静态定义
4. 不改变已有 @GOV 块的 boundary 语义（in / out / effect / does_not 承载实体不变）
5. 不引入新的业务规则判断，改动落在既有规则与链路覆盖范围内
6. 不触及治理文件或测试治理能力（ADU.md / ADP.md / AGENTS.md / CLAUDE.md / ADUS.md /
   APC / governance.config.json / test_governance_path 指向的注册表与脚本）

判据由客观事实判定，不由 AI 主观裁量"是否够小"。存疑时按 `CODE_CHANGE` 处理。

**CODE_CHANGE_MINOR 轻量流程：**
- 不要求完整 Issue Trace，也不要求人类 confirmation 门禁
- 仍必须：用一句话声明改动文件与意图（轻量 Scope 声明）；
  进入 `VALIDATING` 时运行相关测试与 `npm run governance:audit`
- audit 出现 error 级违规 → 回到 `PATCHING` 或按对应 Stop Condition 处理
- 输出 §6.1 紧凑版 Runtime Report

**会话内快速通道（in-session fast-path）：**
- 同一会话已对某 Scope 完成 Issue Trace 且 `confirmation: YES` 后，
  在该 `allowed_files` 范围内的后续迭代修改不重复进入 INTAKE、不重新执行 Issue Trace
- 改动超出 `allowed_files` → 按 §3.3 Scope 扩展处理
- 范围未变但需要新增静态定义（RULE / CHAIN / TERM 等）→ 仍触发对应 Stop Condition

---

## 2. 最小状态机

```
INTAKE       任务已接收，正在判定 task_type 和目标
SCOPED       修改范围已确定，可进入修改
PATCHING     正在修改
VALIDATING   正在验证
READY        任务完成，可交付人类审阅
BLOCKED      触发 Stop Condition，等待解除
ABANDONED    人类终止任务
```

状态推进：

```
INTAKE -> SCOPED -> PATCHING -> VALIDATING -> READY
任意状态 -> BLOCKED
BLOCKED -> 原状态（阻断解除后）
BLOCKED -> ABANDONED（人类终止）
```

死线：
- Stop Condition 未解除时，final_state 只能是 `BLOCKED`，不得写 `READY`。
- `READY` 只能表示无未解除 Stop Condition。
- 需要人类裁决但不阻断交付的事项，必须写入 Task Runtime Report 的 `unresolved_items`；若该事项影响正确性、范围或治理闭环，则必须保持 `BLOCKED`。

---

## 3. Scope

### 3.1 显式 Scope

`CODE_CHANGE` 必须先执行 Issue Trace。Issue Trace 的内容和触发判断见 ADU.md §6。

`CODE_CHANGE_MINOR`（§1.2）不执行完整 Issue Trace，使用轻量 Scope 声明：
一句话列出改动文件与意图，无须人类 confirmation。一旦任一 §1.2 判据被打破，
立即升级为 `CODE_CHANGE` 并回到本节执行完整 Issue Trace 与 Scope Lock。

进入 `SCOPED` 前必须执行 Phase Gate Check。
Phase Gate Check 不新增 ADP 状态，但属于 INTAKE 到 SCOPED 的必经检查。

Phase Gate Check 最小检查：
1. 当前任务的 governance_phase
2. APC 是否对该 phase 定义前置门禁
3. 前置门禁是否满足
4. 是否存在映射链阻断
5. 当前 task_type 是否允许进入该 phase

Phase Gate Check 失败时，进入 `BLOCKED`。

**新概念注册前置判断（TERM 前置门禁）：**

在 Issue Trace 完成、进入 Scope Lock 之前，必须确认：

1. 任务描述或涉及的设计文档中是否出现 ADUS 术语注册表之外的业务名词
2. 若出现且满足 TERM 注册条件（跨 session 容易漂移、代码中有实体承载、命名混乱会导致规则判断错误），按是否实际引用分两档处理：
   → 硬阻断档（本次代码将用 `@GOV term_ref` 实际引用该未注册术语）：
     必须先在设计文档 Terminology Registry 添加 `<!-- TERM -->` 块，
     运行 `npm run governance:generate` 刷新 ADUS 后再继续；未注册即 `SC-02`（TERM_UNREGISTERED）。
   → 非阻断档（仅在文字/命名中提及，代码不写 term_ref 引用）：
     不阻断本次任务，将"建议注册的候选术语"写入 Task Runtime Report 的 `unresolved_items`，
     与 CLAUDE.md §12 新概念软提示一致。
3. 若已注册 → 后续生成代码时使用正式 en 名称，不使用 forbidden 列表中的别名
4. 新增 exported type / interface / class / enum 时，检查名称是否在 ADUS 术语注册表中
   → 不在注册表中 → 判断是否需要注册 TERM，或确认是技术实现词汇不需注册

Issue Trace 产出的文件范围结论是 Scope Lock 的输入：

```
task_type: CODE_CHANGE
allowed_files:
  - [文件路径] # [修改意图]
forbidden_files:
  - [文件路径] # [禁止原因]
confirmation: PENDING | YES
scope_reason: [一句话说明范围依据]
```

`confirmation: PENDING` 时不得修改文件。
`confirmation: YES` 后只能修改 allowed_files。
`confirmation` 字段只能由人类将 PENDING 改为 YES，AI 不得自行修改。

**编辑时刻强制（可选钩子）：**

`scripts/scope_lock_guard.mjs` 作为 Claude Code PreToolUse 钩子（`.claude/settings.json` 中按
`Edit|Write|MultiEdit` 匹配），在每次写文件前强制 Scope Lock。它读取项目根的
`.governance/active_scope.json`——Scope Lock 块的机器可读镜像：

```json
{
  "task_type": "CODE_CHANGE",
  "allowed_files": ["src/foo.ts"],
  "forbidden_files": ["src/baz.ts"],
  "confirmation": "PENDING | YES",
  "scope_reason": "一句话说明范围依据"
}
```

行为：
- 文件不存在（无活跃 Scope Lock）→ 放行，不干扰隐式 Scope 与 `CODE_CHANGE_MINOR`。
- `confirmation` 非 `YES` → 阻断所有文件编辑。
- 目标文件不在 `allowed_files` 或命中 `forbidden_files` → 阻断（SC-01）。

`CODE_CHANGE` 建立 Scope Lock 时应同步写入该文件，`confirmation` 字段同样只能由人类改为 `YES`。

### 3.2 隐式 Scope

以下任务不需要 Issue Trace，但仍必须遵守隐式 Scope：

| task_type | 允许范围 |
|-----------|----------|
| DOC_ONLY | 文档文件 |
| TEST_ONLY | 测试文件 |
| INDEX_ONLY | ADUS.md 自动生成区或生成/审计输出 |
| GOVERNANCE_CHANGE | 人类明确要求修改的治理文件或治理标注 |

隐式 Scope 任务若需要修改范围外文件，必须停止并重新判定 task_type。

### 3.3 Scope 扩展

需要修改 allowed_files 之外的文件时：
1. 进入 `BLOCKED`
2. 输出 `SCOPE_VIOLATION`
3. 说明新增文件和原因
4. 等待人类确认
5. 更新 Scope 后回到原状态

---

## 4. Stop Conditions

Stop Condition 触发时，任务进入 `BLOCKED`。

| 编号 | 名称 | 触发条件 | 解除方式 |
|------|------|----------|----------|
| SC-01 | SCOPE_VIOLATION | 需要修改 Scope 外文件 | 人类确认扩展 Scope |
| SC-02 | MISSING_STATIC_RULE | 缺少 ADU 定义的规则、链路、模块、triage 判断；或代码 @GOV 以 `term_ref` 实际引用了未在 ADUS 术语注册表登记的 TERM（TERM_UNREGISTERED）。仅在文字/命名中提及未注册名词、代码未写 term_ref 引用的，不触发本条，按 §3.1 非阻断档记入 unresolved_items | 人类补充静态定义或确认处理方式 |
| SC-03 | DESIGN_CODE_CONFLICT | 设计文档与代码实现冲突，AI 无法裁决 | 人类裁决设计或代码哪个需要修改 |
| SC-04 | VALIDATION_BLOCKED | 测试、审计或生成无法通过，且当前 Scope 内无法修复 | 人类确认扩展 Scope、接受风险或调整任务 |
| SC-05 | AMBIGUOUS_TASK | 任务目标、范围或验收标准无法确定 | 人类澄清任务 |
| SC-06 | PHASE_GATE_BLOCKED | APC 定义的前置阶段门禁未满足；当前任务要求进入的治理阶段缺少必要前置产物；人类要求越级进入后续阶段；需求、设计、计划任一前置阶段被标记 blocked；APC 要求状态机但状态机设计缺失 | 补齐 APC 要求的前置文档或门禁产物；修正当前任务阶段；人类明确调整任务目标到允许阶段；对阻断项作出项目规则允许的延期、豁免或裁决 |
| SC-07 | MAPPING_CHAIN_BROKEN | REQ 无法追溯到技术规则且当前任务要求进入设计之后阶段；技术规则未注册却被计划、代码或测试引用；开发计划条目没有规则来源；@GOV 引用无法追溯到已注册 RULE / CHAIN / CONSTRAINT；测试 covers 引用未知规则；候选规则被用于运行时代码 | 补齐 REQ_MAP 或等价映射；补齐 RULE / CHAIN / CONSTRAINT；将候选规则升级为正式规则；将映射状态改为 deferred / blocked / superseded；人类裁决缩小任务范围或回退阶段 |

ADU 中的 `RULE_MISSING`、`NEEDS_HUMAN_DECISION` 等治理分类，是 Stop Condition 的触发原因，不单独扩展 ADP 状态。
`SC-02 MISSING_STATIC_RULE` 表示缺少静态定义本身。
`SC-07 MAPPING_CHAIN_BROKEN` 表示静态定义可能存在，但追踪链断裂。

---

## 5. 验证

进入 `VALIDATING` 时，根据任务类型执行最小验证：

| task_type | 最小验证 |
|-----------|----------|
| CODE_CHANGE | 运行 `npm run test:plan -- --files [changed_files]` 审阅变更映射，再运行同一输入的 `npm run test:related -- --files [changed_files]`；选择器按模块、直接消费者与风险自动决定定向或全量测试，并串联治理审计。若本次修改了设计文档标注块（RULE / CHAIN / MODULE / TERM / CONSTRAINT），必须先运行 `npm run governance:generate`；若涉及规则有 `semantic_review_candidates` 标记，必须执行 AI 语义比对（ADU.md §5.7），结论写入 Task Runtime Report |
| CODE_CHANGE_MINOR | 运行 `npm run test:related -- --files [changed_files]`。按 §1.2 判据不应新增标注块或静态定义，故无须 generate；若选择器无法映射可执行文件或 audit 出现 error 级违规，回到 PATCHING 或升级为 CODE_CHANGE |
| GOVERNANCE_CHANGE | 运行 `npm run test:governance` 与 `npm run governance:audit`；若修改了设计文档标注块，必须先运行 `npm run governance:generate` |
| TEST_ONLY | 运行 `npm run test:related -- --files [changed_test_files]`，默认直选变更测试，不扩散成虚构的业务源影响 |
| DOC_ONLY | 检查文档头、引用关系和变更记录 |
| INDEX_ONLY | 运行 `npm run governance:generate` 后检查 ADUS.md 是否与输入一致 |

验证失败且当前 Scope 内可修复时，回到 `PATCHING`。
验证失败且当前 Scope 内不可修复时，触发 `SC-04 VALIDATION_BLOCKED`。

根据 governance_phase 执行补充验证：

| governance_phase | 补充验证 |
|------------------|----------|
| REQUIREMENT | 检查文档头、REQ 标注和变更记录；若 APC 要求流程图或完整性清单，按 APC 检查 |
| DESIGN | 检查 RULE / CHAIN / CONSTRAINT / REQ_MAP；若 APC 要求状态机或 SSOT，按 APC 检查 |
| PLAN | 检查计划条目是否引用技术规则；检查实现顺序、文件范围和测试路径 |
| IMPLEMENTATION | 检查 Issue Trace、Scope Lock、@GOV、`governance:audit` 和相关测试 |
| VALIDATION | 检查生成、审计、测试结果和未解除 Stop Condition |
| NONE | 不执行阶段补充验证；若阶段影响门禁，应先触发 `SC-05 AMBIGUOUS_TASK` |

具体 @GOV、triage、日志、Issue Trace 内容的判断标准来自 ADU.md，不在 ADP 重写。

`test:plan` / `test:related` 的选择结果是运行时证据，不是永久测试清单。每次变更都必须以实际
`changed_files` 重新计算；不得复制上一次任务的测试列表。选择器失败关闭时视为 SC-04，除非根因是
缺失模块、规则或链路定义，此时按 SC-02 / SC-07 处理。

### 5.1 越级请求处理

当人类请求的任务目标属于 IMPLEMENTATION，
但 Phase Gate Check 发现 REQUIREMENT / DESIGN / PLAN 前置门禁未满足时，
AI 不得进入 PATCHING。

处理流程：
1. 判定 task_type 和 governance_phase
2. 执行 Phase Gate Check
3. 若失败，触发 `SC-06 PHASE_GATE_BLOCKED` 或 `SC-07 MAPPING_CHAIN_BROKEN`
4. 输出缺口列表
5. final_state 为 `BLOCKED`

---

## 6. Task Runtime Report

任务结束时必须输出最小运行报告。

```markdown
## Task Runtime Report — [任务简述]

task_type: [CODE_CHANGE | CODE_CHANGE_MINOR | GOVERNANCE_CHANGE | DOC_ONLY | TEST_ONLY | INDEX_ONLY]
governance_phase: [REQUIREMENT | DESIGN | PLAN | IMPLEMENTATION | VALIDATION | NONE]
final_state: [READY | BLOCKED | ABANDONED]
issue_trace: [无 | Issue Trace 路径或 ID]

changed_files:
  - [文件路径] # [新增/修改/删除，简述]

scope:
  mode: [explicit | implicit]
  status: [confirmed | skipped | blocked]

phase_gate:
  status: [passed | blocked | skipped]
  evidence: [一句话说明依据]

mapping_chain:
  status: [complete | partial | blocked | not_applicable]
  evidence: [REQ/RULE/CHAIN/@GOV/Test 的最小说明]

test_impact:
  changed_files: [输入选择器的文件]
  direct_modules: [直接归属模块]
  affected_modules: [含风险扩展后的模块]
  selection: [定向测试路径 | frontend-full | rust-full | not_applicable]

validation:
  - [命令或检查项] # [passed | failed | skipped，原因]

stop_conditions:
  - [无 | SC-xx: 原因与解除方式]

recovery:
  - [无 | BLOCKED 解除动作、验证命令和结果]

unresolved_items:
  - [无 | 等待人类处理的非阻断事项]
```

若 `stop_conditions` 存在未解除项，`final_state` 必须是 `BLOCKED`。

### 6.1 紧凑版 Runtime Report

`CODE_CHANGE_MINOR` 及隐式 Scope 任务（DOC_ONLY / TEST_ONLY / INDEX_ONLY）
可输出紧凑版，省略 phase_gate / mapping_chain / scope / recovery 等对小型变更冗余的字段：

```markdown
## Task Runtime Report — [任务简述]

task_type: [CODE_CHANGE_MINOR | DOC_ONLY | TEST_ONLY | INDEX_ONLY]
final_state: [READY | BLOCKED]
changed_files:
  - [文件路径] # [简述]
validation:
  - [命令或检查项] # [passed | failed | skipped]
unresolved_items:
  - [无 | 等待人类处理的非阻断事项]
```

适用条件：
- 仅当任务满足 `CODE_CHANGE_MINOR` 客观判据（§1.2）或属于隐式 Scope 任务时可用。
- 一旦触发任一 Stop Condition，或升级为完整 `CODE_CHANGE`，必须改用 §6 完整报告。
- `final_state` 为 `BLOCKED` 时，紧凑版必须补充 `stop_conditions` 字段说明阻断项。

---

## 7. 交接点

| 方向 | 内容 |
|------|------|
| CLAUDE.md -> ADP.md | CLAUDE 读取 ADP，并在开发任务开始前进入 INTAKE |
| AGENTS.md -> ADP.md | AGENTS 读取 ADP，并在开发任务开始前进入 INTAKE |
| ADP.md -> ADU.md | ADP 引用 ADU 的静态规则、Issue Trace 内容、@GOV、triage、日志和测试覆盖判断 |
| ADP.md -> ADUS.md | ADP 使用 ADUS.md 查询规则、链路、模块、代码块和动态测试能力索引 |
| ADP.md -> test_governance_path | ADP 使用项目测试注册表进行模块归属、风险扩展、文件预算和测试选择 |
| ADP.md -> package scripts | ADP 通过 `test:plan` / `test:related` / `test:governance` 与 `governance:generate` / `governance:audit` 调用工具链 |

---

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-05-19 | v0.1 | 初始草稿，定义任务状态机、七个 Gate、六个 Stop Condition、Scope Lock 格式、Task Runtime Report 格式、豁免规则与 ADU/CLAUDE.md 交接点 |
| 2026-05-20 | v0.2 | 收敛为最小过程协议；压缩状态机和任务类型；删除过细 Gate 表；修复 READY/BLOCKED 冲突；增加隐式 Scope；统一脚本入口为 package scripts；明确 CLAUDE 入口、ADU 静态规则、adus 索引的边界 |
| 2026-05-21 | v0.3 | §3.1 新增 TERM 前置门禁：进入 Scope Lock 前必须确认新概念是否已注册，未注册需先补充 TERM 块并刷新 ADUS |
| 2026-05-22 | v0.4 | §3.1 补充 confirmation 字段只能由人类修改；§4 SC-02 扩展触发条件覆盖 TERM_UNREGISTERED；§5 将「必要时运行 generate」改为明确触发条件（修改了设计文档标注块时必须运行） |
| 2026-05-22 | v0.5 | §5 CODE_CHANGE 验证项补充语义审计触发条件：涉及规则有 semantic_review_candidates 标记时必须执行 AI 语义比对并将结论写入 Task Runtime Report |
| 2026-05-22 | v0.6 | 新增 governance_phase、Phase Gate Check、SC-06/SC-07、阶段化补充验证、越级请求处理和 Task Runtime Report 最小阶段门禁字段 |
| 2026-06-09 | v0.7 | 自审计修正：§3.1 TERM 前置门禁拆分硬阻断档（代码实际写 term_ref 引用）与非阻断档（仅提及，记入 unresolved_items）；§4 SC-02 触发口径收窄为 term_ref 实际引用未注册 TERM，与 CLAUDE.md §8 软提示对齐 |
| 2026-06-09 | v0.8 | 结构性减负：§1 新增 CODE_CHANGE_MINOR 任务类型；新增 §1.2 变更比例原则（六条客观判据 + 轻量流程 + 会话内快速通道）；§3.1 补充 MINOR 轻量 Scope 声明；§5 验证表新增 CODE_CHANGE_MINOR 行；新增 §6.1 紧凑版 Runtime Report |
| 2026-08-01 | v0.9 | TEST-GOVERNANCE-01：VALIDATING 接入动态变更影响选择器；Runtime Report 新增 test_impact；明确 TEST_ONLY 直选、可执行变更失败关闭及测试/验证证据分离 |
| 2026-06-09 | v0.9 | 编辑时刻强制：§3.1 新增 PreToolUse 钩子说明与 `.governance/active_scope.json`（Scope Lock 机器可读镜像）schema，对应 scripts/scope_lock_guard.mjs 与 .claude/settings.json 钩子配置 |
