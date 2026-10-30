# Issue Trace — DOCX-NATIVE-ENGINE-01

## 治理信息

```
task_type:         CODE_CHANGE
governance_phase:  IMPLEMENTATION
status:            SCOPED（等待人类 confirmation）
scope_mode:        explicit
关联决策:          D1–D12（见文末「人类决策」）
```

> 本 Trace 只覆盖 **阶段 0：引擎策略接口 + 影子骨架**。
> 阶段 1（native 打开）、阶段 2（native 保存）、阶段 3（默认切换与清理）
> 必须在阶段 0 READY 后各自重新评估 Scope，不得沿用本 Scope 自动推进。

---

## 1. 用户可见症状

无单一崩溃或报错投诉。这是**架构级适配损耗**，表现为五个可观测现象：

1. 打开与保存 docx 各需一次 Pandoc 子进程；大文档慢，且存在 15 MB HTML 上限与 60 s 转换超时。
2. 页眉页脚、文本框、SmartArt、VBA、合并单元格属于 `Unsupported`，编辑保存后丢失；字体族/字号/自定义样式只属 `BestEffort`。
3. 颜色、单元格底纹、图片版式无法由 Pandoc 稳定承载，需要「短生命周期标记」在 HTML 与 OOXML 之间走私传递。
4. `blockId`、`tableId`、`imageId` 三套身份锚均不保证跨往返保留；重开文档即重建锚。
5. 原生 docx 无法直接编辑，必须先复制为 `*.draft.docx` 才能进入编辑链。

**结论**：根因不是实现 bug，而是「把一次性格式转换器当作无损往返 + 稳定身份的编辑器内核」的能力契约错配。

---

## 2. 入口点

| # | 入口 | 位置 | 说明 |
|---|---|---|---|
| E1 | `route_open_for_edit` | `src-tauri/src/doc/routes.rs:72` | 打开 BDocx 进入编辑 |
| E2 | `route_save_from_edit` | `src-tauri/src/doc/routes.rs:371` | 编辑保存，L410 调 `html_to_docx` |
| E3 | `route_create_draft` | `src-tauri/src/doc/routes.rs:20` | 原生 docx 创建编辑副本 |
| E4 | `route_create_doc` | `src-tauri/src/workspace/routes.rs:833` | 新建 docx，L846 调 `html_to_docx` |
| E5 | `route_accept_diff` | `src-tauri/src/de/routes.rs:292` | 后台 diff 落盘，L369 调 `html_to_docx` |
| E6 | `read_editable_snapshot` | `src-tauri/src/ag/blocks.rs:543` | 未打开方案读取，L575 调 `docx_to_html` |
| E7 | `export` | `src-tauri/src/prv/mod.rs:134` | md↔docx 导出，L149/L155 调 Pandoc |
| E8 | `docx_to_edit_html` / `html_to_docx` | `src-tauri/src/doc/edit_chain.rs:104` / `:183` | 转换实现本体 |

> 阶段 0 收口 E1–E6；E7（导出）与 E8（legacy 实现）保持原样。

---

## 3. 相关规则

| 规则 | 约束内容 | 本次是否受影响 |
|---|---|---|
| `BR-DOC-DATA-001` | 双链分离、本地不上行、依赖缺失不静默降级 | 不变；新增 native 链后仍需保持不静默降级 |
| `BR-DOC-DATA-002` | 样式子集与往返保真分级 | **语义需重新定义**：从「支持哪些样式」转为「哪些块能正确生成」 |
| `BR-DOC-DATA-003` | 转换层 blockId 兜底 | 阶段 0 不变（D9=A） |
| `BR-DOC-STATE-001` | SM-DOC-01 转换生命周期 | 不变；失败仍不建立编辑会话 |
| `BR-ED-DATA-001` | blockId 锚定 | 阶段 0 不变 |
| `BR-ED-DATA-002` | 转换归 DOC、编辑归 ED | **不变且必须守住** |
| `BR-WS-PERSIST-001` | 原子落盘 | 不变 |
| `BR-WS-STATE-001` | 仅 accepted 落盘 | 不变 |
| `BR-DE-DATA-001/002/003` | ProposalBatch、DiffPayload、生成校验 | 阶段 0 不变 |
| `BR-PRV-DATA-001/002` | 预览只读、管线归属 | 不变 |
| `X-SEQ-002` | 前端不得跨层直连 FS/DB/网络 | native 解析仍只在 Rust 后端 |
| `X-SEQ-004` | 二进制解析唯一通道归后端 | 必须保持 |
| `X-FORBID-004` | 文件读写经 WS | 必须保持 |
| `X-FORBID-005` | 逻辑态只能经 ED 受控接口修改 | 必须保持 |
| `X-SHARED-001` | 共享结构单一来源 | 新增引擎枚举不得复制既有共享结构 |
| `X-FORBID-006` | 轻/重编辑分界（③ 经 DE 裁决） | 必须保持 |

