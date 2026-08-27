/**
 * @GOV
 * codes:    BR-REF-DATA-003-DATA-REF-REF-INJECT-006, BR-SYS-DATA-002-DATA-REF-REF-INJECT-006
 * type:     DATA
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-003, BR-SYS-DATA-002
 * term_ref: TERM-REF-003
 * boundary: in=referenceSourceDragOrPasteData | out=ReferencePayload(validatedExplicitReferenceSnapshot)
 */
// Unified input-side reference payloads. REF owns normalization; owner modules keep
// their asset search/management responsibilities.

export const REFERENCE_MIME = "application/x-binder-reference";
export const LONG_TEXT_REF_THRESHOLD = 500;

export type ReferenceKind =
  | "doc"
  | "selection"
  | "session"
  | "memory"
  | "knowledge"
  | "template"
  | "url"
  | "long_text"
  | "file";

export interface ReferenceSessionMessage {
  role: "user" | "assistant";
  content: string;
}

export type ReferencePayload =
  | { kind: "doc"; relPath: string; label?: string }
  | {
      kind: "selection";
      relPath: string;
      blockId: string;
      start?: number;
      end?: number;
      text: string;
      label?: string;
    }
  | {
      kind: "session";
      sessionId: string;
      title: string;
      messages: ReferenceSessionMessage[];
      label?: string;
    }
  | { kind: "memory"; entryId?: string; title?: string; content: string; label?: string }
  | { kind: "knowledge"; entryId?: string; title?: string; content: string; sourceRef?: string; label?: string }
  | { kind: "template"; templateId?: string; name: string; content: string; label?: string }
  | { kind: "url"; url: string; title?: string; label?: string }
  | { kind: "long_text"; text: string; charCount: number; label?: string }
  | {
      kind: "file";
      name: string;
      path?: string;
      size?: number;
      label?: string;
      /** ExternalContentRef（TERM-REF-005）：可读扩展名的外部文件内容，仅当前回合 data-only；缺省 = 未读取 */
      readContent?: string;
    };

export interface ReferenceTokenAttrs {
  refId: string;
  kind: ReferenceKind;
  label: string;
  payload: ReferencePayload;
}

export type MessageContentPart =
  | { type: "text"; text: string }
  | { type: "reference"; refId: string; kind: ReferenceKind; label: string; payload: ReferencePayload };

interface TransferLike {
  getData(type: string): string;
}

interface TransferTypesLike {
  types?: unknown;
}

interface WritableTransferLike extends TransferLike {
  setData(type: string, data: string): void;
}

