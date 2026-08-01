# Issue Trace — BDocx 表格邻近移动控件

## 1. 用户可见症状

表格整体上移/下移已经实现，但入口位于编辑器顶部表格工具组，距离正文中的目标表格较远，用户不易
发现其与当前表格的关系。用户希望在激活表格附近临时出现上下按钮；不要求拖动或任意位置移动。

## 2. 入口点

- 当前 UI：`src/components/ed/EditorToolbar.tsx::EditorToolbar` 的 `↑表/↓表`。
- 移动事务：`src/services/edTableOps.ts::moveActiveTable` →
  `applyTableOperations` → `moveTable`。
- 编辑器容器：`src/components/ed/Editor.tsx::Editor` 的 `.editor-paper` 与 TipTap EditorView。

## 3. 相关规则

- `BR-ED-DATA-007`：TableOperation 必须以 tableId、fingerprint 和原子 ProseMirror 事务进入受控路径。
- `BR-ED-DATA-003`：分页、缩放及邻近控件属于呈现层，不得改写文档模型。
- `X-FORBID-005`：用户编辑只能经 ED 受控接口进入 LogicalState。

## 4. 相关链路

人工路径：`ED-EDIT：表格选区 → 邻近按钮 → moveActiveTable → TableOperation → LogicalState`

代理路径不变：`AG-TOOL → DE-DIFF → ED-EDIT`。

## 5. 生产者块

- `activeTableSnapshot`：生产当前 tableId、fingerprint 与 moveDirections。
- TipTap EditorView：生产当前选区与目标表格 DOMRect。
- Editor 的 scroll/resize/transaction 事件：生产邻近控件重定位信号。

## 6. 消费者块

- `TableMoveControls`：消费活动表格快照和 DOMRect，呈现临时按钮。
- `moveActiveTable`：消费方向，复用既有 TableOperation 执行器。
- `Editor`：挂载控件并提供当前 paper 可视边界。

## 7. 路径上的空规则块

新控件是 BR-ED-DATA-007 既有移动操作的呈现入口，不新建业务规则。实现块须映射
`BR-ED-DATA-007`；纯几何计算可保持无副作用并由测试覆盖。

## 8. boundary 假设

- 活动表格必须从当前 ProseMirror selection 解析，不缓存跨文档 tableId。
- 控件坐标使用 viewport DOMRect；按钮自身不受编辑器 zoom 缩放。
- `mousedown` 阻止默认行为，避免点击按钮前丢失表格选区。

## 9. 运行时假设

- 控件只在活动表格存在时显示；滚动、缩放、窗口 resize、选区和事务变化时重新定位。
- 表格完全离开 editor paper 可视区时控件隐藏。
- 点击移动后重新把选区定位到同一 tableId 的首个单元格，使控件继续跟随移动后的表格。

## 10. 疑似不变量破坏

- 若按钮点击先让编辑器失焦，`activeTableSnapshot` 会丢失，移动无效。
- 若只在 React render 时计算坐标，滚动/缩放后控件会漂移。
- 若移动后不恢复表格选区，控件会在第一次点击后消失。
- 若保留顶部按钮，会形成两个不一致的人工入口。

## 11. 验证计划

- 几何单测：表格附近位置、可视区钳制、离开可视区隐藏。
- 组件单测：活动表格显示按钮、方向禁用、点击发出移动、`mousedown` 不丢选区。
- TableOperation 单测：移动后同一 tableId 保持激活；既有上移规划继续通过。
- 工具栏回归：顶部不再出现 `↑表/↓表`，其他表格操作不变。
- 类型检查、相关 Vitest、`governance:generate`、`governance:audit`、`git diff --check`。

## Scope Lock

- scope_id：`BDOCX-TABLE-MOVE-CONTROLS-01`
- task_type：`CODE_CHANGE`
- confirmation：以 `.governance/active_scope.json` 的人类确认值为准。
- allowed_files / forbidden_files：以 `.governance/active_scope.json` 为机器可读权威。

## 实现与验证结论（2026-07-28）

状态：`READY_FOR_MANUAL_ACCEPTANCE`

- 已新增 `TableMoveControls`：活动表格附近临时显示上下按钮，按 `moveDirections` 禁用不可执行方向；
  表格离开 paper 可视区或选区离开表格时隐藏。
- 已移除顶部表格工具组的 `↑表/↓表`，其余行列、表头、合并/拆分、底纹和删除表格入口不变。
- 控件监听选区、事务、paper 滚动、窗口 resize 和 zoom 变化；`mousedown` 保留编辑器选区。
- 人工移动复用既有 TableOperation 规划与原子应用，并在同一事务中把选区恢复到同一 tableId，
  允许连续上移/下移。
- 相关 Vitest：3 个测试文件、12 个测试全部通过。
- TypeScript：`npx tsc --noEmit` 通过。
- 治理：`npm run governance:generate` 与 `npm run governance:audit` 通过。
- 全量 Vitest：49 个测试文件通过、1 个失败；376 个测试通过、4 个失败。失败均位于既有
  `src/services/edLocate.test.ts` 的 `resolveSelectionAnchor` 偏移断言，与本课题基线相同且不在
  Scope 内。
- 待人工验收：Tauri/WKWebView 中控件实际位置、滚动/缩放跟随、连续移动以及主题视觉效果。

### 人工反馈后的同 Scope 迭代

