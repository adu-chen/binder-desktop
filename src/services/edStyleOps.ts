import type { Node as PMNode, Mark } from "@tiptap/pm/model";
import { getActiveEditor } from "./edLocate";
import { blockTextLength } from "./edLocate";
import { isContentBlock } from "../components/ed/extensions/BlockIdExtension";

/**
 * 样式协议 ED 侧（PI-ED-11 · A-ED-M-T-01 §5.1 · A-ED-I-P-01 §4.1）。
 * 读侧：样式投影——活动编辑器模型 → 机器可读块结构 {blockId,type,text,marks[],blockAttrs}
 * （marks range 在块内纯文本平面，与字符锚同基准）。
 * 写侧：样式操作（StyleOp）经受控接口进入——`planStyleOps` 纯逻辑（锚校验 + 事务步骤规划，
 * 批内原子：任一锚失效整批拒绝）+ `applyStyleOps` 薄执行壳（单事务应用 → 单撤销步 + 产脏态）。
 * 落盘分流归调用方（裁定 D-3）：回合产出进 DiffPayload 经 DE 裁决（后端扩展待协调），
 * 显式内联经本接口直改（用户编辑语义）。
 */

/** 样式投影：字符 mark 段（range 在块内纯文本平面）。 */
export interface MarkSpan {
  type: string;
  range: [number, number];
  attrs?: Record<string, unknown>;
}

/** 样式投影：块结构（A-ED-M-T-01 §5.1 schema 单一来源）。 */
export interface BlockStyleView {
  blockId: string;
  type: string;
  text: string;
  marks: MarkSpan[];
  blockAttrs: Record<string, unknown>;
}

/** 样式操作（StyleOp，TERM-ED-006）：受控样式修改操作单元。 */
export type StyleOp =
  | { op: "set_mark"; blockId: string; start: number; end: number; mark: { type: string; attrs?: Record<string, unknown> } }
  | { op: "unset_mark"; blockId: string; start: number; end: number; markType: string }
  | { op: "set_block_attr"; blockId: string; attrs: Record<string, unknown> };

const SIMPLE_MARK_TYPES = new Set(["bold", "italic", "underline", "strike", "subscript", "superscript", "code"]);
const SUPPORTED_MARK_TYPES = new Set([...SIMPLE_MARK_TYPES, "highlight", "link", "textStyle"]);
const TEXT_ALIGN_VALUES = new Set(["left", "center", "right", "justify"]);
const HEX_COLOR = /^#[0-9A-Fa-f]{6}$/;
const FONT_SIZE = /^[1-9][0-9]{0,2}(?:\.[0-9]+)?(?:pt|px)$/;

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function exactKeys(value: Record<string, unknown>, allowed: readonly string[]): boolean {
  return Object.keys(value).every((key) => allowed.includes(key));
}

function isSafeHref(value: unknown): value is string {
  if (typeof value !== "string" || !value.trim() || [...value].length > 2048 || /[\u0000-\u001F\u007F]/.test(value)) return false;
  const lower = value.trim().toLowerCase();
  return !lower.startsWith("javascript:") && !lower.startsWith("data:");
}

function isFontFamily(value: unknown): value is string {
  return typeof value === "string" && Boolean(value.trim()) && [...value].length <= 128 && !/[;\u0000-\u001F\u007F]/.test(value);
}

function parseMark(value: unknown): { type: string; attrs?: Record<string, unknown> } | null {
  if (!isRecord(value) || !exactKeys(value, ["type", "attrs"]) || typeof value.type !== "string" || !SUPPORTED_MARK_TYPES.has(value.type)) return null;
  const attrs = value.attrs === undefined ? undefined : isRecord(value.attrs) ? value.attrs : null;
  if (attrs === null) return null;
  if (SIMPLE_MARK_TYPES.has(value.type)) {
    return attrs === undefined ? { type: value.type } : null;
  }
  if (value.type === "highlight") {
    if (!attrs || Object.keys(attrs).length === 0) return { type: value.type };
    return exactKeys(attrs, ["color"]) && typeof attrs.color === "string" && HEX_COLOR.test(attrs.color)
      ? { type: value.type, attrs: { color: attrs.color } }
      : null;
  }
  if (value.type === "link") {
    return attrs && exactKeys(attrs, ["href"]) && isSafeHref(attrs.href)
      ? { type: value.type, attrs: { href: attrs.href } }
      : null;
  }
  if (!attrs || Object.keys(attrs).length === 0 || !exactKeys(attrs, ["color", "fontSize", "fontFamily"])) return null;
  if (attrs.color !== undefined && (typeof attrs.color !== "string" || !HEX_COLOR.test(attrs.color))) return null;
  if (attrs.fontSize !== undefined && (typeof attrs.fontSize !== "string" || !FONT_SIZE.test(attrs.fontSize))) return null;
  if (attrs.fontFamily !== undefined && !isFontFamily(attrs.fontFamily)) return null;
  return { type: value.type, attrs };
}

