import type { Editor } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import type {
  AgentEditableBlockSnapshot,
  StructuredBlockEditPayload,
  StructuredBlockOperation,
} from "../store/edStore";

const BLOCK_OP_CAPABILITY = "replace_node";
const PROTECTED_ANCESTOR_TYPES = new Set([
  "table",
  "tableRow",
  "tableCell",
  "tableHeader",
]);

function stableJson(value: unknown): string {
  if (value === undefined) return "null";
  if (value === null || typeof value !== "object") return JSON.stringify(value) ?? "null";
  if (Array.isArray(value)) return `[${value.map(stableJson).join(",")}]`;
  const record = value as Record<string, unknown>;
  return `{${Object.keys(record).sort().map((key) => `${JSON.stringify(key)}:${stableJson(record[key])}`).join(",")}}`;
}

function semanticBlockType(node: PMNode): string {
  if (node.type.name === "paragraph") return "p";
  if (node.type.name === "heading") {
    const level = Number(node.attrs?.level);
    return Number.isInteger(level) && level >= 1 && level <= 6 ? `h${level}` : "heading";
  }
  if (node.type.name === "codeBlock") return "pre";
  if (node.type.name === "bulletList") return "ul";
  if (node.type.name === "orderedList") return "ol";
  if (node.type.name === "listItem") return "li";
  if (node.type.name === "blockquote") return "blockquote";
  return node.type.name;
}

