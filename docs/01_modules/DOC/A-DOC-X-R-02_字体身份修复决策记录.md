---
文档编号：DOC-X-R-02
文档状态：A
负责模块：DOC
文档职责：PI-DOC-03 字体专项期间的技术决策记录——CoreText 本地化名→英文名注册、Office 缺失字体别名静态表、放弃 FindFontSubstitute 深度改造方案
上游约束：A-CORE-C-P-01_APC.md、A-DOC-X-L-01_文档转换开发计划.md（PI-DOC-03 S2/S3）
直接承接：字体身份修复补丁 `binder_font_identity.patch`、S3 验收
使用边界：本文记录专项迭代中的方向性决策与放弃方案；不是 triage 报告，不定义注册块，不参与 audit 豁免或 ADUS triage 汇总
变更要求：决策变更时更新对应条目并追加变更记录
治理成熟度：draft
---

# DOC 字体专项决策记录（字体身份修复）

## §1 背景

2026-07-23 至 2026-07-24，经过 S0/S1/S3/S4/S5 验证、Writer 链路追踪与 CoreText dump，确认随机性的根本原因为：
CoreText 在非英文 locale 返回本地化字体显示名，而 OOXML 请求英文名；
`GetEnglishSearchFontName()` 无法处理 CJK 本地化名，使 `FindFontFamily("PingFang SC")` 失败，
随后 `FindFontFamilyByAttributes` 的无序遍历产生随机选择。

## §2 决策 1：放弃 FindFontSubstitute 深度改造

原计划（S3）是在 `CoreTextGlyphFallbackSubstitution::FindFontSubstitute` 中按 shaping run/grapheme
cluster 选择确定性候选。追踪证明该函数不在 CJK 字体选择热路径上：整个 DOCX 转换仅调用 4 次，
每次仅处理 1–2 个缺失字符。CJK 选择发生在 `OutputDevice::SetFont` → `ImplNewFont` →
`ImplFontCache::GetFontInstance` → `PhysicalFontCollection::FindFontFamily`，故放弃该方案。

## §3 决策 2：采用字体身份修复方案

在 `SystemFontList` 的 `fontEnumCallBack` 中，为每个枚举字体同时注册英文名，使用
`CTFontDescriptorCopyAttribute(pFD, kCTFontFamilyNameAttribute)` 获取 family name。
这样 `FindFontFamily("PingFang SC")` 会命中指向 蘋方-簡 CTFontDescriptor 的英文索引，
`Arial` 等原英文名保持不变。补丁为 `patches/libreoffice/binder_font_identity.patch`；
变更为 `SystemFontList.hxx` +8 行、`SystemFontList.cxx` +130 行；PingFang SC 20/20 次转换一致。

## §4 决策 3：加入静态 Office 字体别名表

在注册阶段为 macOS 缺失的 Office 字体建立稳定别名：Microsoft YaHei / 等线 → PingFang SC；
SimSun / 宋体 → Songti SC；PMingLiU → PingFang TC；MS Gothic/Mincho → Hiragino Sans/Mincho ProN；
Malgun Gothic / Gulim → Apple SD Gothic Neo；Aptos / Calibri / Segoe UI → Arial；Consolas → Courier New。
这些是稳定生态知识，不增加运行时扫描变量。Microsoft YaHei 20/20 次转换一致。

## §5 决策 4：不引入动态字体扫描与脚本级 fallback

放弃 Rust 字体扫描模块、`detectScriptFromName()` 和动态 `font-alias.json`：它们增加复杂度且不能可靠判断
字体脚本。LO 自身已有 glyph fallback；本专项只需保证 font family identity 正确。

## §6 决策 5：添加最小第 3 层兜底

当 PPTX 使用不在别名表中的字体，仍可能进入 `FindFontFamilyByAttributes` 的无序遍历。因此在
`ImplFindFontFamilyBySearchName` 的最终 `return nullptr` 前，按固定优先级尝试 PingFang SC、
PingFang TC、Hiragino Sans、Apple SD Gothic Neo、Arial（`PhysicalFontCollection.cxx` +12 行）。
该方案不依赖脚本检测，也不构建动态 fallback 引擎。

## §7 与旧补丁的关系

- `binder_deterministic_glyph_fallback.patch`：其 FindFontSubstitute 改造不在热路径，可废弃或仅保留 `binder-fonts/` 注册部分。
- `binder_font_identity.patch`：为正式替代补丁，承接新的字体策略。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-24 | v1.0 | 登记字体专项的方案取舍和验证证据 |
| 2026-08-01 | v2.0 | 更名为技术决策记录；退出 triage 和 audit 豁免体系，决策内容保持不变 |