未发现 `RULE_MISSING`；但 `BR-DOC-DATA-002` 的语义需在阶段 1 前单独重写（非本 Scope）。

---

## 4. 相关链路

```
DOC-CONVERT → WS-SAVE
DOC-CONVERT → DE-DIFF
DOC-CONVERT → AG-TURN（未打开文档只读投影）
DOC-CONVERT → ED-EDIT（HTML 装载）
PRV-VIEW（预览/导出，阶段 0 不涉及）
```

阶段 0 只改变 `DOC-CONVERT` 内部的实现选择，不新增链路、不改变链路顺序。

---

## 5. 生产者块

| @GOV codes | 位置 |
|---|---|
| `BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-016` | `src-tauri/src/doc/edit_chain.rs`（docx→编辑 HTML） |
| `BR-DOC-DATA-002-IO-DOC-DOC-CONVERT-004` | `src-tauri/src/doc/edit_chain.rs`（HTML→docx） |
| `BR-DOC-DATA-003-DATA-DOC-DOC-CONVERT-011` | `src-tauri/src/doc/block_backfill.rs` |
| `BR-DOC-DATA-002-DATA-DOC-DOC-CONVERT-001` | `src-tauri/src/doc/style_subset.rs` |
| `BR-DOC-DATA-001-DATA-DOC-DOC-CONVERT-012` | `src-tauri/src/doc/pandoc_runtime.rs` |
| `BR-DOC-DATA-001-IO-DOC-DOC-CONVERT-001` | `src-tauri/src/doc/pandoc_runtime.rs` |
| `BR-DOC-STATE-001-RB-DOC-DOC-CONVERT-001` | `src-tauri/src/doc/convert.rs` |
| `BR-DOC-DATA-001-GUARD-DOC-DOC-CONVERT-001` | `src-tauri/src/doc/convert.rs` |

阶段 0 新增文件 `doc/engine/` 暂不新增 `@GOV` 块（纯转发层，无新业务规则）；若后续需要标注，按 ADU §4 另开治理变更。

---

## 6. 消费者块

| 位置 | 消费内容 |
|---|---|
| `src-tauri/src/doc/routes.rs` | 打开/保存返回的 HTML 与 docx 字节 |
| `src-tauri/src/workspace/routes.rs` | 新建文档初始 docx 字节 |
| `src-tauri/src/de/routes.rs` | 后台 diff 落盘 docx 字节 |
| `src-tauri/src/ag/blocks.rs` | 未打开文档的只读 HTML 投影 |
| `src-tauri/src/prv/mod.rs` | 导出链（阶段 0 不动） |
| `src/services/docService.ts` / `edService.ts` / `deService.ts` | IPC 调用方，阶段 0 不改前端 |

具体 `@GOV` codes 以 `npm run governance:generate` 后的 ADUS 为准；本次范围依据以文件与函数为准。

---

## 7. 路径上的空规则块

未发现未分类的 `rules: []` 块。

已知技术债（不在本 Scope 修复）：

- `src-tauri/src/ag/blocks.rs` 模块头自述「dev 级轻量字符串扫描，嵌套同名块、畸形 HTML 不保证」。
- `src-tauri/src/doc/block_backfill.rs` 模块头自述「手工字符串扫描，非 HTML parser」。

两项均属同一根因族，但阶段 0 只做调用收口，不做解析器替换。

---

## 8. boundary 假设

本仓库现有 `@GOV` 块未声明 `lifecycle_assumption=` 与 `document_snapshot=` 字段（`doc/` 全目录检索无命中）。因此本项记录语义假设：

| 假设 | 阶段 0 是否成立 |
|---|---|
| 转换失败不建立编辑会话 | 成立（legacy 行为不变） |
| 保存失败原 docx 保持不变、无半成品 | 成立（落盘仍走 WS 原子写） |
| 未修改块不承诺字节保真 | 成立；字节保真是阶段 2 的目标，不是阶段 0 的承诺 |
| 原生 docx 只读、编辑须创建副本 | 成立（D10=A，本阶段不改产品语义） |
| 文档快照以 LogicalState 为准 | 成立（引擎只替换 `文件→HTML` 适配层） |

