# DOC-SYNC-01：最近两天课题 → 文档 / 台账同步

## 背景

2026-08-05 ~ 08-07 共 20 个提交、约 14 个功能课题完成实现，但存在文档 / 台账未同步缺口：
- 功能明细表台账 xlsx（PCG-SPEC-01 一次性构建，143 行）自 08-05 后未更新，无生成管线；
- 产品能力注册表 / 图谱生成物停留在 08-05 状态；
- 模块文档存在完全未同步（ED 工具栏优化、WS-NEW-BUTTON-01）与部分同步缺口；
- 项目进度台账 A-CORE-X-L-05 自 2026-07-17 v1.1 后零更新。

## 人类决策（2026-08-07）

| 决策 | 结论 |
|---|---|
| D1 WS-NEW-BUTTON-01 文档缺口 | 补同步 |
| D2 台账 xlsx 更新方式 | 新增生成脚本，并入 product:generate |
| D3 台账状态口径 | 代码已实现 → 「已实现 · 待核验 · EP」 |
| D4 能力注册表扩展深度 | 全量登记（含右键菜单、选区持久化） |

## 范围

- 新增 `.governance/function-ledger.json`（功能明细台账数据源，147 行）；
- 新增 `scripts/generate_function_ledger.mjs`（由数据源生成 binder-product-function-ledger.xlsx）；
- `package.json` `product:generate` 串联台账生成；
- 扩展 `.governance/product-capability-registry.json` 并重跑图谱生成；
- 回填模块文档（ED / WS / ENT）与全局进度台账（A-CORE-X-L-05 / 06）；
- 不改运行时代码、不新增/修改 RULE/CHAIN/TERM/MODULE 静态定义。

## 产物清单

| 产物 | 状态 |
|---|---|
| .governance/function-ledger.json | 数据源（147 行） |
| scripts/generate_function_ledger.mjs | 生成脚本 |
| docs/00_core/generated/binder-product-function-ledger.xlsx | 重新生成 |
| .governance/product-capability-registry.json | 扩展 |
| docs/01_modules/ED/A-ED-M-T-01、A-ED-M-D-01、A-ED-X-R-03 | 回填 |
| docs/01_modules/WS/A-WS-M-D-01、A-WS-I-P-01、A-WS-M-T-01 | 回填 |
| docs/01_modules/ENT/A-ENT-M-T-01、A-ENT-I-P-01 | 回填 |
| docs/00_core/A-CORE-X-L-05、A-CORE-X-L-06 | 回填 |

## 验证

- `npm run product:generate`（图谱 + 台账）
- `npm run product:audit`
- `npm run governance:audit`
- `npm run test:governance`
