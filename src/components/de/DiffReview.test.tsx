/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { deStore, type DiffView } from "../../store/deStore";
import type { DocumentImageSnapshot } from "../../services/edImageOps";
import { DiffAllActionsBar } from "./DiffAllActionsBar";
import { DiffCard } from "./DiffCard";

// Migrated intact from src/components/de/DiffAllActionsBar.test.tsx.
{
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const diff = (id: string, state: DiffView["state"]): DiffView => ({
  diffId: id,
  blockId: "",
  title: id,
  before: "before",
  after: "after",
  state,
});

beforeEach(() => {
  deStore.reset();
});

afterEach(() => {
  cleanup();
  deStore.reset();
});

describe("DiffAllActionsBar", () => {
  // covers: BR-DE-STATE-001
  it("有待裁决修改时显示批量操作按钮", () => {
    deStore.addDiff(diff("d1", "pending"));

    render(<DiffAllActionsBar />);

    expect(screen.getByText("1 条待裁决")).toBeTruthy();
    expect(screen.getByRole("button", { name: "全部接受" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "全部拒绝" })).toBeTruthy();
  });

  // covers: BR-DE-STATE-001
  it("批量应用中显示进度栏", () => {
    deStore.setBatch("applying");

    render(<DiffAllActionsBar />);

    expect(screen.getByText(/倒序应用中/)).toBeTruthy();
  });

  // covers: BR-DE-STATE-001
  it("批量完成后不显示终态栏", () => {
    deStore.addDiff(diff("d1", "accepted"));
    deStore.setBatch("done");

    const { container } = render(<DiffAllActionsBar />);

    expect(container.firstChild).toBeNull();
    expect(screen.queryByText(/全部应用完成/)).toBeNull();
  });
});
}

// Migrated intact from src/components/de/DiffCard.test.tsx.
{
// DiffCard 经 deService→wsService 引入 invoke；渲染不触发调用，mock 以隔离。
const mocks = vi.hoisted(() => ({ revealBlock: vi.fn(() => "located") }));
vi.mock("../../services/edLocate", () => ({ revealBlock: mocks.revealBlock }));

afterEach(() => {
  cleanup();
  mocks.revealBlock.mockClear();
});

const base: DiffView = { diffId: "d1", blockId: "", relPath: "demo.md", title: "改写开头", before: "旧", after: "新", state: "pending" };
const renderCard = (p: Partial<DiffView>) => render(<DiffCard diff={{ ...base, ...p }} />);
const card = (p: Partial<DiffView>): DiffView => ({ ...base, ...p });

describe("DiffCard 样式变更摘要（PI-ED-11 · D-3 方案A）", () => {
  // covers: BR-DE-DATA-002
  it("styleOps 存在 → 默认仅显示摘要，展开后渲染样式标签", () => {
    renderCard({ styleOps: ["加粗 [2–4]", "颜色 #ff0000 [5–9]"] });
    expect(screen.getByText("样式 2 项")).toBeTruthy();
    expect(screen.queryByText("加粗 [2–4]")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "展开" }));
    expect(screen.getByText("加粗 [2–4]")).toBeTruthy();
    expect(screen.getByText("颜色 #ff0000 [5–9]")).toBeTruthy();
    cleanup();
    renderCard({});
    expect(screen.queryByText(/加粗/)).toBeNull();
  });
});