type NormalizeStyleOpsResult =
  | { ok: true; ops: StyleOp[] }
  | { ok: false; reason: string };

function normalizeStyleOps(value: unknown): NormalizeStyleOpsResult {
  if (!Array.isArray(value)) return { ok: false, reason: "style_operation_invalid: 载荷必须是数组" };
  if (value.length === 0) return { ok: true, ops: [] };
  if (value.length > 64) return { ok: false, reason: "style_operation_invalid: 单批超过 64 项" };
  const ops: StyleOp[] = [];
  for (let index = 0; index < value.length; index += 1) {
    const item = value[index];
    if (!isRecord(item) || typeof item.op !== "string" || typeof item.blockId !== "string" || !item.blockId.trim()) {
      return { ok: false, reason: `style_operation_invalid: 第 ${index + 1} 项缺 op/blockId` };
    }
    if (item.op === "set_block_attr") {
      if (!exactKeys(item, ["op", "blockId", "attrs"]) || !isRecord(item.attrs) || !exactKeys(item.attrs, ["textAlign"])) {
        return { ok: false, reason: `style_operation_invalid: 第 ${index + 1} 项块属性字段非法` };
      }
      if (typeof item.attrs.textAlign !== "string" || !TEXT_ALIGN_VALUES.has(item.attrs.textAlign)) {
        return { ok: false, reason: `style_operation_invalid: 第 ${index + 1} 项 textAlign 非法` };
      }
      ops.push({ op: "set_block_attr", blockId: item.blockId, attrs: { textAlign: item.attrs.textAlign } });
      continue;
    }
    if (!Number.isInteger(item.start) || !Number.isInteger(item.end) || (item.start as number) < 0 || (item.end as number) <= (item.start as number)) {
      return { ok: false, reason: `style_operation_invalid: 第 ${index + 1} 项字符范围非法` };
    }
    if (item.op === "set_mark") {
      if (!exactKeys(item, ["op", "blockId", "start", "end", "mark"])) {
        return { ok: false, reason: `style_operation_invalid: 第 ${index + 1} 项 set_mark 字段非法` };
      }
      const mark = parseMark(item.mark);
      if (!mark) return { ok: false, reason: `style_operation_invalid: 第 ${index + 1} 项 mark/attrs 非法` };
      ops.push({ op: "set_mark", blockId: item.blockId, start: item.start as number, end: item.end as number, mark });
      continue;
    }
    if (item.op === "unset_mark") {
      if (!exactKeys(item, ["op", "blockId", "start", "end", "markType"]) || typeof item.markType !== "string" || !SUPPORTED_MARK_TYPES.has(item.markType)) {
        return { ok: false, reason: `style_operation_invalid: 第 ${index + 1} 项 markType 非法` };
      }
      ops.push({ op: "unset_mark", blockId: item.blockId, start: item.start as number, end: item.end as number, markType: item.markType });
      continue;
    }
    return { ok: false, reason: `style_operation_invalid: 第 ${index + 1} 项 op 不支持` };
  }
  return { ok: true, ops };
}

/** 将不可信 Diff/IPC 载荷按 StyleOp 严格协议解析；任一项非法则整批返回空。 */
export function parseStyleOps(value: unknown): StyleOp[] {
  const parsed = normalizeStyleOps(value);
  return parsed.ok ? parsed.ops : [];
}

/** 内部：叶子内联占位（与 edLocate 同约定，1 字符对齐 PM pos）。 */
const LEAF_CHAR = "￼";

/** blockId 无关的呈现属性剔除表（投影只暴露业务样式属性）。 */
const INTERNAL_ATTRS = new Set(["blockId"]);

function markAttrsOf(m: Mark): Record<string, unknown> | undefined {
  const keys = Object.keys(m.attrs ?? {}).filter((k) => m.attrs[k] !== null && m.attrs[k] !== undefined);
  if (keys.length === 0) return undefined;
  const out: Record<string, unknown> = {};
  for (const k of keys) out[k] = m.attrs[k];
  return out;
}

