# REF-REFERENCE-REPAIR-01 修复方案：引用功能全链缺口闭合

**状态**：已实施（REF-REFERENCE-REPAIR-01，2026-08-07 落地；G1~G7 全部闭合，真实 Tauri 观测待人工）
**提出**：REF 引用功能审计结论（2026-08-07），人类确认三项决策：①覆盖全部缺口（P0+P1+P2）②REF 后端孤岛逻辑抽回 REF 模块 ③外部文件拖入纳入 v1
**关联**：REF-06、A-REF-M-D-01（REQ-REF-001~006）、A-REF-M-T-01（BR-REF-*）、A-REF-I-P-01（四 command）、A-SYS-I-P-01（ReferencePayload/RefInjection）、ED-SELECTION-DRAG-01、PI-ED-10

---

## 1. 问题定义（审计结论回顾）

| 编号 | 优先级 | 缺口 | 现状证据 |
|---|---|---|---|
| G1 | P0 | 拖拽引用链路未闭合（REF-06） | 编辑器选区 dragstart 先写 MIME 后 `return false`，PM 内置 dragstart `clearData()` 并只重写 text/html·text/plain；文件树/标签/会话源 custom MIME 在真实 WebKit 可见性未实测；复制→粘贴路径可用 |
| G2 | P0 | 纯引用无法点击发送 | Composer `canSend = Boolean(draft.prompt.trim()) && !sending`，纯引用时按钮 disabled，只能 Enter 发送（违反 REQ-REF-006「纯引用可发送」） |
| G3 | P1 | REF 后端孤岛 | `resolve.rs`/`inject.rs` 三个规则函数无生产调用方（仅单测）；`ref.*` 四 command 未在 build_registry/dispatch 注册；解析/校验/组装内联进 AG lib.rs |
| G4 | P1 | 选区锚偏移基准不一致 | 前端 start/end 按 UTF-16 code unit（BR-ED-DATA-004），后端校验用 `Vec<char>`（Unicode scalar），含 emoji 选区校验错位→退化 stale |
| G5 | P2 | 外部文件拖入缺失 | REF v1 目标模型无「外部文件」；Composer drop 不处理 `dataTransfer.files`/`file://` |
| G6 | P2 | 锚失效信号不一致 | 设计契约是 Err（REF_ANCHOR_INVALID 不注入），AG 内联实现是「标注注入原始快照」，两处语义漂移 |
| G7 | P2 | @ 候选含资源文件 + doc_type 硬编码 | `ws.list_docs` 返回全量文件，@ 候选未过滤资源文件（选中后发送读取失败）；resolve.rs `doc_type` 固定 "md" |

---

## 2. 修复方案设计

### F1 拖拽引用链路闭合（G1 · P0 · 解 REF-06）

**前提假设**：按 REF-06「custom `application/x-*` MIME 在真实 WebKit drop 不可靠」的最坏假设设计；观测结论若推翻假设，custom MIME 路径保留为加速通道，方案不受影响。

**F1.1 真实事件观测先行（硬前置，不观测不实施）**
- 临时在 Composer `onDrop` 与四类源 `dragstart` 打点：`types` 列表、各 MIME `getData`、`defaultPrevented`、事件 target。
- 产出观测记录（写入 issue_trace），确认：
  - (a) 文件树/标签/会话拖拽：custom MIME 在真实 Tauri/WebKit drop 是否可见；
  - (b) 编辑器选区：PM clearData 是否复现（已按源码确认，需实测确认）；
  - (c) `text/plain` 在 drop 阶段是否恒可见。
- 观测结论决定 F1.2/F1.3 的最终接线，但两种结论下方案主体（drag session store）均成立。