describe("DiffCard 结构化块摘要", () => {
  // covers: BR-DE-DATA-002
  it("按独立 blockId 展示每个节点的精确前后文本", () => {
    renderCard({
      patchKind: "block_ops",
      blockId: "p-1",
      blockOpsPayload: {
        payloadVersion: 1,
        base: { sessionVersion: 7, contentHash: "h7" },
        operations: [
          {
            op: "replace_node",
            blockId: "p-1",
            fingerprint: "fp-1",
            expectedNodeType: "paragraph",
            beforeNode: { type: "paragraph", attrs: { blockId: "p-1" }, content: [{ type: "text", text: "第一条" }] },
            afterNode: { type: "paragraph", attrs: { blockId: "p-1" }, content: [{ type: "text", text: "新第一条" }] },
          },
          {
            op: "replace_node",
            blockId: "p-2",
            fingerprint: "fp-2",
            expectedNodeType: "paragraph",
            beforeNode: { type: "paragraph", attrs: { blockId: "p-2" }, content: [{ type: "text", text: "第二条" }] },
            afterNode: { type: "paragraph", attrs: { blockId: "p-2" }, content: [{ type: "text", text: "新第二条" }] },
          },
        ],
      },
    });
    expect(screen.getByText("独立内容块 2 项")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "展开" }));
    expect(screen.getByText("p-1 · paragraph")).toBeTruthy();
    expect(screen.getByText("p-2 · paragraph")).toBeTruthy();
    expect(screen.getByText("第一条")).toBeTruthy();
    expect(screen.getByText("新第二条")).toBeTruthy();
  });
});

describe("DiffCard 表格结构摘要（BDOCX-TABLE-AGENT-EDIT-01）", () => {
  // covers: BR-DE-DATA-002
  it("tableOps 存在时显示结构操作数量，展开后显示逐项语义和前后网格尺寸", () => {
    renderCard({
      patchKind: "table_ops",
      tableOps: ["在第 3 行前插入一行", "设为首行表头"],
      tableBefore: { tableId: "t1", ordinal: 0, rows: 2, columns: 3, headerRow: false, columnWidths: [], moveDirections: [], fingerprint: "f", cells: [], capabilities: [] },
      tableAfter: { tableId: "t1", ordinal: 0, rows: 3, columns: 3, headerRow: true, columnWidths: [], moveDirections: [], fingerprint: "g", cells: [], capabilities: [] },
    });
    expect(screen.getByText("表格操作 2 项")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "展开" }));
    expect(screen.getByText("在第 3 行前插入一行")).toBeTruthy();
    expect(screen.getByText("2×3 → 3×3")).toBeTruthy();
  });

  it("表格单元格文本修改展开后展示准确坐标与前后内容", () => {
    renderCard({
      patchKind: "table_ops",
      tableOps: ["更新 R1C1 内容"],
      tableOpsPayload: { tableId: "t1", fingerprint: "f", operations: [{ op: "set_cell_text", at: { row: 0, column: 0 }, text: "新值" }] },
      tableBefore: {
        tableId: "t1", ordinal: 0, rows: 1, columns: 1, headerRow: false, columnWidths: [120], moveDirections: [], fingerprint: "f",
        cells: [{ row: 0, column: 0, rowspan: 1, colspan: 1, text: "旧值", backgroundColor: null, contentKind: "plain_text" }], capabilities: ["set_cell_text"],
      },
      tableAfter: {
        tableId: "t1", ordinal: 0, rows: 1, columns: 1, headerRow: false, columnWidths: [120], moveDirections: [], fingerprint: "g",
        cells: [{ row: 0, column: 0, rowspan: 1, colspan: 1, text: "新值", backgroundColor: null, contentKind: "plain_text" }], capabilities: ["set_cell_text"],
      },
    });
    fireEvent.click(screen.getByRole("button", { name: "展开" }));
    expect(screen.getByText("R1C1：旧值 → 新值")).toBeTruthy();
  });

  // covers: BR-ED-DATA-007
  // covers: BR-DE-DATA-002
  it("单元格底纹修改展开后显示准确坐标、前后色块和十六进制值", () => {
    const { container } = renderCard({
      patchKind: "table_ops",
      tableOps: ["设置 #FFE699 R1C1 底纹"],
      tableOpsPayload: {
        tableId: "t1",
        fingerprint: "f",
        operations: [{ op: "set_cell_background", at: { row: 0, column: 0 }, color: "#FFE699" }],
      },
      tableBefore: {
        tableId: "t1", ordinal: 0, rows: 1, columns: 1, headerRow: false, columnWidths: [120], moveDirections: [], fingerprint: "f",
        cells: [{ row: 0, column: 0, rowspan: 1, colspan: 1, text: "内容", backgroundColor: null, contentKind: "rich_content" }],
        capabilities: ["set_cell_background"],
      },
      tableAfter: {
        tableId: "t1", ordinal: 0, rows: 1, columns: 1, headerRow: false, columnWidths: [120], moveDirections: [], fingerprint: "g",
        cells: [{ row: 0, column: 0, rowspan: 1, colspan: 1, text: "内容", backgroundColor: "#FFE699", contentKind: "rich_content" }],
        capabilities: ["set_cell_background"],
      },
    });
    fireEvent.click(screen.getByRole("button", { name: "展开" }));
    expect(screen.getByText("R1C1")).toBeTruthy();
    expect(screen.getByText("无底纹")).toBeTruthy();
    expect(screen.getByText("#FFE699")).toBeTruthy();
    expect(screen.getByLabelText("新底纹 #FFE699")).toBeTruthy();
    expect(container.querySelector(".diffcard-table-shading-swatch[style*=\"background-color\"]")).not.toBeNull();
  });
});