---

## 9. 运行时假设

| 假设 | 现状 | 本次影响 |
|---|---|---|
| 包内 Pandoc/LO 由 manifest 锁定，缺失不静默降级 | `doc/tool_runtime.rs` 用 `OnceLock` 统一校验 | **阶段 1 需改为 per-engine 校验**；阶段 0 不触发 |
| 转换在本地子进程执行，内容不上行 | 成立 | native 引擎为进程内纯本地，假设更强 |
| 单次转换受 15 MB / 60 s 限制 | `edit_chain.rs` 常量 | 阶段 0 不变；阶段 2 后可取消 |
| 前端不直接触达文件系统 | 成立 | 收口后仍成立 |
| 保存不回落 legacy | 新增假设 | **阶段 0 不涉及保存路径，无冲突** |
| 引擎绑定必须跨重启可恢复 | 当前无 engine 字段 | **阶段 2 前需 schema 迁移**（D7=A） |

---

## 10. 疑似不变量破坏

| # | 不变量 | 风险 | 处置 |
|---|---|---|---|
| R1 | `BR-ED-DATA-002` 转换归 DOC、编辑归 ED | 若把 HTML 解析放进 ED 会越界 | 引擎必须留在 `doc/engine/`，ED 只消费 HTML |
| R2 | `BR-DOC-DATA-002` 样式子集语义 | 新引擎不再以「支持清单」表达保真 | 阶段 1 前单独重写该规则与 `StyleSubset` 术语，不在本 Scope 偷偷改 |
| R3 | `BR-DOC-DATA-003` blockId 往返语义 | 新引擎可提供 docxIndex 溯源，可能诱导改变 ID 生成 | D9=A：阶段 0/1/2 保持现有生成方式 |
| R4 | `BR-DE-*` 未决 diff 的基准有效性 | 换引擎后 blockId 不同，旧 diff 不应被应用 | 引擎变更/文档重开时未决 diff 一律 `expired`，不迁移 |
| R5 | 异常退出清算（`BR-SYS-STATE-005`） | 新增 baseline 持久化可能被误当权威 LogicalState | baseline 只作为补丁来源，不得替代 DiskState/LogicalState 判定 |
| R6 | `X-SEQ-004` 二进制解析唯一通道 | 若把解析下沉到前端 | 解析固定在 Rust |

**未发现对 `X-SEQ-002`、`X-FORBID-004`、`X-FORBID-005`、`BR-WS-PERSIST-001` 的破坏。**

---

## 11. 验证计划

### 阶段 0（本 Scope）

1. **编译与静态检查**
   - `cargo check --manifest-path src-tauri/Cargo.toml`
   - `cargo fmt --all --check`
   - `git diff --check`
2. **既有回归（必须全绿，且不得删改断言）**
   - `npm run test:plan -- --files [changed_files]`
   - `npm run test:related -- --files [changed_files]`
   - 选择器必须按实际 changed_files 计算，不得手选测试替代
3. **治理**
   - `npm run governance:audit`
4. **行为等价证据**
   - 阶段 0 收口后，同一批 docx 经 legacy 实现产出的 HTML 与收口前逐字节一致
   - 影子模式只读，不改变返回结果
5. **链路日志**
   - `SM-DOC-01` 事件序列与收口前一致（open→converting→ready / fail）

### 后续阶段（不在本 Scope，先登记预期）

| 阶段 | 验收硬指标 |
|---|---|
| 阶段 1 | 常用块解析覆盖率 ≥ 99%（语料集）；渲染差异清单可解释；回落率 < 1% |
| 阶段 2 | 未修改区域字节一致率 = 100%；生成文件 Word 打开成功率 = 100% |
| 阶段 3 | 现有 DOC/DE/WS 测试全绿；Pandoc 删除后无残留调用 |

---

## Expected Behavior

阶段 0 完成后：

- 所有 docx 打开/保存调用统一经过 `doc::engine` 策略入口；
- 默认 `legacy`，行为与当前**逐字节等价**；
- `BINDER_DOCX_ENGINE=shadow` 时并行记录诊断，但对外结果仍来自 legacy；
- 不依赖 Pandoc 的新引擎可在不改动 ED/DE/AG 契约的前提下接入；
- 任一阶段都可通过开关回到旧路，无需回滚数据。