**F1.2 统一应用内拖拽通道（drag session store）**
- 新增 `src/services/referenceDrag.ts`：
  - `DRAG_SESSION_MIME = "application/x-binder-drag-session"`、`TEXT_MARKER = "binder-ref:"`。
  - 模块级 `sessionStore: Map<dragSessionId, ReferencePayload>`（可注入/清理，供测试）。
  - `startReferenceDrag(dataTransfer, payload)`：
    - `dragSessionId = uid()`；`sessionStore.set(id, payload)`；
    - `setData(DRAG_SESSION_MIME, id)`（加速路径）；
    - `setData(REFERENCE_MIME, JSON.stringify(payload))`（custom MIME 可见时的快路径，保留）；
    - `setData("text/plain", TEXT_MARKER + id)`（**保底通道**：text/plain 在 WebKit 恒可见，PM clearData 后由 F1.3 回写）。
  - `resolveReferenceDrag(dataTransfer): ReferencePayload | null`（解析优先级）：
    1. `readReferencePayloadData`（REFERENCE_MIME → coerce）；
    2. `getData(DRAG_SESSION_MIME)` → `sessionStore.get`；
    3. `text/plain` 以 `TEXT_MARKER` 开头 → 取 id → `sessionStore.get`；
    4. 均无 → `null`（交回现有 uri/plain/file 分类兜底）。
  - `endReferenceDrag(id)`：drop/dragend 后清理 `sessionStore`。

**F1.3 编辑器选区：document 冒泡后置回写（解决 PM clearData）**
- PM 内置 dragstart handler 注册在编辑器 DOM（冒泡阶段）；在 **document 冒泡阶段**注册 dragstart 监听，天然在 PM 处理之后执行。
- 判定：`event.target` 在活动编辑器内 + selection 为非空文本选区 + 非图片 → `getSelectionRef()` 生成 selection payload → `startReferenceDrag(dataTransfer, payload)`（**回写**覆盖 PM 刚清空的 dataTransfer）。
- Editor.tsx 现有 `handleDOMEvents.dragstart` 收敛为只做 `lockDragSelection`（拖动锁定，ED-SELECTION-DRAG-01），MIME 写入统一由 document 冒泡回写承担（单一写入通道，避免两处不一致）。
- 监听生命周期：随编辑器挂载/卸载注册/移除（或模块级单例 + 活动编辑器判定，参考 `edLocate.activeEditor` 模式）。
- `copy` 路径保持不变（copy 已 `preventDefault+return true`，不受 PM clearData 影响）。

**F1.4 各拖拽源改造（统一走 F1.2）**
- `FileTreeNode.tsx` / `TabStrip.tsx`：`onDragStart` 由 `writeDocRefData` 改为 `startReferenceDrag(dataTransfer, makeDocReference(path))`。
- `AgentPanel.tsx` 会话标签：改为 `startReferenceDrag(dataTransfer, sessionPayload)`。
- `Composer.tsx`：`payloadFromTransfer` 首位改调 `resolveReferenceDrag`；`onDrop` 命中后调用 `endReferenceDrag`；`onDragOver` 的 `REFERENCE_DRAG_TYPES` 增 `DRAG_SESSION_MIME`。

