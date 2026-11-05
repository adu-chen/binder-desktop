import type { Editor } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import type {
  AgentEditableBlockSnapshot,
  StructuredBlockEditPayload,
  StructuredBlockOperation,
} from "../store/edStore";

const BLOCK_OP_CAPABILITY = "replace_node";
// AG-TASK-01 F2：删除能力位（与 replace_node 同条件——可写 content 块可删）。
const BLOCK_DELETE_CAPABILITY = "delete_node";
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
      capabilities: writable
        ? [BLOCK_OP_CAPABILITY, "style_ops", BLOCK_DELETE_CAPABILITY]
        : [],
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
  if (
    op !== "replace_node"
    && op !== "insert_before"
    && op !== "insert_after"
    && op !== "append"
    && op !== "delete_node"
  ) {
    return null;
  }

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

  // AG-TASK-01 F2：delete_node——afterNode 为 null（删除标记），只需 blockId + beforeNode。
  if (op === "delete_node") {
    if (
      typeof value.blockId !== "string"
      || !value.blockId
      || !isRecord(value.beforeNode)
      || value.afterNode !== null
    ) {
      return null;
    }
    return {
      op: "delete_node",
      blockId: value.blockId,
      fingerprint: typeof value.fingerprint === "string" ? value.fingerprint : "",
      expectedNodeType: typeof value.expectedNodeType === "string" ? value.expectedNodeType : "",
      beforeNode: value.beforeNode,
      afterNode: null,
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

function findBlocks(doc: PMNode, blockId: string): Array<{ node: PMNode; pos: number }> {
  const found: Array<{ node: PMNode; pos: number }> = [];
  doc.descendants((node, pos) => {
    if (node.attrs?.blockId === blockId) {
      found.push({ node, pos });
    }
    return true;
  });
  return found;
}

function findBlock(doc: PMNode, blockId: string): { node: PMNode; pos: number } | null {
  return findBlocks(doc, blockId)[0] ?? null;
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
  /** 原 proposal 中的稳定顺序；同坐标插入倒序执行才能保留提案中的阅读顺序。 */
  index: number;
  /** 插入操作不使用 replaceWith，而是 insert */
  insertOnly?: boolean;
  /** reverse 的 insert/append：按 afterNode 的稳定 blockId 删除实际插入节点。 */
  reverseInsertedNodeDelete?: boolean;
  /** AG-TASK-01 F2：删除操作——pos 处整块移除 */
  deleteOnly?: boolean;
  /** AG-TASK-01 F2：reverse 重建时插入（按记录的删除位置） */
  restoreOnly?: boolean;
}

// AG-TASK-01 F2：forward 应用删除块时记录"删除位置"，供同一编辑器 reverse（拒绝回滚）按原位置重建。
const deleteRestorePositions = new WeakMap<Editor, Map<string, number>>();

function rememberDeletePosition(editor: Editor, blockId: string, pos: number): void {
  let map = deleteRestorePositions.get(editor);
  if (!map) {
    map = new Map();
    deleteRestorePositions.set(editor, map);
  }
  map.set(blockId, pos);
}

function insertedNodeId(operation: StructuredBlockOperation): string | null {
  const attrs = operation.afterNode && isRecord(operation.afterNode.attrs)
    ? operation.afterNode.attrs
    : null;
  const blockId = attrs?.blockId;
  return typeof blockId === "string" && blockId ? blockId : null;
}

function nodeMatchesJson(node: PMNode, expected: Record<string, unknown> | null): boolean {
  if (!expected) return false;
  // 后端只带业务显式 attrs（如 blockId）；编辑器 schema 会补齐默认 attrs（如 textAlign:null）。
  // 两侧都先按同一 schema 归一化，再比较，避免把 schema 默认值误判为结构不一致。
  try {
    const normalized = node.type.schema.nodeFromJSON(expected);
    return stableJson(node.toJSON()) === stableJson(normalized.toJSON());
  } catch {
    return false;
  }
}

function countAddressableBlocks(doc: PMNode): number {
  let count = 0;
  doc.descendants((node) => {
    if (typeof node.attrs?.blockId === "string" && node.attrs.blockId) count += 1;
    return true;
  });
  return count;
}

function validateForwardPostcondition(
  doc: PMNode,
  payload: StructuredBlockEditPayload,
  beforeBlockCount: number,
): string | null {
  const inserts = payload.operations.filter((operation) => (
    operation.op === "insert_before" || operation.op === "insert_after" || operation.op === "append"
  )).length;
  const hasDelete = payload.operations.some((operation) => operation.op === "delete_node");
  // 无删除时，任何额外的孤儿块都意味着事务没有得到 proposal 所描述的最终结构。
  if (!hasDelete && countAddressableBlocks(doc) !== beforeBlockCount + inserts) {
    return "ED_BLOCK_OPERATION_RESULT_MISMATCH";
  }
  for (const operation of payload.operations) {
    if (operation.op === "delete_node") {
      if (findBlocks(doc, operation.blockId).length !== 0) return "ED_BLOCK_OPERATION_RESULT_MISMATCH";
      continue;
    }
    if (operation.op === "insert_before" || operation.op === "insert_after" || operation.op === "append") {
      const blockId = insertedNodeId(operation);
      const matches = blockId ? findBlocks(doc, blockId) : [];
      if (matches.length !== 1 || !nodeMatchesJson(matches[0].node, operation.afterNode)) {
        return "ED_BLOCK_OPERATION_RESULT_MISMATCH";
      }
      continue;
    }
    const matches = findBlocks(doc, operation.blockId);
    if (matches.length !== 1 || !nodeMatchesJson(matches[0].node, operation.afterNode)) {
      return "ED_BLOCK_OPERATION_RESULT_MISMATCH";
    }
  }
  return null;
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
  for (const [index, operation] of payload.operations.entries()) {
    // forward 以 base 的锚点执行；reverse 则必须定位实际插入的新块，而不是解析空 beforeNode。
    if (operation.op === "insert_before" || operation.op === "insert_after") {
      if (direction === "reverse") {
        const insertedId = insertedNodeId(operation);
        const matches = insertedId ? findBlocks(editor.state.doc, insertedId) : [];
        if (matches.length !== 1 || !nodeMatchesJson(matches[0].node, operation.afterNode)) {
          return { reason: "ED_BLOCK_OPERATION_FINGERPRINT_CHANGED" };
        }
        prepared.push({
          operation,
          pos: matches[0].pos,
          current: matches[0].node,
          replacement: matches[0].node,
          index,
          reverseInsertedNodeDelete: true,
        });
        continue;
      }
      if (!operation.blockId) return { reason: "ED_BLOCK_OPERATION_ANCHOR_LOST" };
      const anchor = findBlock(editor.state.doc, operation.blockId);
      if (!anchor) return { reason: "ED_BLOCK_OPERATION_ANCHOR_LOST" };
      let replacement: PMNode;
      try {
        replacement = editor.schema.nodeFromJSON(operation.afterNode);
      } catch {
        return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
      }
      if (!replacement.isTextblock || containsProtectedDescendant(replacement)) {
        return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
      }
      const insertPos = operation.op === "insert_before"
        ? anchor.pos
        : anchor.pos + anchor.node.nodeSize;
      prepared.push({ operation, pos: insertPos, current: anchor.node, replacement, index, insertOnly: true });
      continue;
    }

    if (operation.op === "append") {
      if (direction === "reverse") {
        const insertedId = insertedNodeId(operation);
        const matches = insertedId ? findBlocks(editor.state.doc, insertedId) : [];
        if (matches.length !== 1 || !nodeMatchesJson(matches[0].node, operation.afterNode)) {
          return { reason: "ED_BLOCK_OPERATION_FINGERPRINT_CHANGED" };
        }
        prepared.push({
          operation,
          pos: matches[0].pos,
          current: matches[0].node,
          replacement: matches[0].node,
          index,
          reverseInsertedNodeDelete: true,
        });
        continue;
      }
      let replacement: PMNode;
      try {
        replacement = editor.schema.nodeFromJSON(operation.afterNode);
      } catch {
        return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
      }
      if (!replacement.isTextblock || containsProtectedDescendant(replacement)) {
        return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
      }
      const insertPos = editor.state.doc.content.size;
      const docNode = editor.state.doc;
      prepared.push({ operation, pos: insertPos, current: docNode, replacement, index, insertOnly: true });
      continue;
    }
    // AG-TASK-01 F2：delete_node reverse（拒绝回滚）——被删块已不存在，须在 findBlock 之前
    // 按记录的原位置重建（forward 应用时 rememberDeletePosition 记录）。
    if (operation.op === "delete_node" && direction === "reverse") {
      if (!operation.blockId) return { reason: "ED_BLOCK_OPERATION_ANCHOR_LOST" };
      if (!unique.add(operation.blockId)) return { reason: "ED_BLOCK_OPERATION_DUPLICATE_TARGET" };
      const restorePos = deleteRestorePositions.get(editor)?.get(operation.blockId);
      if (restorePos === undefined) {
        return { reason: "ED_BLOCK_OPERATION_ANCHOR_LOST" };
      }
      let replacement: PMNode;
      try {
        replacement = editor.schema.nodeFromJSON(operation.beforeNode);
      } catch {
        return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
      }
      if (!replacement.isTextblock || containsProtectedDescendant(replacement)) {
        return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
      }
      const docNode = editor.state.doc;
      prepared.push({
        operation,
        pos: restorePos,
        current: docNode,
        replacement,
        index,
        restoreOnly: true,
      });
      continue;
    }
    if (!unique.add(operation.blockId)) return { reason: "ED_BLOCK_OPERATION_DUPLICATE_TARGET" };
    const found = findBlock(editor.state.doc, operation.blockId);
    if (!found) return { reason: "ED_BLOCK_OPERATION_ANCHOR_LOST" };
    // AG-TASK-01 F2：delete_node forward——校验目标为可删 textblock。
    if (operation.op === "delete_node") {
      if (
        !found.node.isTextblock
        || hasProtectedAncestor(editor.state.doc, found.pos)
        || containsProtectedDescendant(found.node)
      ) {
        return { reason: "ED_BLOCK_OPERATION_TARGET_READONLY" };
      }
      if (found.node.type.name !== operation.expectedNodeType && operation.expectedNodeType) {
        return { reason: "ED_BLOCK_OPERATION_NODE_TYPE_CHANGED" };
      }
      if (fingerprintBlockNode(found.node) !== operation.fingerprint && operation.fingerprint) {
        return { reason: "ED_BLOCK_OPERATION_FINGERPRINT_CHANGED" };
      }
      let replacement: PMNode;
      try {
        // forward：deleteOnly 不使用 replacement（delete 空替换），构造空 textblock 占位即可。
        replacement = editor.schema.nodes.paragraph.create();
      } catch {
        return { reason: "ED_BLOCK_OPERATION_NODE_INVALID" };
      }
      prepared.push({ operation, pos: found.pos, current: found.node, replacement, index, deleteOnly: true });
      continue;
    }
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
      // 此处仅 replace_node 分支可达（delete_node 已提前 continue），afterNode 必为 Record。
      : fingerprintBlockNode(operation.afterNode as Record<string, unknown>);
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
    prepared.push({ operation, pos: found.pos, current: found.node, replacement, index });
  }
  return prepared;
}

function comparePreparedOperations(
  left: PreparedReplacement,
  right: PreparedReplacement,
  direction: "forward" | "reverse",
): number {
  if (left.pos !== right.pos) return right.pos - left.pos;
  if (direction === "forward") {
    // replace/delete 必须先占用旧锚点；随后 insert_before 才能稳定落在新节点之前。
    const leftPriority = left.insertOnly ? 1 : 0;
    const rightPriority = right.insertOnly ? 1 : 0;
    if (leftPriority !== rightPriority) return leftPriority - rightPriority;
    // 多个同坐标 insert 反序写入，最终文档才保留 proposal 的正序阅读顺序。
    return right.index - left.index;
  }
  // 同坐标回滚时先删除实际插入节点，再恢复被删节点，避免恢复内容被误删。
  const leftPriority = left.reverseInsertedNodeDelete ? 0 : left.restoreOnly ? 1 : 2;
  const rightPriority = right.reverseInsertedNodeDelete ? 0 : right.restoreOnly ? 1 : 2;
  if (leftPriority !== rightPriority) return leftPriority - rightPriority;
  return right.index - left.index;
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
  const beforeBlockCount = countAddressableBlocks(editor.state.doc);
  const transaction = editor.state.tr;
  // StarterKit 的 TrailingNode 会在末尾标题后自动补空段落；结构化 proposal 必须只产生 payload
  // 明示的节点，否则 accepted/reject 都无法做到精确往返。
  transaction.setMeta("skipTrailingNode", true);
  for (const item of [...prepared].sort((left, right) => comparePreparedOperations(left, right, direction))) {
    if (item.reverseInsertedNodeDelete) {
      transaction.delete(item.pos, item.pos + item.current.nodeSize);
    } else if (item.insertOnly) {
      transaction.insert(item.pos, item.replacement);
    } else if (item.deleteOnly) {
      // AG-TASK-01 F2：删除 = 空替换（当前块整体移除）；记录位置供 reverse 恢复。
      rememberDeletePosition(editor, item.operation.blockId, item.pos);
      transaction.delete(item.pos, item.pos + item.current.nodeSize);
    } else if (item.restoreOnly) {
      // AG-TASK-01 F2：reverse 重建被删块（按记录位置插入）。
      transaction.insert(item.pos, item.replacement);
    } else {
      transaction.replaceWith(item.pos, item.pos + item.current.nodeSize, item.replacement);
    }
  }
  if (direction === "forward") {
    const postconditionError = validateForwardPostcondition(transaction.doc, payload, beforeBlockCount);
    if (postconditionError) return { ok: false, reason: postconditionError };
  }
  editor.view.dispatch(transaction);
  return { ok: true, affectedBlockIds: prepared.map((item) => item.operation.blockId) };
}
