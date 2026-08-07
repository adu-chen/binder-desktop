# Issue Trace：REF-REFERENCE-REPAIR-01

## 目标

闭合引用功能全链缺口（审计结论，人类 2026-08-07 确认三项决策：①覆盖全部缺口 P0+P1+P2 ②REF 后端孤岛逻辑抽回 REF 模块 ③外部文件拖入纳入 v1）。修复方案见 `docs/02_plans/REF-REFERENCE-REPAIR-01_修复方案.md`。

## Issue Trace（ADU §6.2 十一项）

**1. 用户可见症状**

- 编辑器选区拖拽到 AI 输入框不生成引用标签（复制→粘贴可用，拖拽不可用）。
- 文件树 / 编辑器标签 / 会话标签拖拽到输入框在真实界面不形成引用。
- 纯引用（只拖入引用、无正文文字）时发送按钮置灰，无法点击发送（只能 Enter）。
- 外部文件拖入输入框无任何引用效果。
- 选区含 emoji（非 BMP 字符）时，发送的选区引用被降级为"原始选区快照"标注。

**2. 入口点**

- `src/components/ag/Composer.tsx`：Composer 组件（`onDrop`/`onDragOver`/`onPaste`/`send`/`canSend`）。
- `src/components/ed/Editor.tsx`：`editorProps.handleDOMEvents.dragstart`（写 SelectionRef MIME 后 `return false`）。
- `src/components/ws/FileTreeNode.tsx` / `src/components/shell/TabStrip.tsx` / `src/components/ag/AgentPanel.tsx`：各拖拽源 `onDragStart`。
- `src-tauri/src/lib.rs`：`ag_start_turn`（`references` 消费与内联格式化）。
- `src-tauri/src/reference/resolve.rs` / `inject.rs`：规则函数（无生产调用方）。

**3. 相关规则**

- BR-REF-DATA-001（@mention 触发与解析，限界/无悬空）
- BR-REF-DATA-002（注入载荷组装，读取时点/可溯源）
- BR-REF-DATA-003（多来源归一为 ReferencePayload）
- BR-REF-DATA-004（输入流 ReferenceToken 一致性）
- BR-REF-VERIFY-001（锚失效校验，不静默注入）
- BR-SYS-DATA-002（跨模块共享结构单一来源，ReferencePayload/RefInjection）
- BR-AG-DATA-001（AG 组装消费，显式注入优先保留）
- BR-ED-DATA-004（选区引用锚，UTF-16 偏移语义）
- X-FORBID-004（workspace 限界，REF 无直接文件 I/O）
- X-FORBID-005（触发/候选/标记经 ED 扩展点）
- X-SHARED-001（跨模块共享结构变更须回写 SYS 契约层）

**4. 相关链路**

- REF-INJECT → AG-TURN（引用解析→注入→AG 组装消费）
- ED-EDIT（选区锚产出，BR-ED-DATA-004；与 REF-INJECT 交互经拖拽/复制通道）

**5. 生产者块**

- BR-REF-DATA-003-REF-REF-INJECT-001 位于 src/services/referencePayload.ts（归一化）
- BR-REF-DATA-003-REF-REF-INJECT-002 位于 src/services/referencePayload.ts（coerce）
- BR-REF-DATA-003-REF-REF-INJECT-004 位于 src/services/referencePayload.ts（MIME 类型探测）
- BR-REF-DATA-004-REF-REF-INJECT-003 位于 src/services/referencePayload.ts（prompt 序列化）
- BR-REF-DATA-001-GUARD-REF-REF-INJECT-001 位于 src-tauri/src/reference/resolve.rs（锚解析）
- BR-REF-DATA-002-DATA-REF-REF-INJECT-001 位于 src-tauri/src/reference/inject.rs（载荷组装）
- BR-AG-DATA-001-AG-AG-TURN-009 位于 src-tauri/src/lib.rs（payload 上下文格式化，**本次迁移**）

**6. 消费者块**