describe("DiffCard 图片结构摘要（BDOCX-IMAGE-AGENT-EDIT-01）", () => {
  // covers: BR-DE-DATA-002, BR-ED-DATA-003
  it("逐项展示插入来源、锚、alt、尺寸、哈希和前后图片实例数", () => {
    renderCard({
      patchKind: "image_ops",
      imageOps: ["插入 images/cover.png · 锚点后"],
      imageOpsPayload: {
        base: { sessionVersion: 4, contentHash: "h4" },
        operations: [{
          op: "insert_image",
          anchorBlockId: "b1",
          position: "after",
          assetId: "asset_cover",
          sourcePath: "images/cover.png",
          sourceHash: "a".repeat(64),
          mediaRef: `media/binder-project-${"a".repeat(64)}.png`,
          alt: "封面",
          width: 320,
        }],
      },
      imageBefore: [],
      imageAfter: [{
        imageId: "image_0_abcd",
        fingerprint: "abcd",
        ordinal: 0,
        blockId: "b-new",
        imagePos: 2,
        standalone: true,
        alt: "封面",
        width: "320",
        height: null,
        mediaRef: `media/binder-project-${"a".repeat(64)}.png`,
        projectSourcePath: "images/cover.png",
        projectSourceHash: "a".repeat(64),
        alignment: "left",
        capabilities: ["delete_image", "resize_image", "move_image", "set_image_layout"],
        resizeBounds: { min: 40, max: 800 },
        moveDirections: [],
      }],
    });
    expect(screen.getByText("图片操作 1 项")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "展开" }));
    expect(screen.getByText("图片实例：0 → 1")).toBeTruthy();
    expect(screen.getByText("插入图片 · images/cover.png")).toBeTruthy();
    expect(screen.getByText("位置：Block b1 之后")).toBeTruthy();
    expect(screen.getByText("替代文本：封面")).toBeTruthy();
    expect(screen.getByText("宽度：320px")).toBeTruthy();
    expect(screen.getByText(`来源哈希：${"a".repeat(64)}`)).toBeTruthy();
    expect(screen.getByText("结果：已生成图片实例 image_0_abcd")).toBeTruthy();
  });

  // covers: BR-DE-DATA-002
  it("删除操作明确展示原图片状态且声明保留项目源文件", () => {
    const beforeImage: DocumentImageSnapshot = {
      imageId: "image_0_abcd",
      fingerprint: "abcd",
      ordinal: 0,
      blockId: "b-img",
      imagePos: 2,
      standalone: true,
      alt: "旧图",
      width: "240",
      height: null,
      mediaRef: "media/image1.png",
      projectSourcePath: "images/old.png",
      projectSourceHash: "b".repeat(64),
      alignment: "left" as const,
      capabilities: ["delete_image", "resize_image", "move_image", "set_image_layout"],
      resizeBounds: { min: 40, max: 800 },
      moveDirections: ["down"],
    };
    renderCard({
      patchKind: "image_ops",
      imageOps: ["删除图片 image_0_abcd"],
      imageOpsPayload: {
        base: { sessionVersion: 4, contentHash: "h4" },
        operations: [{ op: "delete_image", imageId: "image_0_abcd", fingerprint: "abcd" }],
      },
      imageBefore: [beforeImage],
      imageAfter: [],
    });
    fireEvent.click(screen.getByRole("button", { name: "展开" }));
    expect(screen.getByText("删除文档图片 · image_0_abcd")).toBeTruthy();
    expect(screen.getByText("原位置：Block b-img")).toBeTruthy();
    expect(screen.getByText("原项目来源：images/old.png")).toBeTruthy();
    expect(screen.getByText(`原来源哈希：${"b".repeat(64)}`)).toBeTruthy();
    expect(screen.getByText("项目源文件：保留，不执行删除")).toBeTruthy();
    expect(screen.getByText("结果：文档图片节点已移除")).toBeTruthy();
  });

  it("缩放、移动和段落对齐展示准确的前后值", () => {
    const snapshot = (
      imageId: string,
      fingerprint: string,
      ordinal: number,
      width: string,
      alignment: "left" | "center" | "right",
    ): DocumentImageSnapshot => ({
      imageId,
      fingerprint,
      ordinal,
      blockId: `b-${imageId}`,
      imagePos: ordinal + 1,
      standalone: true,
      alt: null,
      width,
      height: null,
      mediaRef: `media/${imageId}.png`,
      projectSourcePath: null,
      projectSourceHash: null,
      alignment,
      capabilities: ["delete_image", "resize_image", "move_image", "set_image_layout"],
      resizeBounds: { min: 40, max: 800 },
      moveDirections: ["up", "down"],
    });
    renderCard({
      patchKind: "image_ops",
      imageOps: ["缩放图片 image_resize", "向下移动图片 image_move", "图片 image_align 设为居中"],
      imageOpsPayload: {
        base: { sessionVersion: 4, contentHash: "h4" },
        operations: [
          { op: "resize_image", imageId: "image_resize", fingerprint: "resize-old", width: 240 },
          { op: "move_image", imageId: "image_move", fingerprint: "move-old", direction: "down" },
          { op: "set_image_layout", imageId: "image_align", fingerprint: "align-old", alignment: "center" },
        ],
      },
      imageBefore: [
        snapshot("image_resize", "resize-old", 0, "160", "left"),
        snapshot("image_move", "move-old", 1, "200", "left"),
        snapshot("image_align", "align-old", 2, "200", "left"),
      ],
      imageAfter: [
        snapshot("image_resize", "resize-new", 0, "240", "left"),
        snapshot("image_move", "move-new", 2, "200", "left"),
        snapshot("image_align", "align-new", 1, "200", "center"),
      ],
    });

    fireEvent.click(screen.getByRole("button", { name: "展开" }));
    expect(screen.getByText("缩放图片 · image_resize")).toBeTruthy();
    expect(screen.getByText("宽度：160 → 240")).toBeTruthy();
    expect(screen.getByText("移动图片 · image_move")).toBeTruthy();
    expect(screen.getByText("方向：向下")).toBeTruthy();
    expect(screen.getByText("文档流位置：第 2 张 → 第 3 张")).toBeTruthy();
    expect(screen.getByText("图片段落对齐 · image_align")).toBeTruthy();
    expect(screen.getByText("对齐：left → center")).toBeTruthy();
  });
});

