import StarterKit from "@tiptap/starter-kit";
import type { Extensions } from "@tiptap/core";
import { BlockIdExtension } from "./BlockIdExtension";

/**
 * ED 编辑器扩展注册入口（PI-FE-05）。
 * 统一在此组装 TipTap 扩展集，固定初始化顺序，避免后续 PI 插入扩展时破坏依赖。
 *
 * 预留槽（按 docs/binder_ui README §3，后续 PI 填充）：
 *   - DiffDecorationExtension  → GreenReview（TERM-ED-003），PI-FE-07（基于 ProseMirror Decoration，
 *                                叠加不替换 DOM，X-FORBID-005）
 *   - CopyReferenceExtension   → @mention 引用复制（REF），后续 PI
 *   - PageTopCaretExtension    → 分页顶部光标，后续 PI
 *   - FontSize                 → 字号，后续 PI
 */
export function buildEditorExtensions(): Extensions {
  return [
    // 基础富文本（段落/标题/列表/引用/代码块/加粗等）。
    StarterKit,
    // 稳定 blockId（BR-ED-DATA-001）——必须在 StarterKit 之后，为其块类型附加全局属性。
    BlockIdExtension,
    // —— 预留：DiffDecorationExtension（PI-FE-07）——
    // —— 预留：CopyReferenceExtension ——
    // —— 预留：PageTopCaretExtension ——
    // —— 预留：FontSize ——
  ];
}