function sameMark(a: MarkSpan, b: { type: string; attrs?: Record<string, unknown> }): boolean {
  if (a.type !== b.type) return false;
  // 审计修复（2026-07-24）：对 attrs 的 key 排序后比较，避免 `{a:1,b:2}` vs `{b:2,a:1}` 被误判为不等
  const aKeys = Object.keys(a.attrs ?? {}).sort();
  const bKeys = Object.keys(b.attrs ?? {}).sort();
  if (aKeys.length !== bKeys.length) return false;
  for (let i = 0; i < aKeys.length; i++) {
    if (aKeys[i] !== bKeys[i]) return false;
    const k = aKeys[i];
    if ((a.attrs?.[k] ?? null) !== (b.attrs?.[k] ?? null)) return false;
  }
  return true;
}

/**
 * 单块样式投影纯逻辑（BR-ED-DATA-005 读侧核心，供投影与单测共用）：
 * 遍历块内内联内容，按块内纯文本偏移输出 mark 段；相邻同型同 attrs 段合并。
 */
export function extractBlockStyles(block: PMNode): { text: string; marks: MarkSpan[] } {
  const marks: MarkSpan[] = [];
  let text = "";
  let offset = 0;
  block.content.forEach((child) => {
    const len = child.isText ? (child.text?.length ?? 0) : child.isLeaf ? 1 : 0;
    text += child.isText ? (child.text ?? "") : child.isLeaf ? LEAF_CHAR : "";
    for (const m of child.marks) {
      const span: MarkSpan = { type: m.type.name, range: [offset, offset + len], attrs: markAttrsOf(m) };
      const last = marks[marks.length - 1];
      // 相邻同型同 attrs 合并（last.range[1] === span.range[0]）。
      if (last && last.range[1] === span.range[0] && sameMark(last, { type: span.type, attrs: span.attrs })) {
        last.range[1] = span.range[1];
      } else {
        marks.push(span);
      }
    }
    offset += len;
  });
  return { text, marks };
}

/** 文档级样式投影：遍历所有内容块 → BlockStyleView[]（容器块透明、无 blockId 块跳过）。
 *  审计修复（2026-07-25 · A+D 方案）：从 doc.forEach（仅顶层）改为 doc.descendants（递归），
 *  使嵌套在 orderedList/bulletList 中的 listItem 也能被投影。 */
export function projectDocStyles(doc: PMNode): BlockStyleView[] {
  const out: BlockStyleView[] = [];
  doc.descendants((block, _pos) => {
    if (!isContentBlock(block as unknown as { type: { name: string } })) return true;
    const blockId = (block.attrs.blockId as string | null) ?? "";
    if (!blockId) return true;
    const { text, marks } = extractBlockStyles(block);
    const blockAttrs: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(block.attrs)) {
      if (!INTERNAL_ATTRS.has(k) && v !== null && v !== undefined) blockAttrs[k] = v;
    }
    out.push({ blockId, type: block.type.name, text, marks, blockAttrs });
    return false; // 内容块的子节点不重复投影
  });
  return out;
}

/** 块内纯文本偏移 → 块内容 pos（planStyleOps 用；offset 落在文本中间/叶子边界均精确）。 */
export function posFromCharOffset(block: PMNode, offset: number): number {
  let remaining = Math.max(0, offset);
  let pos = 0;
  let done = false;
  block.content.forEach((child) => {
    if (done) return;
    const len = child.isText ? (child.text?.length ?? 0) : child.isLeaf ? 1 : 0;
    if (remaining <= len) {
      pos += child.isText ? remaining : remaining > 0 ? child.nodeSize : 0;
      done = true;
      return;
    }
    remaining -= len;
    pos += child.nodeSize;
  });
  return pos;
}

/** 规划出的事务步骤（执行壳按序应用为单事务）。 */
export type PlannedStep =
  | { kind: "addMark"; from: number; to: number; markType: string; attrs?: Record<string, unknown> }
  | { kind: "removeMark"; from: number; to: number; markType: string }
  | { kind: "setBlockAttrs"; blockPos: number; attrs: Record<string, unknown> };

export type PlanResult = { ok: true; steps: PlannedStep[] } | { ok: false; reason: string };

/**
 * @GOV
 * codes:    BR-ED-DATA-005-RB-ED-ED-EDIT-008
 * type:     RB
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-005
 * term_ref: TERM-ED-006, TERM-ED-004
 * boundary: in=styleOpBatchWithCharAnchors | out=plannedMarkTransactionSteps(atomicAllOrNothingAnchorValidated)
 */
