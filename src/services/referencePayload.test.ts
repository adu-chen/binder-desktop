import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  DRAG_SESSION_MIME,
  TEXT_MARKER,
  clearDragSessions,
  resolveReferenceDrag,
  startReferenceDrag,
} from "./referenceDrag";
import {
  LONG_TEXT_REF_THRESHOLD,
  REFERENCE_MIME,
  coerceReferencePayload,
  dataTransferHasAnyType,
  dataTransferHasType,
  fileNameFromFileUrl,
  makeReferenceToken,
  promptTextFromParts,
  readReferencePayloadData,
  referenceFromPlainTextPaste,
  referenceKey,
  referenceLabel,
  writeReferencePayloadData,
  type ReferencePayload,
} from "./referencePayload";

function dataTransfer(values: Record<string, string> = {}) {
  const types = Object.keys(values);
  return {
    types,
    getData(type: string) {
      return values[type] ?? "";
    },
    setData(type: string, value: string) {
      values[type] = value;
      if (!types.includes(type)) types.push(type);
    },
  };
}

describe("referencePayload", () => {
  // covers: BR-REF-DATA-003
  // ReferencePayload 八类输入来源必须归一并可校验。
  it("validates all supported payload kinds", () => {
    const payloads: ReferencePayload[] = [
      { kind: "doc", relPath: "notes/a.md" },
      { kind: "selection", relPath: "notes/a.md", blockId: "block-1", start: 1, end: 4, text: "abcd" },
      { kind: "session", sessionId: "s1", title: "会话", messages: [{ role: "user", content: "hello" }] },
      { kind: "memory", entryId: "m1", content: "remember this" },
      { kind: "knowledge", entryId: "k1", content: "fact", sourceRef: "src" },
      { kind: "template", templateId: "t1", name: "周报", content: "template body" },
      { kind: "url", url: "https://example.com/a" },
      { kind: "long_text", text: "x".repeat(600), charCount: 600 },
    ];

    for (const payload of payloads) {
      expect(coerceReferencePayload(payload)).toEqual(payload);
      expect(referenceKey(payload)).toContain(payload.kind);
    }
  });

  // covers: BR-REF-DATA-003
  // DataTransfer 使用统一 MIME 在拖拽/粘贴边界传递 ReferencePayload。
  it("round-trips payloads through DataTransfer MIME", () => {
    const dt = dataTransfer();
    const payload: ReferencePayload = { kind: "doc", relPath: "docs/a.md" };

    writeReferencePayloadData(dt, payload);

    expect(dt.getData(REFERENCE_MIME)).toContain("docs/a.md");
    expect(readReferencePayloadData(dt)).toEqual(payload);
  });

  // covers: BR-REF-DATA-003
  // WebView 可能以 DOMStringList/array-like 暴露 DataTransfer.types，识别逻辑不得只依赖数组 includes。
  it("detects transfer MIME types across array and DOMStringList-like shapes", () => {
    expect(dataTransferHasType(dataTransfer({ [REFERENCE_MIME]: "x" }), REFERENCE_MIME)).toBe(true);
    expect(
      dataTransferHasType(
        {
          types: {
            contains: (value: string) => value === REFERENCE_MIME,
          },
        },
        REFERENCE_MIME,
      ),
    ).toBe(true);
    expect(
      dataTransferHasAnyType(
        {
          types: {
            length: 2,
            item: (index: number) => (index === 1 ? "text/uri-list" : "text/plain"),
          },
        },
        [REFERENCE_MIME, "text/uri-list"],
      ),
    ).toBe(true);
  });

  // covers: BR-REF-DATA-003
  // URL 与超过阈值的外部纯文本进入引用通道；普通短文本仍作为正文输入。
  it("classifies URL and long external paste without catching short text", () => {
    expect(referenceFromPlainTextPaste("https://example.com/page")).toEqual({
      kind: "url",
      url: "https://example.com/page",
    });
    expect(referenceFromPlainTextPaste("short plain text")).toBeNull();

    const long = "a".repeat(LONG_TEXT_REF_THRESHOLD + 1);
    expect(referenceFromPlainTextPaste(long)).toEqual({
      kind: "long_text",
      text: long,
      charCount: long.length,
    });
  });

  // covers: BR-REF-DATA-004
  // ReferenceToken 在发送序列化时保留输入流内标签位置。
  it("serializes reference tokens as inline prompt markers", () => {
    const token = makeReferenceToken({ kind: "doc", relPath: "notes/a.md" });

    expect(
      promptTextFromParts([
        { type: "text", text: "请总结 " },
        { type: "reference", ...token },
      ]),
    ).toBe("请总结 [引用:notes/a.md]");
  });

  // covers: BR-REF-DATA-003
  // file 类型（F5）：coerce/label/key 归一，MIME roundtrip；file:// 文本识别且普通文本不误判。
  it("normalizes file reference payloads and file:// plain text", () => {
    const file: ReferencePayload = { kind: "file", name: "report.pdf", size: 1024 };
    expect(coerceReferencePayload(file)).toEqual(file);
    expect(referenceLabel(file)).toBe("文件 report.pdf");
    expect(referenceKey(file)).toContain("file:report.pdf");

    const dt = dataTransfer();
    writeReferencePayloadData(dt, file);
    expect(readReferencePayloadData(dt)).toEqual(file);

    expect(referenceFromPlainTextPaste("file:///Users/a/b/report.pdf")).toEqual({
      kind: "file",
      name: "report.pdf",
    });
    expect(fileNameFromFileUrl("https://example.com/x.pdf")).toBeNull();
    expect(referenceFromPlainTextPaste("plain text")).toBeNull();
  });
});