function fnv1a(value: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

/** ProseMirror Block 节点指纹；只覆盖节点类型、attrs 与内容，不包含易漂移的文档位置。 */
export function fingerprintBlockNode(node: PMNode | Record<string, unknown>): string {
  const json = "toJSON" in node && typeof node.toJSON === "function"
    ? node.toJSON()
    : node;
  return `block-v1-${fnv1a(stableJson(json))}`;
}

function nearestParentBlock(doc: PMNode, pos: number): { blockId: string | null; depth: number } {
  const resolved = doc.resolve(pos);
  let blockDepth = 0;
  for (let depth = 0; depth <= resolved.depth; depth += 1) {
    if (typeof resolved.node(depth).attrs?.blockId === "string") blockDepth += 1;
  }
  for (let depth = resolved.depth; depth >= 0; depth -= 1) {
    const blockId = resolved.node(depth).attrs?.blockId;
    if (typeof blockId === "string" && blockId) {
      return { blockId, depth: blockDepth };
    }
  }
  return { blockId: null, depth: blockDepth };
}

function hasProtectedAncestor(doc: PMNode, pos: number): boolean {
  const resolved = doc.resolve(pos);
  for (let depth = resolved.depth; depth >= 0; depth -= 1) {
    if (PROTECTED_ANCESTOR_TYPES.has(resolved.node(depth).type.name)) return true;
  }
  return false;
}

function containsProtectedDescendant(node: PMNode): boolean {
  let protectedNode = false;
  node.descendants((child) => {
    if (child.type.name === "image" || PROTECTED_ANCESTOR_TYPES.has(child.type.name)) {
      protectedNode = true;
      return false;
    }
    return !protectedNode;
  });
  return protectedNode;
}

function projectMarks(node: PMNode): AgentEditableBlockSnapshot["marks"] {
  const marks: AgentEditableBlockSnapshot["marks"] = [];
  let offset = 0;
  node.descendants((child) => {
    if (child.isText) {
      const text = child.text ?? "";
      const end = offset + text.length;
      for (const mark of child.marks) {
        marks.push({
          type: mark.type.name,
          range: [offset, end],
          ...(Object.keys(mark.attrs ?? {}).length ? { attrs: { ...mark.attrs } } : {}),
        });
      }
      offset = end;
      return false;
    }
    if (child.isLeaf) {
      offset += 1;
      return false;
    }
    return true;
  });
  return marks;
}

/**
 * 从当前 ProseMirror LogicalState 直接生成代理层级块投影。
 *
 * 所有带 blockId 的节点都提供上下文；只有不处于表格、且不含图片/结构节点的 textblock
 * 具备 replace_node 能力。列表项和列表容器因此只作为层级上下文，其段落子节点才是可写目标。
 */
export function projectAgentEditableBlocks(editor: Editor | null): AgentEditableBlockSnapshot[] {
  if (!editor) return [];
  const blocks: AgentEditableBlockSnapshot[] = [];
  editor.state.doc.descendants((node, pos, _parent, ordinal) => {
    const blockId = node.attrs?.blockId;
    if (typeof blockId !== "string" || !blockId) return true;
    const parent = nearestParentBlock(editor.state.doc, pos);
    const role = node.isAtom ? "atom" : node.isTextblock ? "content" : "container";
    const writable = role === "content"
      && !hasProtectedAncestor(editor.state.doc, pos)
      && !containsProtectedDescendant(node);
    const nodeJson = node.toJSON() as Record<string, unknown>;
    blocks.push({
      blockId,
      blockIndex: blocks.length,
      blockType: semanticBlockType(node),
      nodeType: node.type.name,
      role,
      parentBlockId: parent.blockId,
      depth: parent.depth,
      ordinal,
      text: node.textBetween(0, node.content.size, "\n", "￼"),
      attrs: { ...node.attrs },
      marks: projectMarks(node),
      nodeJson,
      fingerprint: fingerprintBlockNode(node),
      capabilities: writable ? [BLOCK_OP_CAPABILITY, "style_ops"] : [],
    });
    return true;
  });
  return blocks;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function parseStructuredBlockOperation(value: unknown): StructuredBlockOperation | null {
  if (!isRecord(value)) return null;
  const op = value.op;
  if (op !== "replace_node" && op !== "insert_before" && op !== "insert_after" && op !== "append") return null;

  if (op === "replace_node") {
    if (
      typeof value.blockId !== "string"
      || !value.blockId
      || typeof value.fingerprint !== "string"
      || !value.fingerprint
      || typeof value.expectedNodeType !== "string"
      || !value.expectedNodeType
      || !isRecord(value.beforeNode)
      || !isRecord(value.afterNode)
    ) {
      return null;
    }
    return {
      op: "replace_node",
      blockId: value.blockId,
      fingerprint: value.fingerprint,
      expectedNodeType: value.expectedNodeType,
      beforeNode: value.beforeNode,
      afterNode: value.afterNode,
    };
  }

  // insert_before / insert_after / append: afterNode required, blockId optional
  if (!isRecord(value.afterNode)) return null;
  return {
    op: op as StructuredBlockOperation["op"],
    blockId: typeof value.blockId === "string" ? value.blockId : "",
    fingerprint: typeof value.fingerprint === "string" ? value.fingerprint : "",
    expectedNodeType: typeof value.expectedNodeType === "string" ? value.expectedNodeType : "",
    beforeNode: isRecord(value.beforeNode) ? value.beforeNode : {},
    afterNode: value.afterNode,
  };
}

/** 解析后端已校验并补齐 beforeNode 的 BlockOperation Diff 投影。 */
export function parseStructuredBlockEditPayload(value: unknown): StructuredBlockEditPayload | undefined {
  if (!isRecord(value) || value.payloadVersion !== 1 || !isRecord(value.base) || !Array.isArray(value.operations)) {
    return undefined;
  }
  const sessionVersion = value.base.sessionVersion;
  const contentHash = value.base.contentHash;
  if (!Number.isInteger(sessionVersion) || (sessionVersion as number) < 0 || typeof contentHash !== "string" || !contentHash) {
    return undefined;
  }
  const operations = value.operations.map(parseStructuredBlockOperation);
  if (!operations.length || operations.some((operation) => operation === null)) return undefined;
  return {
    payloadVersion: 1,
    base: { sessionVersion: sessionVersion as number, contentHash },
    operations: operations as StructuredBlockOperation[],
  };
}

function findBlock(doc: PMNode, blockId: string): { node: PMNode; pos: number } | null {
  let found: { node: PMNode; pos: number } | null = null;
  doc.descendants((node, pos) => {
    if (found) return false;
    if (node.attrs?.blockId === blockId) {
      found = { node, pos };
      return false;
    }
    return true;
  });
  return found;
}

function sameAttrs(left: PMNode, right: PMNode): boolean {
  return stableJson(left.attrs) === stableJson(right.attrs);
}

export type StructuredBlockApplyResult =
  | { ok: true; affectedBlockIds: string[] }
  | { ok: false; reason: string };

interface PreparedReplacement {
  operation: StructuredBlockOperation;
  pos: number;
  current: PMNode;
  replacement: PMNode;
  /** 插入操作不使用 replaceWith，而是 insert */
  insertOnly?: boolean;
}

function prepareStructuredBlockOperations(
  editor: Editor,
  payload: StructuredBlockEditPayload,
  direction: "forward" | "reverse",
  currentBase?: { sessionVersion: number; contentHash: string },
): PreparedReplacement[] | { reason: string } {
  if (
    direction === "forward"
    && (
      !currentBase
      || currentBase.sessionVersion !== payload.base.sessionVersion
      || currentBase.contentHash !== payload.base.contentHash
    )
  ) {
    return { reason: "ED_BLOCK_OPERATION_BASE_STALE" };
  }
  const unique = new Set<string>();
  const prepared: PreparedReplacement[] = [];
  for (const operation of payload.operations) {
    // insert/append 操作：不需要 fingerprint / expectedNodeType 校验
    if (operation.op === "insert_before" || operation.op === "insert_after") {
      if (!operation.blockId) return { reason: "ED_BLOCK_OPERATION_ANCHOR_LOST" };
      const anchor = findBlock(editor.state.doc, operation.blockId);
      if (!anchor) return { reason: "ED_BLOCK_OPERATION_ANCHOR_LOST" };
      const afterNode = direction === "forward" ? operation.afterNode : operation.beforeNode;
      let replacement: PMNode;
      try {
        replacement = editor.schema.nodeFromJSON(afterNode);
      } catch {
        return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
      }
      if (!replacement.isTextblock || containsProtectedDescendant(replacement)) {
        return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
      }
      const insertPos = operation.op === "insert_before"
        ? anchor.pos
        : anchor.pos + anchor.node.nodeSize;
      prepared.push({ operation, pos: insertPos, current: anchor.node, replacement, insertOnly: true });
      continue;
    }

    if (operation.op === "append") {
      const afterNode = direction === "forward" ? operation.afterNode : operation.beforeNode;
      let replacement: PMNode;
      try {
        replacement = editor.schema.nodeFromJSON(afterNode);
      } catch {
        return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
      }
      if (!replacement.isTextblock || containsProtectedDescendant(replacement)) {
        return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
      }
      const insertPos = editor.state.doc.content.size;
      const docNode = editor.state.doc;
      prepared.push({ operation, pos: insertPos, current: docNode, replacement, insertOnly: true });
      continue;
    }
    if (!unique.add(operation.blockId)) return { reason: "ED_BLOCK_OPERATION_DUPLICATE_TARGET" };
    const found = findBlock(editor.state.doc, operation.blockId);
    if (!found) return { reason: "ED_BLOCK_OPERATION_ANCHOR_LOST" };
    if (
      !found.node.isTextblock
      || hasProtectedAncestor(editor.state.doc, found.pos)
      || containsProtectedDescendant(found.node)
    ) {
      return { reason: "ED_BLOCK_OPERATION_TARGET_READONLY" };
    }
    if (found.node.type.name !== operation.expectedNodeType) {
      return { reason: "ED_BLOCK_OPERATION_NODE_TYPE_CHANGED" };
    }
    const expectedFingerprint = direction === "forward"
      ? operation.fingerprint
      : fingerprintBlockNode(operation.afterNode);
    if (fingerprintBlockNode(found.node) !== expectedFingerprint) {
      return { reason: "ED_BLOCK_OPERATION_FINGERPRINT_CHANGED" };
    }
    const replacementJson = direction === "forward" ? operation.afterNode : operation.beforeNode;
    let replacement: PMNode;
    try {
      replacement = editor.schema.nodeFromJSON(replacementJson);
    } catch {
      return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
    }
    if (
      !replacement.isTextblock
      || replacement.type.name !== operation.expectedNodeType
      || replacement.attrs?.blockId !== operation.blockId
      || !sameAttrs(found.node, replacement)
      || containsProtectedDescendant(replacement)
    ) {
      return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
    }
    prepared.push({ operation, pos: found.pos, current: found.node, replacement });
  }
  return prepared;
}

/** 只读预检；拒绝前用于在后端进入 rejected 终态之前报告结构冲突。 */
export function validateStructuredBlockOperations(
  editor: Editor,
  payload: StructuredBlockEditPayload,
  direction: "forward" | "reverse",
  currentBase?: { sessionVersion: number; contentHash: string },
): StructuredBlockApplyResult {
  const prepared = prepareStructuredBlockOperations(editor, payload, direction, currentBase);
  if (!Array.isArray(prepared)) return { ok: false, reason: prepared.reason };
  return { ok: true, affectedBlockIds: prepared.map((item) => item.operation.blockId) };
}

/**
 * 原子应用 BlockOperation：先完成整批锚、类型、指纹、attrs 和 schema 校验，再按位置倒序写入同一
 * ProseMirror transaction；任一项失败时不 dispatch，LogicalState 保持不变。
 */
export function applyStructuredBlockOperations(
  editor: Editor,
  payload: StructuredBlockEditPayload,
  direction: "forward",
  currentBase: { sessionVersion: number; contentHash: string },
): StructuredBlockApplyResult;
export function applyStructuredBlockOperations(
  editor: Editor,
  payload: StructuredBlockEditPayload,
  direction: "reverse",
  currentBase?: undefined,
): StructuredBlockApplyResult;
export function applyStructuredBlockOperations(
  editor: Editor,
  payload: StructuredBlockEditPayload,
  direction: "forward" | "reverse",
  currentBase?: { sessionVersion: number; contentHash: string },
): StructuredBlockApplyResult {
  const prepared = prepareStructuredBlockOperations(editor, payload, direction, currentBase);
  if (!Array.isArray(prepared)) return { ok: false, reason: prepared.reason };
  const transaction = editor.state.tr;
  for (const item of [...prepared].sort((left, right) => right.pos - left.pos)) {
    if (item.insertOnly) {
      transaction.insert(item.pos, item.replacement);
    } else {
      transaction.replaceWith(item.pos, item.pos + item.current.nodeSize, item.replacement);
    }
  }
  editor.view.dispatch(transaction);
  return { ok: true, affectedBlockIds: prepared.map((item) => item.operation.blockId) };
}