---

## Validation Commands

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo fmt --all --check
npm run test:plan -- --files <changed_files>
npm run test:related -- --files <changed_files>
npm run governance:audit
git diff --check
```

---

## Scope Lock

> **confirmation 为 PENDING。人类确认前不得修改任何文件。**
> 本 Scope 仅覆盖阶段 0。

```yaml
task_type: CODE_CHANGE
governance_phase: IMPLEMENTATION
allowed_files:
  - .governance/issues/DOCX-NATIVE-ENGINE-01.md          # 本 Trace
  - src-tauri/src/doc/mod.rs                             # 声明 engine 子模块
  - src-tauri/src/doc/engine/mod.rs                      # 新增：DocEngine 策略入口
  - src-tauri/src/doc/engine/legacy.rs                   # 新增：legacy 转发实现
  - src-tauri/src/doc/routes.rs                          # E1/E2/E3 收口
  - src-tauri/src/workspace/routes.rs                    # E4 收口
  - src-tauri/src/de/routes.rs                           # E5 收口
  - src-tauri/src/ag/blocks.rs                           # E6 收口
  - src-tauri/tests/core_chain.rs                        # 新增行为等价回归
forbidden_files:
  - src-tauri/src/doc/edit_chain.rs                      # legacy 实现本体保持原样
  - src-tauri/src/doc/pandoc_runtime.rs                  # 阶段 0 不动
  - src-tauri/src/doc/style_subset.rs                    # 语义重写属阶段 1
  - src-tauri/src/doc/block_backfill.rs                  # 阶段 0 不动
  - src-tauri/src/doc/tool_runtime.rs                    # per-engine 校验属阶段 1
  - src-tauri/src/prv/**                                 # 预览/导出不在阶段 0
  - src/components/**                                    # 前端不改
  - src/services/**                                      # 前端不改
  - docs/**                                              # 设计文档重写属阶段 1
  - ADU.md / ADP.md / AGENTS.md / CLAUDE.md / ADUS.md    # 治理控制文档
  - .governance/test-governance.json                     # 测试治理注册表
confirmation: PENDING
scope_reason: 阶段 0 只做 DOC 引擎策略收口与影子骨架，保持行为逐字节等价，为后续 native 引擎提供可回退接入点。
```

### 关于 `.governance/active_scope.json`

当前该文件由另一条活跃 Scope（`AG-CLOSURE-REPAIR-01` / 嵌套 `AG-TOOL-CONTRACT-RECOVERY-02`）持有，且工作树存在 50+ 个未提交文件。
**本 Trace 不覆盖该文件。** 需人类在下列方案中裁决后，才能写入 DOCX Scope 的镜像：

- 方案 A：先收尾/提交 AG 工作，再切换 active scope；
- 方案 B：以嵌套 work package 形式与 AG Scope 并存；
- 方案 C：另开独立分支，active scope 按分支隔离。

---

## 人类决策（已确认）

| # | 决策 | 结论 |
|---|---|---|
| D1 | 并行形态 | **A：主干内策略并行（strangler）+ 短期特性分支 review** |
| D2 | 开关层级 | **C：编译期 feature + 运行时开关 + 文档级引擎绑定** |
| D3 | 默认引擎 | **A：默认 legacy，手动开 native；稳定后转 B** |
| D4 | 打开回落粒度 | **A+B：整篇回落 legacy；单块失败标 `unmapped`** |
| D5 | 保存是否回落 | **A：绝对不回落** |
| D6 | 会话中切引擎 | **A：禁止，按文档绑定** |
| D7 | 引擎绑定/baseline | **A：workspace.db 持久化** |
| D8 | 影子诊断 | **A：`app_data_dir` JSONL** |
| D9 | blockId 语义 | **A：保持现有生成方式** |
| D10 | 原生 docx 只读语义 | **A：保持不变（改产品语义需另行 APC 授权）** |
| D11 | Pandoc/LO 移除时点 | **A：并行期全部保留** |
| D12 | 验收阈值 | **按建议初值，阶段 1 后校准** |

---

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-10-28 | v1.0 | 建立 DOCX 原生引擎并行改造 Issue Trace；确认 D1–D12；Scope 限阶段 0（策略接口 + 影子骨架），confirmation 待人类确认。 |