const uid = () =>
  typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `ref-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const str = (value: unknown): string | undefined => (typeof value === "string" && value.trim() ? value : undefined);
const num = (value: unknown): number | undefined =>
  typeof value === "number" && Number.isFinite(value) ? value : undefined;

function base(path: string): string {
  return path.split("/").filter(Boolean).pop() ?? path;
}

function truncate(input: string, max = 28): string {
  const clean = input.replace(/\s+/g, " ").trim();
  return clean.length > max ? `${clean.slice(0, max - 1)}…` : clean;
}

function parseMessages(value: unknown): ReferenceSessionMessage[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((m) => {
      if (!isRecord(m)) return null;
      const role = m.role === "assistant" ? "assistant" : m.role === "user" ? "user" : null;
      const content = str(m.content);
      return role && content ? { role, content } : null;
    })
    .filter((m): m is ReferenceSessionMessage => Boolean(m));
}

/**
 * @GOV
 * codes:    BR-REF-DATA-003-GUARD-REF-REF-INJECT-006
 * type:     GUARD
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-003
 * term_ref: TERM-REF-003
 * boundary: in=dataTransferTypeList | out=referenceTransferTypeDetected(crossWebView)
 */
export function dataTransferHasType(dataTransfer: TransferTypesLike, type: string): boolean {
  const types = dataTransfer.types;
  if (!types) return false;
  if (Array.isArray(types)) return types.includes(type);
  if (typeof types !== "object") return false;

  const maybeList = types as {
    contains?: (value: string) => boolean;
    includes?: (value: string) => boolean;
    item?: (index: number) => unknown;
    length?: unknown;
    [Symbol.iterator]?: () => Iterator<unknown>;
  };

  try {
    if (typeof maybeList.contains === "function" && maybeList.contains(type)) return true;
  } catch {
    // Some WebView implementations expose host objects that may throw on feature probing.
  }

  try {
    if (typeof maybeList.includes === "function" && maybeList.includes(type)) return true;
  } catch {
    // Fall through to iterator/array-like probing.
  }

  try {
    if (typeof maybeList[Symbol.iterator] === "function") {
      for (const value of types as Iterable<unknown>) {
        if (value === type) return true;
      }
    }
  } catch {
    // Fall through to array-like probing.
  }

  const length = maybeList.length;
  if (typeof length !== "number" || !Number.isFinite(length)) return false;
  for (let index = 0; index < length; index += 1) {
    const value =
      typeof maybeList.item === "function"
        ? maybeList.item(index)
        : (types as Record<number, unknown>)[index];
    if (value === type) return true;
  }
  return false;
}

export function dataTransferHasAnyType(dataTransfer: TransferTypesLike, types: readonly string[]): boolean {
  return types.some((type) => dataTransferHasType(dataTransfer, type));
}

/**
 * @GOV
 * codes:    BR-REF-DATA-003-GUARD-REF-REF-INJECT-007
 * type:     GUARD
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-003
 * term_ref: TERM-REF-003
 * boundary: in=unknownReferencePayloadJson | out=ReferencePayloadOrNull(validatedShape)
 */
export function coerceReferencePayload(value: unknown): ReferencePayload | null {
  if (!isRecord(value) || typeof value.kind !== "string") return null;
  switch (value.kind) {
    case "doc": {
      const relPath = str(value.relPath);
      return relPath ? { kind: "doc", relPath, label: str(value.label) } : null;
    }
    case "selection": {
      const relPath = str(value.relPath);
      const blockId = str(value.blockId);
      const text = str(value.text);
      if (!relPath || !blockId || !text) return null;
      return {
        kind: "selection",
        relPath,
        blockId,
        text,
        ...(num(value.start) !== undefined ? { start: num(value.start) } : {}),
        ...(num(value.end) !== undefined ? { end: num(value.end) } : {}),
        ...(str(value.label) ? { label: str(value.label) } : {}),
      };
    }
    case "session": {
      const sessionId = str(value.sessionId);
      const title = str(value.title);
      if (!sessionId || !title) return null;
      return {
        kind: "session",
        sessionId,
        title,
        messages: parseMessages(value.messages),
        ...(str(value.label) ? { label: str(value.label) } : {}),
      };
    }
    case "memory": {
      const content = str(value.content);
      return content
        ? { kind: "memory", content, entryId: str(value.entryId), title: str(value.title), label: str(value.label) }
        : null;
    }
    case "knowledge": {
      const content = str(value.content);
      return content
        ? {
            kind: "knowledge",
            content,
            entryId: str(value.entryId),
            title: str(value.title),
            sourceRef: str(value.sourceRef),
            label: str(value.label),
          }
        : null;
    }
    case "template": {
      const name = str(value.name);
      const content = str(value.content);
      return name && content
        ? { kind: "template", name, content, templateId: str(value.templateId), label: str(value.label) }
        : null;
    }
    case "url": {
      const url = str(value.url);
      return url && isLikelyUrl(url) ? { kind: "url", url, title: str(value.title), label: str(value.label) } : null;
    }
    case "long_text": {
      const text = str(value.text);
      const charCount = num(value.charCount);
      return text
        ? {
            kind: "long_text",
            text,
            charCount: charCount ?? text.length,
            label: str(value.label),
          }
        : null;
    }
    case "file": {
      const name = str(value.name);
      return name
        ? {
            kind: "file",
            name,
            path: str(value.path),
            size: num(value.size),
            label: str(value.label),
            // ExternalContentRef（TERM-REF-005）：可读扩展名的外部文件内容（仅当前回合 data-only）。
            readContent: typeof value.readContent === "string" ? value.readContent : undefined,
          }
        : null;
    }
    default:
      return null;
  }
}

export function makeDocReference(relPath: string, label?: string): ReferencePayload {
  return { kind: "doc", relPath, ...(label ? { label } : {}) };
}

/**
 * MemorySelectionRef（TERM-AST-002 / BR-SYS-DATA-007）：用户显式选择的记忆条目 → data-only 引用快照，
 * 仅当前回合 payload（不写回 AST、跨会话不串）。title 缺省用 kind 语义兜底。
 */
export function makeMemoryReference(entryId: string, title: string, content: string): ReferencePayload {
  return { kind: "memory", entryId, title: title || "记忆条目", content };
}

/**
 * 引用注入总线（轻量模块级回调，无全局 store 槽）：侧栏面板（如记忆库）构造 ReferencePayload 后
 * 经 requestReferenceInsert 注入 Composer 输入区（Composer 挂载时 registerReferenceInsertHandler）。
 */
let referenceInsertHandler: ((payload: ReferencePayload) => void) | null = null;

export function registerReferenceInsertHandler(handler: (payload: ReferencePayload) => void): void {
  referenceInsertHandler = handler;
}

export function requestReferenceInsert(payload: ReferencePayload): void {
  referenceInsertHandler?.(payload);
}

export function makeSelectionReference(ref: {
  path: string;
  blockId: string;
  start: number;
  end: number;
  text: string;
}): ReferencePayload {
  const hasRange = ref.end > ref.start;
  return {
    kind: "selection",
    relPath: ref.path,
    blockId: ref.blockId,
    text: ref.text,
    ...(hasRange ? { start: ref.start, end: ref.end } : {}),
  };
}

export function makeReferenceToken(payload: ReferencePayload): ReferenceTokenAttrs {
  return {
    refId: uid(),
    kind: payload.kind,
    label: referenceLabel(payload),
    payload,
  };
}

export function referenceLabel(payload: ReferencePayload): string {
  if (payload.label) return payload.label;
  switch (payload.kind) {
    case "doc":
      return payload.relPath;
    case "selection": {
      const range =
        payload.start !== undefined && payload.end !== undefined ? `[${payload.start}-${payload.end}]` : "";
      return `选区 ${base(payload.relPath)}#${payload.blockId.slice(0, 6)}${range}`;
    }
    case "session":
      return `会话 ${truncate(payload.title, 22)}`;
    case "memory":
      return `记忆 ${truncate(payload.title || payload.content, 22)}`;
    case "knowledge":
      return `知识 ${truncate(payload.title || payload.sourceRef || payload.content, 22)}`;
    case "template":
      return `模板 ${truncate(payload.name, 22)}`;
    case "url":
      return payload.title ? `${payload.title} ${payload.url}` : payload.url;
    case "long_text":
      return `长文本 ${payload.charCount || payload.text.length}字`;
    case "file":
      return payload.path ? `文件 ${payload.name}（${payload.path}）` : `文件 ${payload.name}`;
  }
}