- BR-REF-DATA-004-AG-REF-INJECT-001 位于 src/components/ag/Composer.tsx（ReferenceTokenNode）
- BR-AG-DATA-001-AG-AG-TURN-009 位于 src-tauri/src/lib.rs（ag_start_turn 组装 mention_block）
- AG-TURN 链路组装（回合上下文、预算裁剪）

**7. 路径上的空规则块**

- 未发现：REF-INJECT 路径上现有 @GOV 块均有 rules 引用（BR-REF-* / BR-SYS-* / BR-AG-*）。

**8. boundary 假设**

- 现有块均为标准 `in=`/`out=` boundary 声明，无 `lifecycle_assumption=` / `document_snapshot=` 特殊假设。
- 本次新增块（referenceDrag.ts、reference/context.rs、ref.* route）将声明标准 boundary，不引入生命周期/快照假设。
- 假设「custom `application/x-*` MIME 在真实 WebKit drop 可见」在 F1.1 观测前**不成立即修正**（方案按最坏假设设计 drag session store 保底）。

**9. 运行时假设**

- `caller_context=`：现有块未声明特殊 caller_context；新块沿用调用方（Composer / ag_start_turn）上下文。
- `runtime_guard=`：Composer 依赖 `dataTransferHasAnyType` 判定拖拽类型（将被 drag session 通道扩展）；resolve.rs 假设 `doc_type="md"`（将按扩展名推断修正）。
- 假设「PM 内置 dragstart 执行 `clearData()` 后回写」将在 F1.1 观测中确认，未确认前按方案回写路径实现。

**10. 疑似不变量破坏**

- F3 迁移 `format_reference_payload_context` 后，BR-AG-DATA-001 的 @GOV 覆盖链必须保持完整（迁移后 context.rs 承接 BR-REF-* 引用，AG 组装块保留 BR-AG-DATA-001 引用）。
- F5 增 file 类型后，ReferencePayload 结构变化须回写 A-SYS-I-P-01（X-SHARED-001），否则违反 BR-SYS-DATA-002。
- F6 修订 BR-REF-VERIFY-001 语义描述须刷新 ADUS 并复核引用该规则的全部 @GOV 块。
- F1 拖拽通道变更不得破坏 ED-EDIT 的选区拖动移动语义（ED-SELECTION-DRAG-01 的 move 型 drop 不受影响）。

**11. 验证计划**

- 前端：referenceDrag.test.ts（新）、referencePayload.test.ts（file 类型）、Composer 组件测试（纯引用发送/drop/@ 过滤）、edRuntime.test.ts（document 冒泡回写）、agRuntime.test.ts（references 契约回归）。
- Rust：reference/context.rs 测试（各 payload 格式化 + UTF-16 emoji 用例 + stale 标注）、ref.* command 测试（dispatch 信封）、既有 lib.rs 引用上下文测试迁移后全绿。
- 链路日志：REF-INJECT 拖拽 session 生命周期（start→resolve→end）与 AG-TURN 显式引用组装日志序列。
- 人工：Tauri 真实拖拽观测（F1.1）+ 四源拖拽/外部文件/纯引用发送冒烟。
- 工具链：`npm run governance:generate`（设计文档标注块修改后）+ `governance:audit`；`test:related` 按 changed_files 选择。

## 范围结论（ADP Scope Lock 输入）

**允许修改的文件**

- 前端：`src/services/referenceDrag.ts`（新增）、`src/services/referencePayload.ts`、`src/components/ag/Composer.tsx`、`src/components/ed/Editor.tsx`、`src/components/ws/FileTreeNode.tsx`、`src/components/shell/TabStrip.tsx`、`src/components/ag/AgentPanel.tsx`、`src/components/ag/MessageList.tsx`
- 前端测试：`src/services/referenceDrag.test.ts`（新增）、`src/services/referencePayload.test.ts`、`src/components/ag/Composer.test.tsx`（新增）、`src/services/edRuntime.test.ts`、`src/services/agRuntime.test.ts`
- 后端：`src-tauri/src/reference/context.rs`（新增）、`src-tauri/src/reference/resolve.rs`、`src-tauri/src/reference/mod.rs`、`src-tauri/src/lib.rs`、`src-tauri/src/types/mod.rs`
- 设计文档：`docs/01_modules/REF/A-REF-M-D-01_引用功能需求描述.md`、`docs/01_modules/REF/A-REF-M-T-01_引用技术设计.md`、`docs/01_modules/REF/A-REF-I-P-01_引用接口协议.md`、`docs/01_modules/SYS/A-SYS-I-P-01_数据结构定义.md`
- 治理文件：`.governance/issues/REF-REFERENCE-REPAIR-01.md`、`.governance/active_scope.json`

