# Issue Trace · MODEL-CONFIG-01 模型配置体系修复（来源分层 + 能力偏好 + 预设清单 + BYOK 管理）

承接 MODEL-CONFIG-FIX-PLAN-01（方案讨论稿）人类四项确认（2026-08-06）：
①「局部修改」仅指 inline-assist（不含 DiffEdit）② 预设模型权威源归云端（桌面端为消费方）
③ 能力偏好持久化 P0-P1 沿用 localStorage（P3 统一设置独立立项）④ 构建模式模型配置归 AG。

## 1. 用户可见症状

模型配置只有一个「添加模型」弹窗；辅助续写/局部修改无法配置模型（固定 deepseek-chat）；
无法按四类 agent 能力独立配置模型；预设模型与 BYOK 模型未分层。

## 2. 入口点

- UI：`src/components/ag/ModelConfig.tsx`（弹窗）、`src/components/ag/Composer.tsx`（模型下拉）
- 调用：`src/services/agService.ts#requestAssist`、`src/components/ed/Editor.tsx`（assist 触发点）
- 后端：`src-tauri/src/lib.rs#ag_request_assist`（已支持 provider/model，前端未传）

## 3. 相关规则

- 既有：BR-AG-STATE-003（AG-ASSIST）、BR-AG-VERIFY-001（门禁）、BR-ENT-DATA-001（BYOK 零残留）、
  BR-ENT-VERIFY-001 / BR-ENT-STATE-001（门禁通道）、BR-SYS-DATA-006（BDocx 能力清单）
- 跨模块约束：X-FORBID-001/002（密钥与门禁）、X-SEQ-001（门禁前置）
- 本次新增（A-AG-M-T-01 注册）：BR-AG-DATA-006（模型来源分层）、BR-AG-DATA-007（能力偏好解析）
- 需求：新增 REQ-AG-006（能力模型独立配置，A-AG-M-D-01 注册）

## 4. 相关链路

AG-ASSIST → AG-TURN；新增 AG-MODEL-CATALOG（模型注册表与能力偏好，active）。

## 5. 生产者块

- `BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-002`（agService.requestAssist，L802）— 本次改 boundary（in 增 provider/model）
- `BR-AG-STATE-003-EFFECT-AG-AG-ASSIST-003`（Editor.openAutocompleteAssist，L987）
- `BR-AG-STATE-003-RB-AG-AG-ASSIST-002`（Editor.applyInline，L1809）
- modelStore（无 @GOV，store 偏好）— 本次重构

## 6. 消费者块

- Composer.startTurn → ag_start_turn（Rust L6228）— 消费全局/agentEdit 偏好
- Editor 两处 requestAssist 调用点 — 消费 autocomplete/inlineAssist 偏好
- ModelConfig / Composer 模型下拉 — 消费注册表展示

## 7. 路径上的空规则块

- modelStore 非导出执行块（store 偏好对象，无 @GOV）：能力偏好解析为纯逻辑，将新增导出函数
  `resolveModelFor`，按 UTIL/业务规则归置（挂 BR-AG-DATA-007），不构成 rules: [] 遗留。

## 8. boundary 假设

- agService.requestAssist @GOV：`in=AssistReq | out=AssistResponse(...)` — in 承载实体增加
  provider/model，语义不变（透传不改判断）；`does_not=swallow_invoke_error` 保持。
- Editor assist 调用点：in 增加 modelStore 偏好读取，out 不变。

## 9. 运行时假设

- 调用点均属 async_chain；assist 采用 latest-wins（BR-AG-STATE-003），模型参数随请求闭包传递，
  不引入跨请求状态。能力偏好解析为同步纯函数，无异步依赖。

## 10. 疑似不变量破坏

- 未发现破坏。X-FORBID-001/002、门禁前置（X-SEQ-001）保持：本次仅客户端传递 provider/model
  标识符（不含密钥），密钥仍仅存网关；能力偏好不改变任何门禁裁决路径。
- BYOK 状态展示为脱敏 preview，不落盘密钥。

## 11. 验证计划

- 测试：agRuntime.test.ts（requestAssist provider/model 透传）；assistRuntime.test.ts（能力偏好解析、
  来源分层 state 逻辑）；既有 AgentUI.test.tsx / assistRuntime 回归。
- 链路日志：AG-ASSIST 门禁前置（gate=pass/deny/offline）日志序列保持。
- 治理：修改设计文档标注块后先 `npm run governance:generate`，再 `governance:audit`；
  `npm run test:related -- --files [changed_files]`；`npx tsc --noEmit`；`npm run build`。

## Scope Lock

```
task_type: CODE_CHANGE
scope_id: MODEL-CONFIG-01
governance_phase: IMPLEMENTATION
allowed_files:
  - .governance/active_scope.json / issues/MODEL-CONFIG-01.md
  - src/store/modelStore.ts                # 注册表 + source 分层 + 能力偏好
  - src/services/agService.ts              # AssistReq provider/model 透传
  - src/components/ed/Editor.tsx           # assist 调用点按偏好解析
  - src/components/ag/Composer.tsx         # 模型下拉来源标记 + 按能力偏好传参
  - src/components/ag/ModelConfig.tsx      # 分区（预设/BYOK + 能力偏好）
  - src/components/shell/LeftRail.tsx      # 设置入口打开模型配置
  - src/styles.css                          # 新增样式
  - src/services/agRuntime.test.ts         # requestAssist 契约
  - src/store/assistRuntime.test.ts        # 能力偏好/来源分层 state 测试（并入，不新增文件）
  - docs/01_modules/AG/A-AG-M-D-01_代理功能需求描述.md   # REQ-AG-006
  - docs/01_modules/AG/A-AG-M-T-01_代理技术设计.md       # TERM/CHAIN/RULE/REQ_MAP
  - docs/00_core/A-CORE-C-R-01_ADUS.md      # governance:generate 输出
  - cloud/dev-gateway/server.mjs            # GET /models、GET/DELETE /byok/keys
  - cloud/dev-gateway/byok.mjs              # 脱敏 preview / 删除
  - cloud/dev-gateway/README.md             # 接口登记
forbidden_files:
  - ADU.md / ADP.md / AGENTS.md / CLAUDE.md / APC / governance.config.json
  - scripts/test_governance.mjs / scripts/test_governance.test.mjs
confirmation: YES  # 人类 2026-08-06 指示「按顺序实现优化」
scope_reason: 模型配置体系四阶段修复（方案 MODEL-CONFIG-FIX-PLAN-01 经人类确认）
```
