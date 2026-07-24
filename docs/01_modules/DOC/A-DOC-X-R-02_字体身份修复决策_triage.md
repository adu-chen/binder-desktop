---
文档编号：DOC-X-R-02
文档状态：A
负责模块：DOC
文档职责：PI-DOC-03 字体专项期间的设计决策登记——CoreText 本地化名→英文名注册、Office 缺失字体别名静态表、放弃 FindFontSubstitute 深度改造方案
上游约束：A-CORE-C-P-01_APC.md、A-DOC-X-L-01_文档转换开发计划.md（PI-DOC-03 S2/S3）
直接承接：字体身份修复补丁 `binder_font_identity.patch`、S3 验收
使用边界：本文是 triage 登记，记录专项迭代中的方向性决策与放弃方案；不定义注册块
变更要求：决策变更时更新对应条目并追加变更记录
治理成熟度：draft
---

# DOC 字体专项决策登记（字体身份修复）

## §1 背景

2026-07-23 至 2026-07-24，经过多轮（S0/S1/S3/S4/S5 验证 + Writer 链路追踪 + CoreText dump）
深入审计 LO 的字体选择全链路，确定随机性的根本原因为：

**CoreText 在非英文系统 locale 下返回字体的本地化显示名（如 "蘋方-簡"），而 OOXML
文档请求的是 ASCII 英文名（"PingFang SC"）。LO 的 `GetEnglishSearchFontName()` 无法
处理 CJK 本地化名，导致 `FindFontFamily("PingFang SC")` hash 查找失败，进而进入
`FindFontFamilyByAttributes` 遍历 `unordered_map` 触发随机选择。**

这个根因同时解释了之前四个验证层（FindFontFamilyByAttributes、ImplFindFontFamilyBySearchName、
FindFontSubstitute、CTFontCreateForString）全部拦截失败的诡异现象——CJK 字体根本不在
PhysicalFontCollection 中，所以这四层根本不会被触发。

## §2 决策 1：放弃 FindFontSubstitute 深度改造

**原计划（S3）**：改造 `CoreTextGlyphFallbackSubstitution::FindFontSubstitute`，按
shaping run/grapheme cluster 选择确定性候选。

**决策**：放弃此方案。原方案目标函数不在 CJK 字体选择的热路径上——追踪证实
`FindFontSubstitute` 在整个 DOCX 转换中仅被调用 4 次（每次 1-2 个缺失字符），
远不是 CJK 字体随机的主因。

**理由**：CJK 字体选择发生在 VCL 层 `OutputDevice::SetFont` → `ImplNewFont` →
`ImplFontCache::GetFontInstance` → `PhysicalFontCollection::FindFontFamily`，而非
glyph fallback 层。

## §3 决策 2：采用字体身份修复方案

**方案**：在 `SystemFontList` 的 `fontEnumCallBack` 中，为每个枚举到的字体同时
注册英文名（通过 `CTFontDescriptorCopyAttribute(pFD, kCTFontFamilyNameAttribute)`）。

**效果**：
- `FindFontFamily("PingFang SC")` → 命中 "PingFang SC" → 实际指向 蘋方-簡 的 CTFontDescriptor
- `FindFontFamily("Arial")` → 命中 "Arial"（本来就是英文名，不变）

**改动**：`SystemFontList.hxx` +8 行、`SystemFontList.cxx` +130 行。补丁保存为
`patches/libreoffice/binder_font_identity.patch`。

**验证**：PingFang SC 20/20 次转换完全一致（嵌入 PingFangSC-Regular）。

## §4 决策 3：加入静态 Office 字体别名表

**方案**：在字体注册阶段，对已知 Office 生态中 macOS 缺失的字体（如 Microsoft YaHei、
Aptos、Calibri、等线），注册硬编码别名指向本机已有字体。

**别名表**：
- 简体中文：Microsoft YaHei / 等线 → PingFang SC；SimSun / 宋体 → Songti SC
- 繁体中文：PMingLiU → PingFang TC
- 日文：MS Gothic/Mincho → Hiragino Sans/Mincho ProN
- 韩文：Malgun Gothic / Gulim → Apple SD Gothic Neo
- 拉丁：Aptos / Calibri / Segoe UI → Arial
- 等宽：Consolas → Courier New

**理由**：这些是稳定的生态知识，不是运行环境变量。静态表维护成本零，后续 LO
升级只需保留这一段。

**验证**：Microsoft YaHei 20/20 次转换完全一致（嵌入 PingFangSC-Regular）。

## §5 决策 4：不引入动态字体扫描与脚本级 fallback

**考虑过但放弃**：
- Rust 字体扫描模块：对当前需求过重；Binder 不是字体管理系统
- `detectScriptFromName()` 脚本检测：字体名不可靠判断脚本（"FZHei" 不包含中文字符但是 CJK）
- 动态 font-alias.json：增加变量而不减少风险

**理由**：LO 自身已有 glyph fallback 处理真正的缺字场景。当前修复只需保证
font family identity 正确即可。

## §6 决策 5（追加）：添加最小第3层兜底

**原情况**：§5 决定不引入脚本级 fallback。但实测发现，当 PPTX 使用不在别名表中的字体时，
LO 仍会进入 `FindFontFamilyByAttributes` 的 `unordered_map` 随机遍历。

**决策**：在 `ImplFindFontFamilyBySearchName` 的最终 return nullptr 前，按固定优先级尝试
本机默认字体（PingFang SC → PingFang TC → Hiragino Sans → Apple SD Gothic Neo → Arial）。

**改动**：`PhysicalFontCollection.cxx` +12 行。

**理由**：不依赖脚本检测，不做动态选择。只是：当所有查找失败时，用确定的默认字体
替代随机遍历。和 §5 的决策一致——不引入复杂 fallback 引擎。

## §7 与旧补丁的关系

- `binder_deterministic_glyph_fallback.patch`：FindFontSubstitute 确定性改造已被证实
  不在热路径上，可废弃或仅保留其中的 `binder-fonts/` 目录注册部分。
- `binder_font_identity.patch`：为新的正式补丁，替代上述补丁的字体策略部分。

## 变更记录

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-07-24 | v1.0 | 登记 4 条字体专项决策：放弃 FindFontSubstitute 改造、英文名注册、静态别名表、不引入动态扫描 |