- 人工反馈：编辑状态下未显示邻近按钮；同时要求删除表格入口并入该按钮组。
- 根因收敛：首版显示依赖 `nodeDOM(tablePos)` 和 ProseMirror 状态事件；真实 TipTap TableView
  可能返回 wrapper，且 WebKit 再次点击相同选区不保证产生 `selectionUpdate`，形成显示阻断点。
- 修复：优先从当前选区 `domAtPos` 向上查找 table，并保留 nodeDOM fallback；补充
  `pointerup/keyup/focusin` 与嵌套 scroll 捕获；即使 tableId 补发事务尚未完成也先显示控件和删除
  入口，上下移动在快照就绪前保持禁用。
- 删除表格：迁入邻近控件并调用既有 `deleteTable` 命令；顶部表格工具组删除该重复入口。
- 更新后相关 Vitest：3 个测试文件、14 个测试全部通过，覆盖 Text DOM 向上识别 table、
  tableId 补发前显示删除入口、pointerup 重新唤起、方向禁用、移动和删除命令。
- 更新后全量 Vitest：49 个测试文件通过、1 个失败；378 个测试通过、4 个失败。失败仍全部来自
  Scope 外既有 `edLocate.test.ts::resolveSelectionAnchor` 偏移断言。
- `npx tsc --noEmit`、治理生成/审计及 `git diff --check` 通过后，状态恢复为
  `READY_FOR_MANUAL_ACCEPTANCE`；本机界面控制服务不可用，未把 Tauri 可视效果伪报为已验证。

### 第二次人工反馈：位置错位

- 截图证据：表格位于第二页顶部，但控件出现在编辑区右下角；显示链已恢复，定位坐标系错误。
- 根因：首版把 viewport DOMRect 直接写入 `position: fixed`；Tauri WebKit 在当前 grid/overflow
  组合中实际定位包含块与预期 viewport 不一致，导致横纵方向同时漂移。
- 修复：控件改为 `editor-host` 内 `position:absolute`，用 table/paper 的 viewport DOMRect 计算
  邻近点后减去 host DOMRect 原点，统一为 host 局部坐标；默认放表格右上侧，右侧不足回落左侧。
- 继续保留 paper 可视区判断、上下钳制、滚动/缩放/事务重算以及删除按钮。

### 第三次人工反馈：废止祖先坐标换算

- 人工复测：editor-host 局部坐标方案仍离表格很远，证明分页、滚动和 zoom 组合下祖先布局坐标不能
  作为稳定锚。
- 新方案：控件通过 React portal 直接挂到 `document.body`，恢复 `position:fixed`，但不再作为
  editor-host 后代；位置只读取当前活动 table 的 `getBoundingClientRect()`，与浮层严格共享
  viewport 坐标系。
- 已移除 `hostRef` 和所有 host 原点减法。paper DOMRect 仅用于可见性判断与边界钳制，不参与跨坐标系
  换算。

### 第四次人工反馈：改为 DOM 归属锚定

- 人工复测：body portal + table DOMRect 的效果目测仍与修复前一致，说明运行环境中的 table 几何值
  也不能作为可靠位置真值。
- 依赖核实：TipTap `TableView` 固定生成 `.tableWrapper > table`，其 `ignoreMutation` 对 wrapper
  内、`contentDOM` 外的 childList/attribute/characterData 返回 true，允许安全附加纯交互 UI。
- 最终锚定方案：删除全部 top/left、viewport、paper、host 和 scroll 坐标计算；React portal 直接
  挂载到活动表格的 `.tableWrapper`，CSS 以 wrapper 内 `position:absolute; top:6px; right:6px`
  定位。按钮随表格 DOM 自然移动，不再存在跨坐标系漂移。
- 更新后相关 Vitest：3 个测试文件、13 个测试通过；组件测试明确断言控件父节点就是活动
  `.tableWrapper`，且不存在内联 top/left。
- 更新后全量 Vitest：49 个测试文件通过、1 个失败；377 个测试通过、4 个失败。失败仍全部来自
  Scope 外既有 `edLocate.test.ts::resolveSelectionAnchor`。

### 第五次人工反馈：防止覆盖单元格

- 截图证据：DOM 归属锚定已生效，但竖向按钮组覆盖表格右上区域及最后一列。
- 优化：按钮组改为右上方横向迷你工具条，按钮缩为 22×22；删除按钮使用左分隔线。
- `.tableWrapper` 为所有表格统一保留 `var(--sp-3)` 顶部安全区，使工具条位于表格上方且不遮挡
  单元格；安全区不随选区增删，避免激活表格时正文发生跳动。
- 保持 wrapper portal 锚定，不恢复任何几何或滚动坐标计算。

## 课题闭合（2026-07-28）

- final_state：`CLOSED`
- 人类决定：2026-07-28 明确要求“闭合本次课题”。
- 闭合范围：表格移动入口从顶部工具组迁移到活动 `.tableWrapper`；上下逐块移动、不可用方向禁用、
  移动后同一 tableId 继续激活、删除表格入口合并、WebKit 显示唤起，以及不遮挡单元格的横向迷你
  工具条布局。
- 最终锚定：控件直接 portal 到 TipTap TableView wrapper，不依赖 viewport、paper、host、分页或
  滚动坐标换算。
- 自动验证：相关测试 13/13、TypeScript、`governance:generate`、`governance:audit` 与
  `git diff --check` 通过；最近一次全量前端为 377/381。
- 范围外保留：`edLocate.test.ts::resolveSelectionAnchor` 既有 4 项失败；拖动/任意位置、浮动定位、
  嵌套表格跨容器移动和相邻表格交换不属于本课题，也不随闭合被声明实现。