**禁止修改的文件**

- ADU.md / ADP.md / AGENTS.md / CLAUDE.md / docs/00_core/A-CORE-C-P-01_APC.md / governance.config.json / scripts/（audit、generate、test_governance 等）——需修改须人类明确要求。
- ADUS.md 自动生成区（人工禁改；由 `governance:generate` 刷新）。

## 决策

- 人类 2026-08-07：①覆盖全部缺口（P0+P1+P2）②REF 后端孤岛逻辑抽回 REF 模块 ③外部文件拖入纳入 v1。

## 实现（REF-REFERENCE-REPAIR-01 落地）

- **F1 拖拽链路**：新增 `src/services/referenceDrag.ts`（drag session store，REFERENCE_MIME → DRAG_SESSION_MIME → text/plain marker 三级解析，命中即消费 + TTL 兜底）；编辑器选区在 document 冒泡阶段后置回写（PM clearData 后）；文件树/编辑器标签/会话标签/编辑器选区四源统一走 `startReferenceDrag`。
- **F2 纯引用发送**：Composer `canSend` 放开（审计原判 G2 实为误报——prompt 序列化含 `[引用:label]` 标记本已非空；修复作为防御性加固保留）。
- **F3 孤岛收敛**：新增 `src-tauri/src/reference/context.rs`（`validate_selection` UTF-16 / `format_payload_context` / `format_block_ref_context`，纯函数不依赖 AG 类型）；lib.rs 收敛为读取+分派；注册 `ref.list_candidates/resolve_anchor/verify_anchor/build_injection` 四 command 并经 dispatch 路由（块级锚在 command 路径保守 Undetectable）。
- **F4 偏移基准**：`validate_selection` 按 UTF-16 code unit 切片（BR-ED-DATA-004），含 emoji 用例。
- **F5 外部文件**：`ReferencePayload` 增 `file` 类型（前端/Rust/context 三端 + A-SYS-I-P-01 基线回写 v1.12）。
- **F6 失效信号**：AG 回合内统一「引用锚已失效」前缀标注注入；command 路径保留 Err 契约。
- **F7 候选/doc_type**：@ 候选与 `ref.list_candidates` 过滤可读文档；`infer_doc_type` 按扩展名。

## 验证

- 前端全量 542 通过（含 referencePayload 12、AgentUI 23、WorkspaceUI 30、ShellComponents 20）。
- Rust 全量通过（lib 284 + core_chain/sm_broadcast 8），含 context.rs 10、ref_route_tests 4、resolve 1、types file roundtrip。
- `npx tsc --noEmit` 通过；`governance:generate`（@GOV 块 241、测试资产 93、用例 835）+ `governance:audit` 全部 passed；`test:governance` 通过。
- `test:plan` 升级全量回归（AG/REF 高影响）；`test:related` 全绿（含治理自测断言同步 79→82）。

## 遗留

- **F1.1 真实 Tauri/WebKit 人工观测（2026-08-07 已完成）**：人工实测「拖拽生成引用标签」「单引用发送」均正常（REF-01 闭合确认）。四源拖拽（编辑器选区/文件树/编辑器标签/会话标签）冒烟通过；外部文档内容读取经审计确认为 REF-07 设计边界（X-FORBID-004/X-SEQ-002），非本课题缺陷。
- ref.* command 注册后当前无前端消费者（契约闭合 + AG 进程内调用底层函数），诚实登记。
- G2 审计原判「纯引用无法点击发送」为误报（prompt 序列化含引用标记），修复保留为防御性加固。
- 审计延伸债务 REF-07/08/09 已登记 A-REF-M-T-01 §9，待立项。
