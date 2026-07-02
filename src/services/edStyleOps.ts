import type { Node as PMNode, Mark } from "@tiptap/pm/model";
import { getActiveEditor } from "./edLocate";
import { blockTextLength } from "./edLocate";

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
  return a.type === b.type && JSON.stringify(a.attrs ?? {}) === JSON.stringify(b.attrs ?? {});
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

/** 文档级样式投影：全部顶层块 → BlockStyleView[]（无 blockId 块跳过——不产无锚投影）。 */
export function projectDocStyles(doc: PMNode): BlockStyleView[] {
  const out: BlockStyleView[] = [];
  doc.forEach((block) => {
    const blockId = (block.attrs.blockId as string | null) ?? "";
    if (!blockId) return;
    const { text, marks } = extractBlockStyles(block);
    const blockAttrs: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(block.attrs)) {
      if (!INTERNAL_ATTRS.has(k) && v !== null && v !== undefined) blockAttrs[k] = v;
    }
    out.push({ blockId, type: block.type.name, text, marks, blockAttrs });
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
  if (ops.length === 0) return { ok: true, steps: [] };
  // 顶层块索引：blockId → {node, blockPos}
  const index = new Map<string, { node: PMNode; blockPos: number }>();
  let p = 0;
  doc.forEach((node) => {
    const id = (node.attrs.blockId as string | null) ?? "";
    if (id) index.set(id, { node, blockPos: p });
    p += node.nodeSize;
  });
  const steps: PlannedStep[] = [];
  for (const op of ops) {
    const hit = index.get(op.blockId);
    if (!hit) return { ok: false, reason: `anchor_lost: 块不存在 ${op.blockId}` };
    if (op.op === "set_block_attr") {
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
      steps.push({ kind: "addMark", from, to, markType: op.mark.type, attrs: op.mark.attrs });
    } else {
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