### F2 纯引用发送（G2 · P0）
- [Composer.tsx](file:///Users/imatstarbucks/binder/src/components/ag/Composer.tsx) `canSend` 改为：
  `Boolean(draft.prompt.trim() || draft.tokens.length > 0) && !sending`。
- 按钮 `title` 文案同步（「输入内容或引用后可发送」）。
- 发送侧 `send()` guard 不变（已允许纯引用）。

### F3 REF 后端孤岛收敛（G3 · P1 · 逻辑抽回 REF 模块）

**F3.1 新增 `src-tauri/src/reference/context.rs`（规则逻辑归 REF，不反向依赖 AG）**
- `validate_selection(payload, block_text: Option<&str>, doc_content: Option<&str>) -> SelectionCheck`：
  `Located { current: String }` / `Stale { reason }`（纯函数，UTF-16 语义，见 F4）。
- `format_payload_context(payload: &ReferencePayload, cap: usize) -> String`：doc/selection/session/memory/knowledge/template/url/long_text/**file** 各分支的注入上下文格式化（自 lib.rs `format_reference_payload_context` 迁移；doc/selection 的内容由调用方读取后以参数传入，REF 不直接持有 AgentReadContext，保持模块边界）。
- `format_block_ref_context(br, capped_text)`、`block_ref_range_label`（自 lib.rs 迁移，供 AG 兼容路径）。
- @GOV 归属：BR-REF-DATA-003 / BR-REF-VERIFY-001。

**F3.2 lib.rs 收敛为调用方**
- 删除内联 `format_reference_payload_context` / `format_block_ref_context` / `block_ref_range_label`。
- `ag_start_turn` 中：从 `AgentReadContext.snapshot` 取块/内容 → 调 `reference::context::validate_selection` + `format_payload_context`；**组装与预算仍归 AG**（BR-AG-DATA-001 不变）。
- 既有 `ag_reference_context_tests` 断言迁移到 `context.rs` 对应测试，lib.rs 保留调用侧测试。

**F3.3 注册 `ref.*` Tauri command（契约落地）**
- `build_registry` 增注册：`ref.list_candidates` / `ref.resolve_anchor` / `ref.verify_anchor` / `ref.build_injection`。
- `dispatch` 增 `route_ref_*` 分支（envelope 模式，与 ws/de 一致）：
  - `ref.list_candidates`（workspace_id, query）→ 复用 ws.list_docs 数据 → 过滤可读文档（见 F7）→ 候选列表。
  - `ref.resolve_anchor`（rel_path, block_ids?）→ `resolve_mention`（workspace 限界 + target_exists 判定）。
  - `ref.verify_anchor`（anchor）→ 路径级经 WS 判存在；块级经 ed locate（`LocateResult`）→ `verify_anchor`。
  - `ref.build_injection`（anchor）→ verify 通过后经 WS 读**读取时点**内容 → `build_injection`（RefInjection）。
- **诚实声明**：AG 回合组装在进程内调用 `reference::context`（不绕 Tauri IPC）；`ref.*` command 当前无前端消费者，其价值是接口协议（A-REF-I-P-01）契约闭合 + 规则函数拥有真实运行调用链，供调试与未来消费方；不注册才会持续孤岛。

### F4 选区锚偏移基准统一（G4 · P1）
- `validate_selection` 校验改用 UTF-16 语义：
  `let utf16: Vec<u16> = block_text.encode_utf16().collect(); let current = String::from_utf16_lossy(&utf16[start..end]);`
- 与前端 `textBetween` / `String.length` 语义对齐（BR-ED-DATA-004）。
- 补测试：含 emoji/非 BMP 字符的选区，前后端偏移一致。

### F5 外部文件拖入（G5 · P2 · 纳入 v1）
- `referencePayload.ts`：`ReferenceKind` 增 `"file"`；payload 类型
  `{ kind:"file"; name: string; path?: string; size?: number; label?: string }`；
  `coerceReferencePayload` / `referenceLabel`（`文件 <name>`）/ `referenceKey`（`file:name:size`）同步；
  `referenceFromPlainTextPaste` 增 `file://` 识别（`isLikelyUrl` 之外单列，不把 file:// 当 http 链接）。
- Rust `types/mod.rs` `ReferencePayload` 增 `File` 变体（serde camelCase 对齐）。
- `reference/context.rs` 增 File 分支：`引用文件「name」（来源 path?，大小 size）：仅文件引用，未读取内容`。
- Composer `payloadFromTransfer`：`dataTransfer.files` 非空 → file payload（name/size；path 只取文件名或 `webkitRelativePath`，**不携带外部绝对路径**，守 X-FORBID-004 精神——file 引用只传元数据不读内容）。
- 文档回写：REF L1/L2 目标模型、REQ-REF-005 范围、SYS ReferencePayload 基线（X-SHARED-001）增 file 字段。

### F6 锚失效信号一致性（G6 · P2）
- 语义统一为「**不中断回合的标注注入**」：`validate_selection` 返回 Stale 时，`format_payload_context` 输出统一前缀
  `[引用锚已失效，以下为发送时原始快照，不得作为当前文档事实]`。
- 修订 A-REF-M-T-01 BR-REF-VERIFY-001 语义描述：失效信号允许两种形态（注入文本明确标注 / 接口 Err），REF 回合内统一采用标注注入；`verify_anchor` 的 Err 契约保留供 command 路径使用（F3.3）。
- 前端消息渲染对 stale 引用加视觉标记（可选，延后到前端 G4 若需要）。

### F7 @ 候选过滤 + doc_type 精度（G7 · P2）
- Composer `atMatches`：`fileDocs` 过滤为可读文档（md/markdown/docx/doc/html/txt/rtf），资源文件（图片/Excel/PPT/音视频）不进入 @ 候选。
- `ref.list_candidates` 同规则过滤。
- `resolve.rs` `resolve_mention`：`doc_type` 按扩展名推断（md→md / docx→docx / html→html / 其余→txt），不再硬编码 "md"。

---

## 3. 文件范围清单

| 文件 | 变更 | 关联修复 |
|---|---|---|
| `src/services/referenceDrag.ts` | 新增：session store / start / resolve / end | F1 |
| `src/services/referencePayload.ts` | file 类型、file:// 识别 | F5 |
| `src/components/ag/Composer.tsx` | canSend、drop 走 resolveReferenceDrag、@ 过滤、files 识别 | F1/F2/F5/F7 |
| `src/components/ed/Editor.tsx` | dragstart 收敛为 lock + document 冒泡回写接线 | F1 |
| `src/components/ws/FileTreeNode.tsx` / `src/components/shell/TabStrip.tsx` | dragstart 改 startReferenceDrag | F1 |
| `src/components/ag/AgentPanel.tsx` | 会话标签 dragstart 改 startReferenceDrag | F1 |
| `src/components/ag/MessageList.tsx` | stale 引用视觉标记（可选） | F6 |
| `src-tauri/src/reference/context.rs` | 新增：validate_selection / format_payload_context / block_ref 辅助 | F3/F4/F5/F6 |
| `src-tauri/src/reference/resolve.rs` | doc_type 推断 | F7 |
| `src-tauri/src/reference/mod.rs` | 登记 context 模块 | F3 |
| `src-tauri/src/lib.rs` | 删除内联格式化；AG 调 context；注册 ref.* command + route_ref_* | F3 |
| `src-tauri/src/types/mod.rs` | ReferencePayload 增 File | F5 |
| 测试 | referenceDrag.test.ts（新）、referencePayload.test.ts、Composer 组件测试、edRuntime.test.ts、context.rs 测试、command 测试 | 全部 |

---

## 4. 测试计划

**前端（vitest）**
1. `referenceDrag.test.ts`（新）：session 生命周期（start→resolve→end 清理）；REFERENCE_MIME / DRAG_SESSION_MIME / text/plain marker 三通道解析；store 隔离。
2. `referencePayload.test.ts` 增：file 类型 coerce/label/key/roundtrip；`file://` 识别且普通文本不误判。
3. Composer 组件测试（新/扩展现有组件测试）：纯引用 canSend=true 且点击发送经 `startTurn` 收到 `references`；drop（mock DataTransfer 各通道）插入 token；@ 候选过滤资源文件。
4. `edRuntime.test.ts` 增：document 冒泡回写（jsdom 模拟 PM clearData 后回写 selection MIME）。
5. `agRuntime.test.ts` 既有 references 契约回归。

**Rust（cargo test）**
6. `reference/context.rs`：各 payload 格式化断言（含 file）；selection UTF-16 校验（**含 emoji 用例**）；Stale 前缀标注。
7. `ref.*` command：list_candidates 过滤 / resolve 越界与不可达 / verify 失效保守 / build_injection 读取时点（经 dispatch 信封测试，复用既有 route 测试模式）。
8. 既有 lib.rs 引用上下文测试迁移后全绿。

**真实环境（人工，F1.1 观测 + 冒烟）**
9. Tauri 实测拖拽：编辑器选区 / 文件树 / 编辑器标签 / 会话标签 → Composer 均生成 ReferenceToken；外部文件拖入生成 file 引用。

---

## 5. 验证命令

```bash
npx vitest run src/services/referenceDrag.test.ts src/services/referencePayload.test.ts
npx vitest run src/components/ag/  src/services/edRuntime.test.ts   # 组件 + 选区回写
npm run test:related -- --files <changed_files>
cargo test --manifest-path src-tauri/Cargo.toml
npm run governance:generate && npm run governance:audit
# 人工：Tauri dev 冒烟（拖拽四源 + 外部文件 + 纯引用发送）
```

---

## 6. 边界与风险

| 项 | 处理 |
|---|---|
| drag session store 泄漏 | drop/dragend/dragcancel 三处清理；drop 超时（如 30s）兜底清；store 上限防膨胀 |
| text/plain 标记污染外部粘贴 | marker 前缀 `binder-ref:` 唯一性强，误撞概率极低；`coerceReferencePayload` 仍先验 shape |
| 拖拽预览显示 marker | 接受（短暂、低影响）；若 UX 不可接受可改 `setData("text/plain", label)` + 保留 DRAG_SESSION_MIME 通道，但丧失保底，需观测结论支撑 |
| document 冒泡回写误伤 | 仅当 target 在活动编辑器内 + 非图片文本选区时回写；与 ED-SELECTION-DRAG-01 的 move 语义无冲突（回写只增 dataTransfer 内容） |
| ref.* command 无前端消费者 | 诚实登记：契约闭合 + AG 内部进程内调用底层函数；不伪造调用方 |
| file 引用 X-FORBID-004 | file 只传元数据不读内容；不携带外部绝对路径；workspace 内文档仍走 doc/selection |
| 观测推翻最坏假设 | 方案主体不变；仅 F1.4 可保留 custom MIME 单通道，删 marker 回写 |
| 迁移 lib.rs 测试风险 | 断言逐一迁移到 context.rs，保留调用侧集成断言，防止"迁移即丢覆盖" |

---

## 7. 治理

- issue_trace：`.governance/issues/REF-REFERENCE-REPAIR-01.md`（含 F1.1 观测记录）。
- 文档回写（人类确认后）：
  - A-REF-M-D-01：REF-06 关闭；REQ-REF-005 增 file 来源；范围更新。
  - A-REF-M-T-01：目标模型增 file；BR-REF-VERIFY-001 失效信号形态澄清（标注注入/Err 双形态）。
  - A-REF-I-P-01：四 command 落地状态更新（§2 表）+ file 类型行（§2.1）。
  - A-SYS-I-P-01：ReferencePayload 基线增 file 字段（X-SHARED-001 回写）。
  - ADUS 重新生成；@GOV 新增块标注（BR-REF-DATA-003/004/VERIFY-001）。
- ADP：进入 INTAKE，分批 scope（建议批 1=F1+F2，批 2=F3+F4，批 3=F5+F6+F7），每批独立 Runtime Report。

---

## 8. 实施顺序（建议分批）

1. **批 1（P0）**：F1.1 观测 → F1.2 drag session store → F1.3 编辑器回写 → F1.4 各源接线 + F2 纯引用发送。
2. **批 2（P1）**：F3 context.rs 迁移 + lib.rs 收敛 + ref.* command + F4 UTF-16 校验。
3. **批 3（P2）**：F5 file 类型 + F6 失效信号一致 + F7 候选/doc_type + 文档回写与 ADUS 刷新。

---

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-07 | v1.0 | 初版：基于引用功能审计结论，人类确认三项决策（全量缺口 / 逻辑抽回 REF / 外部文件纳入 v1）后生成；覆盖 G1~G7 七项缺口，含观测先行、drag session store、PM 后置回写、REF command 契约闭合、UTF-16 校验、file 类型、失效信号双形态、候选过滤与 doc_type 推断 |