describe("referenceDrag · drag session store", () => {
  const dragPayload: ReferencePayload = { kind: "doc", relPath: "notes/a.md" };

  beforeEach(() => {
    clearDragSessions();
  });

  afterEach(() => {
    clearDragSessions();
  });

  // covers: BR-REF-DATA-003
  // startReferenceDrag 写入三通道（REFERENCE_MIME / DRAG_SESSION_MIME / text/plain marker）。
  it("startReferenceDrag writes all three transfer channels", () => {
    const dt = dataTransfer();
    const id = startReferenceDrag(dt, dragPayload);

    expect(dt.getData(DRAG_SESSION_MIME)).toBe(id);
    expect(dt.getData(REFERENCE_MIME)).toContain("notes/a.md");
    expect(dt.getData("text/plain")).toBe(`${TEXT_MARKER}${id}`);
  });

  // covers: BR-REF-DATA-003
  // 三通道任一可用即可解析；custom MIME 全丢时 text/plain marker 保底。
  it("resolves via REFERENCE_MIME fast path", () => {
    const dt = dataTransfer();
    startReferenceDrag(dt, dragPayload);
    expect(resolveReferenceDrag(dt)).toEqual(dragPayload);
  });

  it("resolves via DRAG_SESSION_MIME when REFERENCE_MIME is cleared (PM clearData 场景)", () => {
    const dt = dataTransfer();
    const id = startReferenceDrag(dt, dragPayload);
    // 模拟 ProseMirror 内置 dragstart clearData() 后仅保留 session MIME。
    const fallback = dataTransfer({ [DRAG_SESSION_MIME]: id });
    expect(resolveReferenceDrag(fallback)).toEqual(dragPayload);
  });

  it("resolves via text/plain marker as last-resort (custom MIME invisible in WebKit)", () => {
    const dt = dataTransfer();
    const id = startReferenceDrag(dt, dragPayload);
    const fallback = dataTransfer({ "text/plain": `${TEXT_MARKER}${id}` });
    expect(resolveReferenceDrag(fallback)).toEqual(dragPayload);
  });

  // covers: BR-REF-DATA-003
  // drop 命中即消费清理（含快路径），防 session store 泄漏；TTL 兜底 dragend 未 drop 场景。
  it("consumes and clears the session entry after resolve via any channel", () => {
    const dt = dataTransfer();
    const id = startReferenceDrag(dt, dragPayload);
    expect(resolveReferenceDrag(dt)).toEqual(dragPayload); // 快路径命中
    const again = dataTransfer({ "text/plain": `${TEXT_MARKER}${id}` });
    expect(resolveReferenceDrag(again)).toBeNull(); // 消费后同一 sessionId 不再命中
  });

  // covers: BR-REF-DATA-003
  // 非引用拖拽（无任何引用通道）不误识别。
  it("returns null for unrelated drag payloads", () => {
    const dt = dataTransfer({ "text/plain": "hello" });
    expect(resolveReferenceDrag(dt)).toBeNull();
  });
});
