---
文档编号：VAL-X-V-03
文档状态：A
负责模块：CORE
文档职责：产品能力图谱的系统验证卡与人工观察证据登记
上游约束：CORE-C-P-01、CORE-C-P-02、.governance/product-capability-registry.json
直接承接：产品能力沙盘、G5 集成/系统验证、发布验收
使用边界：不替代自动化测试结果；只记录需要真实运行环境或完整用户工作流的验证
变更要求：能力变体的系统验证状态变化时，须同步更新本卡和产品能力注册表
---

# 产品能力图谱验证

## VAL-CORE-EDIT-DIFF-SAVE

```yaml
validation_id: VAL-CORE-EDIT-DIFF-SAVE
title: 文档编辑、代理 Diff 审阅与保存重开的系统路径
scope: Markdown、HTML、纯文本、BDocx 编辑及其受控代理修改路径
source: REQ-ED-001、REQ-ED-008、REQ-DOC-001、REQ-AG-001、REQ-AG-002
preconditions:
  - 在真实桌面运行时打开测试 workspace
  - 准备各格式的最小样例，以及含表格、图片、样式的 BDocx 受支持样例
steps:
  - 用户分别打开 Markdown、HTML、纯文本和 BDocx，执行图谱登记范围内的编辑操作
  - 对可代理编辑的格式发起一次局部修改，审阅并接受或拒绝 Diff
  - 保存、关闭并重开文件，核对内容、结构、格式支持边界及未写盘保障
expected:
  - 支持范围内的修改可保存重开且符合能力变体的 acceptance
  - 不支持或不安全的场景明确阻断或提示，不静默降级为直接写盘
  - 代理提议在用户裁决前不写入文件
status: blocked
blocker: 真实 WKWebView / 代理到工具到 Diff 的确定性端到端环境尚未形成；自动化证据不替代本卡
owner: 人类产品验收 + 对应模块实现负责人
evidence: []
next_action: 建立可复现的桌面运行时夹具后，按格式和代理路径拆分执行记录
```

## 状态口径

- `pending`：验证卡已定义，尚未执行或证据不足。
- `blocked`：验证前置环境或链路尚未具备；不得据此宣称功能未实现，也不得晋级发布状态。
- `passed`：本卡每项预期都有可追溯的真实运行证据。

## 变更记录

| 日期 | 版本 | 变更内容 |
|---|---|---|
| 2026-08-02 | v1.0 | PCG-SPEC-01：建立产品能力图谱首张 G5 系统验证卡，显式登记当前阻断条件 |