/// 样式操作规划守则（BR-ED-DATA-005 写侧核心，纯逻辑可单测）：每个 StyleOp 先校验锚（blockId 存在
/// + 偏移区间 ≤ 块文本长），**任一锚失效整批拒绝**（批内原子，不产生半应用状态）；校验通过后把
/// 字符锚区间换算为 PM 位置输出事务步骤。样式修改必经受控接口（X-FORBID-005），本函数是唯一规划面。
export function planStyleOps(doc: PMNode, ops: StyleOp[]): PlanResult {
  const normalized = normalizeStyleOps(ops);
  if (!normalized.ok) return { ok: false, reason: normalized.reason };
  ops = normalized.ops;
  if (ops.length === 0) return { ok: true, steps: [] };
  // 内容块索引：blockId → {node, blockPos}。审计修复（2026-07-25 · A+D）：改为 descendants 遍历支持嵌套块
  const index = new Map<string, { node: PMNode; blockPos: number }>();
  doc.descendants((node, pos) => {
    if (!isContentBlock(node as unknown as { type: { name: string } })) return true;
    const id = (node.attrs.blockId as string | null) ?? "";
    if (id) index.set(id, { node, blockPos: pos });
    return false; // 内容块子节点不索引
  });
  const steps: PlannedStep[] = [];
  for (const op of ops) {
    const hit = index.get(op.blockId);
    if (!hit) return { ok: false, reason: `anchor_lost: 块不存在 ${op.blockId}` };
    if (op.op === "set_block_attr") {
      if (!Object.prototype.hasOwnProperty.call(hit.node.attrs, "textAlign")) {
        return { ok: false, reason: `style_operation_invalid: 块不支持 textAlign ${op.blockId}` };
      }
      steps.push({ kind: "setBlockAttrs", blockPos: hit.blockPos, attrs: op.attrs });
      continue;
    }
    const textLen = blockTextLength(hit.node);
    if (op.start < 0 || op.end < op.start || op.end > textLen) {
      return { ok: false, reason: `anchor_lost: 偏移越界 ${op.blockId}[${op.start},${op.end}] len=${textLen}` };
    }
    if (op.start === op.end) return { ok: false, reason: `invalid_range: 空区间 ${op.blockId}` };
    const contentStart = hit.blockPos + 1;
    const from = contentStart + posFromCharOffset(hit.node, op.start);
    const to = contentStart + posFromCharOffset(hit.node, op.end);
    if (op.op === "set_mark") {
      if (!hit.node.type.schema.marks[op.mark.type]) {
        return { ok: false, reason: `unknown_mark: ${op.mark.type}` };
      }
      steps.push({ kind: "addMark", from, to, markType: op.mark.type, attrs: op.mark.attrs });
    } else {
      if (!hit.node.type.schema.marks[op.markType]) {
        return { ok: false, reason: `unknown_mark: ${op.markType}` };
      }
      steps.push({ kind: "removeMark", from, to, markType: op.markType });
    }
  }
  return { ok: true, steps };
}

/** 读侧入口：活动文档样式投影（BR-ED-DATA-005 读侧；无活动编辑器 → null）。
 *  AG 消费面（ag.list_blocks 样式字段）由 AG 侧后续承接（A-AG-I-P-01 §4.1）；活动文档以编辑器
 *  内存模型为权威真源（含未保存编辑），非活动文档走后端文本级路径（无样式）。 */
export function listActiveDocStyles(): BlockStyleView[] | null {
  const ed = getActiveEditor();
  if (!ed) return null;
  return projectDocStyles(ed.state.doc);
}

/** 写侧入口：批内原子应用 StyleOp（显式内联直改路径，裁定 D-3 B；回合产出勿经此——走 DE diff）。
 *  单事务应用（单撤销步）→ 触发 onUpdate 产脏态（用户编辑语义，SM-ED-01→SM-WS-01）。
 *  返回 planStyleOps 的结果（失败含 anchor_lost 明确原因，调用方转达消费方）。 */
export function applyStyleOps(ops: StyleOp[]): PlanResult {
  const ed = getActiveEditor();
  if (!ed) return { ok: false, reason: "no_active_editor" };
  const plan = planStyleOps(ed.state.doc, ops);
  if (!plan.ok || plan.steps.length === 0) return plan;
  const { state, view } = ed;
  const tr = state.tr;
  for (const s of plan.steps) {
    if (s.kind === "addMark") {
      const markType = state.schema.marks[s.markType];
      if (!markType) return { ok: false, reason: `unknown_mark: ${s.markType}` };
      tr.addMark(s.from, s.to, markType.create(s.attrs));
    } else if (s.kind === "removeMark") {
      const markType = state.schema.marks[s.markType];
      if (!markType) return { ok: false, reason: `unknown_mark: ${s.markType}` };
      tr.removeMark(s.from, s.to, markType);
    } else {
      const node = tr.doc.nodeAt(s.blockPos);
      if (!node) return { ok: false, reason: "anchor_lost: 块已变动" };
      tr.setNodeMarkup(s.blockPos, undefined, { ...node.attrs, ...s.attrs });
    }
  }
  view.dispatch(tr);
  return plan;
}
