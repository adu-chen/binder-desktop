/**
 * @GOV
 * codes:    BR-REF-DATA-003-REF-REF-INJECT-005
 * type:     DATA
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-003
 * term_ref: TERM-REF-003
 * boundary: in=referenceSourceDragStart | out=dragSessionRegisteredAndTransferMarked(textPlainFallbackAcrossWebView)
 */
// 应用内统一拖拽通道（REF-REFERENCE-REPAIR-01 F1，解 REF-06）：
// custom application/x-* MIME 在真实 Tauri/WebKit drop 可能被清洗/不可见，故 dragstart 时写入
// 进程内 sessionStore，dataTransfer 以 text/plain marker 保底传递 sessionId；drop 侧按
// REFERENCE_MIME → DRAG_SESSION_MIME → text/plain marker 三级解析。不依赖 custom MIME 可见性。
// 编辑器选区源因 ProseMirror 内置 dragstart clearData()，需在 document 冒泡阶段后置回写（见 Editor.tsx）。

import {
  REFERENCE_MIME,
  readReferencePayloadData,
  type ReferencePayload,
} from "./referencePayload";

export const DRAG_SESSION_MIME = "application/x-binder-drag-session";
/** text/plain 保底标记：拖拽预览会短暂显示该前缀 + sessionId，接受该 UX 成本。 */
export const TEXT_MARKER = "binder-ref:";
/** session 保底过期时间（ms）：drop 未命中（拖拽取消/未进应用）时兜底清理。 */
const SESSION_TTL = 30_000;

export interface DragSessionEntry {
  payload: ReferencePayload;
  expiresAt: number;
}

/** 模块级拖拽会话存储：dragstart 写入，drop 命中或超时清理。测试可经 clearDragSessions 重置。 */
const sessionStore = new Map<string, DragSessionEntry>();

function dragSessionId(): string {
  return typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `drag-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function sweepExpired(now = Date.now()): void {
  for (const [id, entry] of sessionStore) {
    if (entry.expiresAt <= now) sessionStore.delete(id);
  }
}

/**
 * @GOV
 * codes:    BR-REF-DATA-003-REF-REF-INJECT-005
 * type:     GUARD
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-003
 * term_ref: TERM-REF-003
 * boundary: in=dragStartSourcePayloadAndDataTransfer | out=registeredDragSessionIdOrTransferUnmarked
 */
export function startReferenceDrag(
  dataTransfer: Pick<DataTransfer, "setData">,
  payload: ReferencePayload,
): string {
  sweepExpired();
  const id = dragSessionId();
  sessionStore.set(id, { payload, expiresAt: Date.now() + SESSION_TTL });
  dataTransfer.setData(DRAG_SESSION_MIME, id);
  dataTransfer.setData(REFERENCE_MIME, JSON.stringify(payload));
  dataTransfer.setData("text/plain", `${TEXT_MARKER}${id}`);
  return id;
}

/**
 * @GOV
 * codes:    BR-REF-DATA-003-REF-REF-INJECT-006
 * type:     GUARD
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-003
 * term_ref: TERM-REF-003
 * boundary: in=dataTransferAndDragSessionStore | out=referencePayloadOrNull(validatedExplicitReferenceSnapshot)
 */
export function resolveReferenceDrag(dataTransfer: Pick<DataTransfer, "getData">): ReferencePayload | null {
  sweepExpired();
  const sessionId = safeGetData(dataTransfer, DRAG_SESSION_MIME);
  const plain = safeGetData(dataTransfer, "text/plain");
  const markerId = plain?.startsWith(TEXT_MARKER) ? plain.slice(TEXT_MARKER.length).trim() : undefined;
  // 1) custom MIME 可见时的快路径（shape 校验防伪造；命中同时按 sessionId/marker 消费 session，防泄漏）。
  const generic = readReferencePayloadData(dataTransfer);
  if (generic) {
    if (sessionId) sessionStore.delete(sessionId);
    else if (markerId) sessionStore.delete(markerId);
    return generic;
  }
  // 2) drag session MIME。
  if (sessionId) {
    const hit = sessionStore.get(sessionId);
    if (hit) {
      sessionStore.delete(sessionId); // drop 命中即消费清理
      return hit.payload;
    }
  }
  // 3) text/plain marker 保底（WebKit 恒可见）。
  if (markerId) {
    const hit = sessionStore.get(markerId);
    if (hit) {
      sessionStore.delete(markerId); // drop 命中即消费清理
      return hit.payload;
    }
  }
  return null;
}

function safeGetData(dataTransfer: Pick<DataTransfer, "getData">, type: string): string | undefined {
  try {
    const value = dataTransfer.getData(type);
    return typeof value === "string" && value.trim() ? value : undefined;
  } catch {
    return undefined; // WebView 某些 MIME 读取抛错时保守降级。
  }
}

/** 拖拽结束（drop 命中 / dragend / dragcancel / 超时）后清理会话。 */
export function endReferenceDrag(id: string): void {
  if (id) sessionStore.delete(id);
}

/** 测试辅助：清空全部会话。 */
export function clearDragSessions(): void {
  sessionStore.clear();
}