describe("DiffCard 8 状态渲染（互斥）", () => {
  // covers: BR-DE-DATA-002
  it("默认紧凑态按标题、位置、红删、绿增、操作五行渲染", () => {
    const { container } = renderCard({ state: "pending" });
    expect(container.querySelector(".diffcard-row-title")).not.toBeNull();
    expect(container.querySelector(".diffcard-row-location")).not.toBeNull();
    expect(container.querySelector(".diffcard-row-before")).not.toBeNull();
    expect(container.querySelector(".diffcard-row-after")).not.toBeNull();
    expect(container.querySelector(".diffcard-row-actions")).not.toBeNull();
    expect(container.querySelector(".diffcard-detail")).toBeNull();
  });

  it("pending：PENDING badge + 接受/拒绝", () => {
    renderCard({ state: "pending" });
    expect(screen.getByText("PENDING")).toBeTruthy();
    expect((screen.getByRole("button", { name: /接受/ }) as HTMLButtonElement).disabled).toBe(false);
    expect(screen.getByRole("button", { name: /拒绝/ })).toBeTruthy();
  });

  it("conflict：接受禁用 + 显示冲突对象", () => {
    renderCard({ state: "conflict", conflictWith: "d5" });
    expect(screen.getByText("冲突")).toBeTruthy();
    expect((screen.getByRole("button", { name: /接受/ }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(/d5/)).toBeTruthy();
  });

  it("expired：原因引导文案 + 重新生成", () => {
    renderCard({ state: "expired", expiredReason: "external_file_changed" });
    expect(screen.getByText("已失效")).toBeTruthy();
    expect(screen.getByText(/文件在磁盘上被外部修改/)).toBeTruthy();
    expect(screen.getByRole("button", { name: /重新生成/ })).toBeTruthy();
  });

  it("未应用 error：保留错误码并提示重新生成，不把接受动作伪装成重试", () => {
    renderCard({ state: "error", errorCode: "WS_APPLY_REPLACE_FAILED" });
    expect(screen.getByText(/WS_APPLY_REPLACE_FAILED/)).toBeTruthy();
    expect(screen.getByText("需重新生成修改建议")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /重试/ })).toBeNull();
    expect(screen.getByRole("button", { name: "关闭" })).toBeTruthy();
  });

  it("已应用 error：保留持久化重试入口", () => {
    renderCard({ state: "error", errorCode: "DE_ACCEPT_FAILED", draftApplied: true });
    expect(screen.getByRole("button", { name: /重试/ })).toBeTruthy();
  });

  it("accepted：旧落盘路径显示时间轴号", () => {
    const { container } = renderCard({ state: "accepted", timelineSeq: 101 });
    expect(screen.getByText("ACCEPTED")).toBeTruthy();
    expect(screen.getByText(/时间轴 #101/)).toBeTruthy();
    expect(container.querySelector(".diffcard-accepted")).not.toBeNull();
  });

  it("accepted：编辑器草稿路径显示待保存或已保存", () => {
    const { rerender } = render(<DiffCard diff={card({ state: "accepted", draftApplied: true })} />);
    expect(screen.getByText(/已接受 · 待保存/)).toBeTruthy();
    rerender(<DiffCard diff={card({ state: "accepted", draftApplied: true, draftSaved: true })} />);
    expect(screen.getByText("已保存")).toBeTruthy();
  });

  // covers: BR-SYS-DATA-005, BR-SYS-STATE-005
  it("历史终态：明确区分已保存、异常退出失效与已接受但内容丢弃", () => {
    const { rerender } = render(<DiffCard diff={card({ state: "accepted", readonly: true, effectStatus: "saved" })} />);
    expect(screen.getByText("已接受并保存")).toBeTruthy();

    rerender(<DiffCard diff={card({ state: "expired", readonly: true, terminalReason: "abnormal_exit" })} />);
    expect(screen.getByText("因上次异常退出已失效")).toBeTruthy();

    rerender(<DiffCard diff={card({ state: "accepted", readonly: true, effectStatus: "discarded_on_abnormal_exit" })} />);
    expect(screen.getByText("已接受，但未保存内容已丢弃")).toBeTruthy();
  });

  it("validating/applying：过渡态显示处理中（按钮禁用）", () => {
    const { container } = renderCard({ state: "applying" });
    expect(screen.getByText("写入中")).toBeTruthy();
    expect(container.querySelector(".diffcard-progress")).not.toBeNull();
    expect((screen.getByRole("button", { name: /处理中/ }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("rejected：弱化终态 class", () => {
    const { container } = renderCard({ state: "rejected" });
    expect(container.querySelector(".diffcard-rejected")).not.toBeNull();
  });

  it("before/after 内容渲染", () => {
    renderCard({ state: "pending", before: "原文X", after: "新文Y" });
    expect(screen.getByText(/原文X/)).toBeTruthy();
    expect(screen.getByText(/新文Y/)).toBeTruthy();
  });

  it("长 before/after 默认短预览，可展开查看完整内容", () => {
    const long = `第一行\n${"长文本".repeat(90)}`;
    const { container } = renderCard({ state: "pending", before: long, after: "新文" });
    const beforeLine = container.querySelector(".diffcard-before");
    expect(beforeLine?.textContent).not.toContain(long);
    fireEvent.click(screen.getByRole("button", { name: "展开" }));
    expect(container.querySelector(".diffcard-detail")?.textContent).toContain(long);
    expect(beforeLine?.textContent).not.toContain(long);
  });

  it("有 blockId 时显示定位按钮并调用编辑器锚定位", () => {
    renderCard({ blockId: "b1", locationLabel: "Block 1" });
    expect(screen.getByText(/demo.md · Block 1/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "定位" }));
    expect(mocks.revealBlock).toHaveBeenCalledWith("b1");
  });

  it("定位失败显示锚失效提示，后续成功定位会清除提示", () => {
    mocks.revealBlock.mockReturnValueOnce("anchor_lost").mockReturnValueOnce("located");
    renderCard({ blockId: "b1" });
    fireEvent.click(screen.getByRole("button", { name: "定位" }));
    expect(screen.getByText(/目标块已不可定位/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "定位" }));
    expect(screen.queryByText(/目标块已不可定位/)).toBeNull();
  });

  it("blockId 变化时清除上一锚的定位失败提示", () => {
    mocks.revealBlock.mockReturnValueOnce("anchor_lost");
    const { rerender } = render(<DiffCard diff={card({ blockId: "b1" })} />);
    fireEvent.click(screen.getByRole("button", { name: "定位" }));
    expect(screen.getByText(/目标块已不可定位/)).toBeTruthy();
    rerender(<DiffCard diff={card({ blockId: "b2" })} />);
    expect(screen.queryByText(/目标块已不可定位/)).toBeNull();
  });
});

describe("DiffCard 右键（CTX-MENU-01）", () => {
  it("pending 态右键弹出接受/拒绝菜单项", () => {
    const { container } = renderCard({ state: "pending" });
    fireEvent.contextMenu(container.querySelector(".diffcard") as HTMLElement, { clientX: 5, clientY: 6 });
    expect(screen.getByRole("menuitem", { name: "接受" })).toBeTruthy();
    expect(screen.getByRole("menuitem", { name: "拒绝" })).toBeTruthy();
  });

  it("非 pending 态（accepted/rejected）右键不弹菜单", () => {
    for (const state of ["accepted", "rejected"] as const) {
      cleanup();
      const { container } = renderCard({ state });
      fireEvent.contextMenu(container.querySelector(".diffcard") as HTMLElement);
      expect(screen.queryByRole("menuitem", { name: "接受" })).toBeNull();
    }
  });

  it("readonly pending 不弹菜单", () => {
    const { container } = renderCard({ state: "pending", readonly: true });
    fireEvent.contextMenu(container.querySelector(".diffcard") as HTMLElement);
    expect(screen.queryByRole("menuitem", { name: "接受" })).toBeNull();
  });
});
}
