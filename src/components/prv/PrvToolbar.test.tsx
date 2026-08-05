/** @vitest-environment jsdom */
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { PrvToolbar, exportTargetsForPreview } from "./PrvToolbar";

afterEach(cleanup);

// covers: BR-PRV-DATA-001, BR-PRV-DATA-002
describe("PrvToolbar · 预览模式工具栏自适应（PRV-VIEW）", () => {
  const base = {
    status: "ready" as const,
    zoom: 1,
    onZoomChange: vi.fn(),
    onEnterEdit: vi.fn(),
    onExport: vi.fn(async () => ({ ok: true as const, message: "已导出：out.pdf" })),
    onPrint: vi.fn(),
  };

  it("导出能力矩阵：docx→PDF/MD；xlsx/pptx→PDF；csv/图片/音视频→无导出", () => {
    expect(exportTargetsForPreview("a.docx")).toEqual(["pdf", "md"]);
    expect(exportTargetsForPreview("a.xlsx")).toEqual(["pdf"]);
    expect(exportTargetsForPreview("a.pptx")).toEqual(["pdf"]);
    expect(exportTargetsForPreview("a.csv")).toEqual([]);
    expect(exportTargetsForPreview("a.png")).toEqual([]);
    expect(exportTargetsForPreview("a.mp4")).toEqual([]);
  });

  it("pdf（原生 docx）：创建草稿 + 导出(PDF/MD) + 打印；无缩放", () => {
    render(<PrvToolbar mode="pdf" path="note.docx" {...base} />);
    expect(screen.getByTitle("创建草稿进行编辑")).toBeTruthy();
    expect(screen.getByTitle("导出文档")).toBeTruthy();
    expect(screen.getByTitle("打印")).toBeTruthy();
    expect(screen.queryByTitle("缩小")).toBeNull();
    expect(screen.queryByTitle("放大")).toBeNull();
  });

  it("pdf（xlsx/pptx）：导出(PDF) + 打印；无创建草稿", () => {
    render(<PrvToolbar mode="pdf" path="sheet.xlsx" {...base} />);
    expect(screen.queryByTitle("创建草稿进行编辑")).toBeNull();
    expect(screen.getByTitle("导出文档")).toBeTruthy();
    expect(screen.getByTitle("打印")).toBeTruthy();
  });

  it("pdf 未就绪时打印禁用（BR-PRV-DATA-002 失败态不静默）", () => {
    render(<PrvToolbar mode="pdf" path="note.docx" {...base} status="loading" />);
    expect((screen.getByTitle("打印") as HTMLButtonElement).disabled).toBe(true);
  });

  it("table：导出(PDF) + 打印 + 缩放", () => {
    render(<PrvToolbar mode="table" path="data.xlsx" {...base} />);
    expect(screen.getByTitle("导出文档")).toBeTruthy();
    expect(screen.getByTitle("打印")).toBeTruthy();
    expect(screen.getByTitle("缩小")).toBeTruthy();
    expect(screen.getByTitle("放大")).toBeTruthy();
  });

  it("table（csv）：无导出但保留打印 + 缩放", () => {
    render(<PrvToolbar mode="table" path="data.csv" {...base} />);
    expect(screen.queryByTitle("导出文档")).toBeNull();
    expect(screen.getByTitle("打印")).toBeTruthy();
    expect(screen.getByTitle("缩小")).toBeTruthy();
  });

  it("image：仅缩放，无导出/打印/创建草稿", () => {
    render(<PrvToolbar mode="image" path="photo.png" {...base} />);
    expect(screen.getByTitle("缩小")).toBeTruthy();
    expect(screen.getByTitle("放大")).toBeTruthy();
    expect(screen.queryByTitle("导出文档")).toBeNull();
    expect(screen.queryByTitle("打印")).toBeNull();
    expect(screen.queryByTitle("创建草稿进行编辑")).toBeNull();
  });

  it("video/audio：无额外工具（播放由原生控件承担）", () => {
    const { unmount } = render(<PrvToolbar mode="video" path="clip.mp4" {...base} />);
    expect(screen.queryByTitle("导出文档")).toBeNull();
    expect(screen.queryByTitle("打印")).toBeNull();
    expect(screen.queryByTitle("缩小")).toBeNull();
    expect(screen.queryByTitle("创建草稿进行编辑")).toBeNull();
    unmount();
    render(<PrvToolbar mode="audio" path="song.mp3" {...base} />);
    expect(screen.queryByTitle("打印")).toBeNull();
  });

  it("缩放按钮步进并钳制（复用 zoom.ts 语义，纯视图态）", () => {
    render(<PrvToolbar mode="image" path="photo.png" {...base} />);
    fireEvent.click(screen.getByTitle("放大"));
    expect(base.onZoomChange).toHaveBeenCalledWith(1.1);
    fireEvent.click(screen.getByTitle("缩小"));
    expect(base.onZoomChange).toHaveBeenCalledWith(0.9);
  });

  it("导出菜单项按能力显示，导出回调带格式", async () => {
    render(<PrvToolbar mode="pdf" path="note.docx" {...base} />);
    fireEvent.click(screen.getByTitle("导出文档"));
    fireEvent.click(screen.getByTitle("导出为 PDF"));
    expect(base.onExport).toHaveBeenCalledWith("pdf");
  });

  it("不提供关闭工具（关闭由 TabStrip 标签 × 承担）", () => {
    render(<PrvToolbar mode="pdf" path="note.docx" {...base} />);
    expect(screen.queryByTitle("关闭")).toBeNull();
    expect(screen.queryByText("关闭")).toBeNull();
  });
});