export function referenceKey(payload: ReferencePayload): string {
  switch (payload.kind) {
    case "doc":
      return `doc:${payload.relPath}`;
    case "selection":
      return `selection:${payload.relPath}:${payload.blockId}:${payload.start ?? ""}:${payload.end ?? ""}:${payload.text}`;
    case "session":
      return `session:${payload.sessionId}`;
    case "memory":
      return `memory:${payload.entryId ?? payload.content}`;
    case "knowledge":
      return `knowledge:${payload.entryId ?? payload.sourceRef ?? payload.content}`;
    case "template":
      return `template:${payload.templateId ?? payload.name}:${payload.content}`;
    case "url":
      return `url:${payload.url}`;
    case "long_text":
      return `long_text:${payload.text}`;
    case "file":
      return `file:${payload.name}:${payload.size ?? ""}`;
  }
}

export function writeReferencePayloadData(dataTransfer: WritableTransferLike, payload: ReferencePayload): void {
  dataTransfer.setData(REFERENCE_MIME, JSON.stringify(payload));
  dataTransfer.setData("text/plain", referenceLabel(payload));
}

export function readReferencePayloadData(dataTransfer: TransferLike): ReferencePayload | null {
  const raw = dataTransfer.getData(REFERENCE_MIME);
  if (!raw) return null;
  try {
    return coerceReferencePayload(JSON.parse(raw));
  } catch {
    return null;
  }
}

export function isLikelyUrl(text: string): boolean {
  const clean = text.trim();
  try {
    const u = new URL(clean);
    return u.protocol === "http:" || u.protocol === "https:";
  } catch {
    return false;
  }
}

/** 从 file:// URL 提取文件名（F5 外部文件拖入的 text/uri-list 兜底）；非 file:// 返回 null。 */
export function fileNameFromFileUrl(text: string): string | null {
  const clean = text.trim();
  try {
    const u = new URL(clean);
    if (u.protocol !== "file:") return null;
    const name = u.pathname.split("/").filter(Boolean).pop();
    return name ? decodeURIComponent(name) : null;
  } catch {
    return null;
  }
}

export function referenceFromPlainTextPaste(text: string): ReferencePayload | null {
  const clean = text.trim();
  if (!clean) return null;
  const fileName = fileNameFromFileUrl(clean);
  if (fileName) return { kind: "file", name: fileName };
  if (isLikelyUrl(clean)) return { kind: "url", url: clean };
  if (clean.length > LONG_TEXT_REF_THRESHOLD) {
    return { kind: "long_text", text: clean, charCount: clean.length };
  }
  return null;
}

/**
 * @GOV
 * codes:    BR-REF-DATA-004-RB-REF-REF-INJECT-001
 * type:     RB
 * chain:    REF-INJECT
 * rules:    BR-REF-DATA-004
 * term_ref: TERM-REF-004, TERM-REF-003
 * boundary: in=ReferenceTokenAttrsAndTextSegments | out=messageContentPartsAndPromptText
 */
export function promptTextFromParts(parts: MessageContentPart[]): string {
  return parts
    .map((part) => (part.type === "text" ? part.text : `[引用:${part.label}]`))
    .join("")
    .replace(/[ \t]+\n/g, "\n")
    .trim();
}
