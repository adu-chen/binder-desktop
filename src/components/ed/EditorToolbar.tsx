import { useEffect, useReducer, useState } from "react";
import type { Editor } from "@tiptap/react";
import { ZOOM, formatZoom } from "./zoom";
import type { PaperKind, PaperOrientation } from "../../store/edViewPrefsStore";
import type { EditFormat } from "../../store/edStore";
import { triggerActiveSave } from "../../services/edLocate";
import { TbBtn, Sep, ToolbarMenu } from "./edToolbarParts";

const PAPER_KINDS: PaperKind[] = ["A4", "A3", "A5", "Letter"];
const FONT_SIZES = [12, 14, 16, 18, 20, 24, 28, 36] as const;
const FONT_FAMILIES = ["Arial", "Helvetica", "Georgia", "Times New Roman", "Courier New"] as const;
const HIGHLIGHT_COLORS = [
  { label: "黄色", color: "#FDE68A" },
  { label: "绿色", color: "#BBF7D0" },
  { label: "蓝色", color: "#BAE6FD" },
  { label: "粉色", color: "#FBCFE8" },
  { label: "紫色", color: "#DDD6FE" },
] as const;
type ExportFormat = "pdf" | "docx" | "md";

interface TextMatch {
  from: number;
  to: number;
}

function sourceMatches(text: string, query: string): TextMatch[] {
  if (!query) return [];
  const matches: TextMatch[] = [];
  let index = text.indexOf(query);
  while (index !== -1) {
    matches.push({ from: index, to: index + query.length });
    index = text.indexOf(query, index + query.length);
  }
  return matches;
}

function documentMatches(editor: Editor, query: string): TextMatch[] {
  if (!query) return [];
  if (!editor.state?.doc) return [];
  const matches: TextMatch[] = [];
  editor.state.doc.descendants((node, pos) => {
    if (!node.isText || !node.text) return true;
    let index = node.text.indexOf(query);
    while (index !== -1) {
      matches.push({ from: pos + index, to: pos + index + query.length });
      index = node.text.indexOf(query, index + query.length);
    }
    return true;
  });
  return matches;
}

function sourceLineStart(text: string, line: number): number | null {
  if (!Number.isInteger(line) || line < 1) return null;
  let offset = 0;
  for (let index = 1; index < line; index++) {
    const next = text.indexOf("\n", offset);
    if (next === -1) return null;
    offset = next + 1;
  }
  return offset;
}

/**
 * 编辑器两行工具栏。第一行仅承载编辑器级操作；第二行按格式显示可持久化的文档操作。
 * 图片和表格操作由选中对象附近的临时工具组负责，不形成第三条常驻工具栏。
 */
export function EditorToolbar({
  editor,
  zoom,
  onZoomIn,
  onZoomOut,
  onZoomReset,
  paper,
  orientation,
  onPaperChange,
  onOrientationChange,
  localImageDropOnly = false,
  editFormat,
  sourceView,
  onToggleSourceView,
  sourceText,
  onSourceTextChange,
  onJumpToLine,
  exportTargets = [],
  onExport,
}: {
  editor: Editor | null;
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onZoomReset: () => void;
  paper?: PaperKind;
  orientation?: PaperOrientation;
  onPaperChange?: (paper: PaperKind) => void;
  onOrientationChange?: (orientation: PaperOrientation) => void;
  localImageDropOnly?: boolean;
  editFormat?: EditFormat;
  sourceView?: boolean;
  onToggleSourceView?: () => void;
  sourceText?: string;
  onSourceTextChange?: (text: string) => void;
  onJumpToLine?: (line: number) => void;
  exportTargets?: ExportFormat[];
  onExport?: (format: ExportFormat) => Promise<{ ok: boolean; message?: string }>;
}) {
  const [, tick] = useReducer((count: number) => count + 1, 0);
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [findQuery, setFindQuery] = useState("");
  const [replaceText, setReplaceText] = useState("");
  const [findMessage, setFindMessage] = useState("");
  const [jumpValue, setJumpValue] = useState("1");
  const [jumpMessage, setJumpMessage] = useState("");
  const [pageInfo, setPageInfo] = useState({ current: 1, total: 0 });
  const [exportState, setExportState] = useState<"idle" | "exporting" | "success" | "error">("idle");
  const [exportMessage, setExportMessage] = useState("");
  useEffect(() => {
    if (!editor) return;
    const refresh = () => {
      setSaveState("idle");
      tick();
    };
    editor.on("transaction", refresh);
    return () => {
      editor.off("transaction", refresh);
    };
  }, [editor]);

  useEffect(() => {
    if (!editor) return;
    const root = editor.view?.dom as HTMLElement | undefined;
    if (!root) return;
    let frame = 0;
    const syncPageInfo = () => {
      if (!root.classList.contains("rm-paginated")) {
        setPageInfo({ current: 1, total: 0 });
        return;
      }
      const gaps = Array.from(root.querySelectorAll<HTMLElement>(".rm-page-gap"));
      let current = 1;
      try {
        const caret = editor.view.coordsAtPos(editor.state.selection.head);
        for (let index = 0; index < gaps.length; index++) {
          if (caret.top >= gaps[index].getBoundingClientRect().top) current = index + 2;
        }
      } catch {
        // 页码导航可降级为首个页面，不能影响编辑器本身。
      }
      setPageInfo({ current: Math.min(current, gaps.length + 1), total: gaps.length + 1 });
    };
    const scheduleSync = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(syncPageInfo);
    };
    const observer = typeof MutationObserver === "undefined"
      ? null
      : new MutationObserver(scheduleSync);
    editor.on("transaction", scheduleSync);
    observer?.observe(root, { childList: true, subtree: true });
    scheduleSync();
    return () => {
      cancelAnimationFrame(frame);
      editor.off("transaction", scheduleSync);
      observer?.disconnect();
    };
  }, [editor]);

  if (!editor) return null;
  const chain = () => editor.chain().focus();
  const isMarkdown = editFormat === "markdown";
  const isPlainText = editFormat === "text";
  const isSourceView = isMarkdown && sourceView;
  const queryMatches = isSourceView
    ? sourceMatches(sourceText ?? "", findQuery)
    : documentMatches(editor, findQuery);
  const rawSize = (editor.getAttributes("textStyle").fontSize as string | undefined) ?? "";
  const currentSize = rawSize ? String(parseFloat(rawSize)) : "";
  const currentFontFamily = (editor.getAttributes("textStyle").fontFamily as string | undefined) ?? "";
  const activeHeading = [1, 2, 3, 4, 5, 6].find((level) => editor.isActive("heading", { level }));

  const setHeading = (value: string) => {
    if (value === "p") chain().setParagraph().run();
    else chain().toggleHeading({ level: Number(value.slice(1)) as never }).run();
  };
  const setFontSize = (value: string) => {
    if (!value) chain().unsetFontSize().run();
    else chain().setFontSize(value + "pt").run();
  };
  const promptLink = () => {
    const previous = (editor.getAttributes("link").href as string | undefined) ?? "";
    const href = window.prompt("链接地址（留空移除）", previous);
    if (href === null) return;
    if (href === "") chain().extendMarkRange("link").unsetLink().run();
    else chain().extendMarkRange("link").setLink({ href }).run();
  };
  const promptImage = () => {
    const src = window.prompt("图片地址（URL）");
    if (src) chain().setImage({ src }).run();
  };
  const saveCurrent = async () => {
    if (saveState === "saving") return;
    setSaveState("saving");
    try {
      const outcome = await triggerActiveSave();
      setSaveState(outcome.ok ? "saved" : "error");
    } catch {
      setSaveState("error");
    }
  };
  const sourceTextarea = () => editor.view?.dom.closest(".editor-host")?.querySelector<HTMLTextAreaElement>(".editor-source-view") ?? null;
  const selectSourceMatch = (match: TextMatch) => {
    const textarea = sourceTextarea();
    textarea?.focus();
    textarea?.setSelectionRange(match.from, match.to);
  };
  const selectDocumentMatch = (match: TextMatch) => {
    editor.chain().focus().setTextSelection(match).scrollIntoView().run();
  };
  const findNext = () => {
    if (!findQuery) {
      setFindMessage("请输入查找内容");
      return;
    }
    if (!queryMatches.length) {
      setFindMessage("未找到匹配内容");
      return;
    }
    const selectionEnd = isSourceView
      ? (sourceTextarea()?.selectionEnd ?? -1)
      : editor.state.selection.to;
    const match = queryMatches.find((item) => item.from >= selectionEnd) ?? queryMatches[0];
    if (isSourceView) selectSourceMatch(match);
    else selectDocumentMatch(match);
    setFindMessage(`${queryMatches.length} 处匹配`);
  };
  const replaceCurrent = () => {
    if (!findQuery) {
      setFindMessage("请输入查找内容");
      return;
    }
    if (!queryMatches.length) {
      setFindMessage("未找到匹配内容");
      return;
    }
    if (isSourceView) {
      const text = sourceText ?? "";
      const selection = sourceTextarea();
      const selected = selection ? text.slice(selection.selectionStart, selection.selectionEnd) : "";
      const match = selected === findQuery
        ? { from: selection!.selectionStart, to: selection!.selectionEnd }
        : (queryMatches.find((item) => item.from >= (selection?.selectionEnd ?? -1)) ?? queryMatches[0]);
      onSourceTextChange?.(text.slice(0, match.from) + replaceText + text.slice(match.to));
      setFindMessage("已替换 1 处");
      return;
    }
    const selected = editor.state.doc.textBetween(editor.state.selection.from, editor.state.selection.to, "\n");
    const match = selected === findQuery
      ? { from: editor.state.selection.from, to: editor.state.selection.to }
      : (queryMatches.find((item) => item.from >= editor.state.selection.to) ?? queryMatches[0]);
    editor.view.dispatch(editor.state.tr.insertText(replaceText, match.from, match.to));
    editor.commands.focus();
    setFindMessage("已替换 1 处");
  };
  const replaceAll = () => {
    if (!findQuery) {
      setFindMessage("请输入查找内容");
      return;
    }
    if (!queryMatches.length) {
      setFindMessage("未找到匹配内容");
      return;
    }
    if (isSourceView) {
      const text = sourceText ?? "";
      onSourceTextChange?.(text.split(findQuery).join(replaceText));
    } else {
      let transaction = editor.state.tr;
      for (const match of [...queryMatches].reverse()) {
        transaction = transaction.insertText(replaceText, match.from, match.to);
      }
      editor.view.dispatch(transaction);
      editor.commands.focus();
    }
    setFindMessage(`已替换 ${queryMatches.length} 处`);
  };
  const jumpToLocation = () => {
    const line = Number(jumpValue);
    if (!Number.isInteger(line) || line < 1) {
      setJumpMessage("请输入大于 0 的编号");
      return;
    }
    if (isSourceView) {
      const offset = sourceLineStart(sourceText ?? "", line);
      if (offset === null) {
        setJumpMessage("该行不存在");
        return;
      }
      selectSourceMatch({ from: offset, to: offset });
    } else {
      onJumpToLine?.(line);
    }
    setJumpMessage("");
  };
  const goToPage = (page: number) => {
    const root = editor.view?.dom as HTMLElement | undefined;
    if (!root || page < 1 || page > pageInfo.total) return;
    const target = page === 1
      ? root
      : root.querySelectorAll<HTMLElement>(".rm-page-gap")[page - 2];
    target?.scrollIntoView({ block: "start", behavior: "smooth" });
    setPageInfo((current) => ({ ...current, current: page }));
  };
  const exportCurrent = async (format: ExportFormat) => {
    if (!onExport || exportState === "exporting") return;
    setExportState("exporting");
    setExportMessage("正在保存并导出…");
    const outcome = await onExport(format);
    setExportState(outcome.ok ? "success" : "error");
    setExportMessage(outcome.message ?? (outcome.ok ? "导出完成" : "导出失败"));
  };

  return (
    <div className="ed-toolbar" role="toolbar" aria-label="编辑工具栏">
      <div className="ed-tb-row ed-tb-row-editor" aria-label="编辑器工具">
        <div className="ed-tb-section">
          <TbBtn title="保存 ⌘S" onClick={() => { void saveCurrent(); }} disabled={saveState === "saving"}>
            {saveState === "saving" ? "保存中" : "保存"}
          </TbBtn>
          <TbBtn title="撤销 ⌘Z" onClick={() => chain().undo().run()} disabled={!editor.can().undo()}>↶</TbBtn>
          <TbBtn title="重做 ⇧⌘Z" onClick={() => chain().redo().run()} disabled={!editor.can().redo()}>↷</TbBtn>
        </div>
        {saveState !== "idle" && (
          <span className={"ed-tb-save-state " + saveState} role="status">
            {saveState === "saved" ? "已保存" : saveState === "error" ? "保存失败" : "正在保存"}
          </span>
        )}
        <Sep />
        <ToolbarMenu label="查找" title="查找和替换">
          <label className="ed-tb-field ed-tb-search-field">
            <span>查找</span>
            <input aria-label="查找内容" value={findQuery} onChange={(event) => setFindQuery(event.target.value)} />
          </label>
          <label className="ed-tb-field ed-tb-search-field">
            <span>替换</span>
            <input aria-label="替换内容" value={replaceText} onChange={(event) => setReplaceText(event.target.value)} />
          </label>
          <TbBtn title="查找下一处" onClick={findNext}>下一处</TbBtn>
          <TbBtn title="替换当前匹配" onClick={replaceCurrent}>替换</TbBtn>
          <TbBtn title="替换全部匹配" onClick={replaceAll}>全部替换</TbBtn>
          {(findMessage || findQuery) && <span className="ed-tb-action-state" role="status">{findMessage || `${queryMatches.length} 处匹配`}</span>}
        </ToolbarMenu>
        <ToolbarMenu label={isSourceView || isPlainText ? "跳转行号" : "定位段落"} title={isSourceView || isPlainText ? "跳转行号" : "跳转段落"}>
          <label className="ed-tb-field">
            <span>{isSourceView || isPlainText ? "行号" : "段落"}</span>
            <input aria-label={isSourceView || isPlainText ? "跳转行号输入" : "跳转段落输入"} type="number" min="1" value={jumpValue} onChange={(event) => setJumpValue(event.target.value)} />
          </label>
          <TbBtn title={isSourceView || isPlainText ? "跳转到行号" : "跳转到段落"} onClick={jumpToLocation}>跳转</TbBtn>
          {jumpMessage && <span className="ed-tb-action-state" role="status">{jumpMessage}</span>}
        </ToolbarMenu>
        {pageInfo.total > 0 && (
          <ToolbarMenu label={`第 ${pageInfo.current}/${pageInfo.total} 页`} title="分页导航">
            <TbBtn title="上一页" disabled={pageInfo.current <= 1} onClick={() => goToPage(pageInfo.current - 1)}>上一页</TbBtn>
            <label className="ed-tb-field">
              <span>页码</span>
              <select aria-label="选择页码" value={pageInfo.current} onChange={(event) => goToPage(Number(event.target.value))}>
                {Array.from({ length: pageInfo.total }, (_, index) => <option key={index + 1} value={index + 1}>第 {index + 1} 页</option>)}
              </select>
            </label>
            <TbBtn title="下一页" disabled={pageInfo.current >= pageInfo.total} onClick={() => goToPage(pageInfo.current + 1)}>下一页</TbBtn>
          </ToolbarMenu>
        )}
        {exportTargets.length > 0 && onExport && (
          <ToolbarMenu label="导出" title="导出文档">
            {exportTargets.map((format) => <TbBtn key={format} title={`导出为 ${format.toUpperCase()}`} disabled={exportState === "exporting"} onClick={() => { void exportCurrent(format); }}>导出 {format.toUpperCase()}</TbBtn>)}
            {exportMessage && <span className={`ed-tb-action-state ${exportState}`} role="status">{exportMessage}</span>}
          </ToolbarMenu>
        )}
        {isMarkdown && onToggleSourceView && (
          <>
            <TbBtn title={sourceView ? "切换到富文本视图" : "切换到 Markdown 源码视图"} active={sourceView} onClick={onToggleSourceView}>
              {sourceView ? "富文本" : "源码"}
            </TbBtn>
            <Sep />
          </>
        )}
        <div className="ed-tb-section ed-tb-view-section">
          <TbBtn title="缩小" onClick={onZoomOut} disabled={zoom <= ZOOM.min}>−</TbBtn>
          <button
            type="button"
            className="ed-tb-zoom"
            title="重置缩放（100%）"
            aria-label={"当前缩放 " + formatZoom(zoom) + "，点击重置为 100%"}
            onMouseDown={(event) => event.preventDefault()}
            onClick={onZoomReset}
          >
            {formatZoom(zoom)}
          </button>
          <TbBtn title="放大" onClick={onZoomIn} disabled={zoom >= ZOOM.max}>+</TbBtn>
        </div>
      </div>

      <div className="ed-tb-row ed-tb-row-document" aria-label="文档工具">
        {isPlainText ? (
          <span className="ed-tb-mode-note" role="status">纯文本模式：仅保留文本内容与编辑器工具</span>
        ) : isSourceView ? (
          <span className="ed-tb-mode-note" role="status">Markdown 源码模式：格式操作请使用 Markdown 语法</span>
        ) : (
          <>
            <ToolbarMenu label="样式" title="段落和标题样式">
              <label className="ed-tb-field">
                <span>块类型</span>
                <select value={activeHeading ? "h" + activeHeading : "p"} onChange={(event) => setHeading(event.target.value)}>
                  <option value="p">正文</option>
                  {[1, 2, 3, 4, 5, 6].map((level) => <option key={level} value={"h" + level}>标题 {level}</option>)}
                </select>
              </label>
            </ToolbarMenu>

            <ToolbarMenu label="文字" title="文字格式">
              <TbBtn title="加粗 ⌘B" active={editor.isActive("bold")} onClick={() => chain().toggleBold().run()}><b>B</b></TbBtn>
              <TbBtn title="斜体 ⌘I" active={editor.isActive("italic")} onClick={() => chain().toggleItalic().run()}><i>I</i></TbBtn>
              <TbBtn title="删除线" active={editor.isActive("strike")} onClick={() => chain().toggleStrike().run()}><s>S</s></TbBtn>
              {isMarkdown ? (
                <>
                  <TbBtn title="行内代码" active={editor.isActive("code")} onClick={() => chain().toggleCode().run()}>代码</TbBtn>
                </>
              ) : (
                <>
                  <TbBtn title="下划线 ⌘U" active={editor.isActive("underline")} onClick={() => chain().toggleUnderline().run()}><u>U</u></TbBtn>
                  <TbBtn title="高亮" active={editor.isActive("highlight")} onClick={() => chain().toggleHighlight({ color: "#FDE68A" }).run()}>高亮</TbBtn>
                  <label className="ed-tb-color-field">
                    <span>文字颜色</span>
                    <input
                      type="color"
                      aria-label="文字颜色"
                      value={(editor.getAttributes("textStyle").color as string | undefined) ?? "#000000"}
                      onChange={(event) => chain().setColor(event.target.value).run()}
                    />
                  </label>
                  <div className="ed-tb-highlight-control" role="group" aria-label="高亮底色">
                    {HIGHLIGHT_COLORS.map(({ label, color }) => (
                      <button
                        key={color}
                        type="button"
                        className="ed-tb-highlight-swatch"
                        aria-label={`设置高亮为${label}`}
                        title={`高亮：${label}`}
                        style={{ backgroundColor: color }}
                        onMouseDown={(event) => event.preventDefault()}
                        onClick={() => chain().toggleHighlight({ color }).run()}
                      />
                    ))}
                    <label className="ed-tb-highlight-custom" title="自定义高亮底色">
                      <span>更多</span>
                      <input
                        type="color"
                        aria-label="自定义高亮底色"
                        value={(editor.getAttributes("highlight").color as string | undefined) ?? "#FDE68A"}
                        onChange={(event) => chain().toggleHighlight({ color: event.target.value }).run()}
                      />
                    </label>
                    <TbBtn title="清除高亮" onClick={() => chain().unsetHighlight().run()}>清除高亮</TbBtn>
                  </div>
                  <TbBtn title="上标" active={editor.isActive("superscript")} onClick={() => chain().toggleSuperscript().run()}>X<sup>2</sup></TbBtn>
                  <TbBtn title="下标" active={editor.isActive("subscript")} onClick={() => chain().toggleSubscript().run()}>X<sub>2</sub></TbBtn>
                  <label className="ed-tb-field">
                    <span>字号</span>
                    <select value={currentSize} onChange={(event) => setFontSize(event.target.value)}>
                      <option value="">默认</option>
                      {FONT_SIZES.map((size) => <option key={size} value={size}>{size} pt</option>)}
                    </select>
                  </label>
                  <label className="ed-tb-field">
                    <span>字体</span>
                    <select
                      value={currentFontFamily}
                      onChange={(event) => {
                        if (event.target.value) chain().setFontFamily(event.target.value).run();
                        else chain().unsetFontFamily().run();
                      }}
                    >
                      <option value="">默认</option>
                      {FONT_FAMILIES.map((family) => <option key={family} value={family}>{family}</option>)}
                    </select>
                  </label>
                </>
              )}
              <TbBtn title="清除文字格式（保留段落结构）" onClick={() => chain().unsetAllMarks().run()}>清除格式</TbBtn>
            </ToolbarMenu>

            <ToolbarMenu label="段落" title="段落工具">
              {!isMarkdown && (
                <>
                  <TbBtn title="左对齐" active={editor.isActive({ textAlign: "left" })} onClick={() => chain().setTextAlign("left").run()}>左</TbBtn>
                  <TbBtn title="居中" active={editor.isActive({ textAlign: "center" })} onClick={() => chain().setTextAlign("center").run()}>中</TbBtn>
                  <TbBtn title="右对齐" active={editor.isActive({ textAlign: "right" })} onClick={() => chain().setTextAlign("right").run()}>右</TbBtn>
                  <TbBtn title="两端对齐" active={editor.isActive({ textAlign: "justify" })} onClick={() => chain().setTextAlign("justify").run()}>齐</TbBtn>
                </>
              )}
              <TbBtn title="引用" active={editor.isActive("blockquote")} onClick={() => chain().toggleBlockquote().run()}>引用</TbBtn>
              <TbBtn title="代码块" active={editor.isActive("codeBlock")} onClick={() => chain().toggleCodeBlock().run()}>代码块</TbBtn>
              <TbBtn title="插入分隔线" onClick={() => chain().setHorizontalRule().run()}>分隔线</TbBtn>
            </ToolbarMenu>

            <ToolbarMenu label="列表" title="列表工具">
              <TbBtn title="无序列表" active={editor.isActive("bulletList")} onClick={() => chain().toggleBulletList().run()}>• 列表</TbBtn>
              <TbBtn title="有序列表" active={editor.isActive("orderedList")} onClick={() => chain().toggleOrderedList().run()}>1. 列表</TbBtn>
              {isMarkdown && <TbBtn title="任务列表" active={editor.isActive("taskList")} onClick={() => chain().toggleTaskList().run()}>☐ 任务</TbBtn>}
              <TbBtn title="降低列表层级" disabled={!editor.isActive("listItem")} onClick={() => chain().liftListItem("listItem").run()}>左移</TbBtn>
              <TbBtn title="提高列表层级" disabled={!editor.isActive("listItem")} onClick={() => chain().sinkListItem("listItem").run()}>右移</TbBtn>
            </ToolbarMenu>

            <ToolbarMenu label="插入" title="插入内容">
              {!localImageDropOnly && <TbBtn title="链接" active={editor.isActive("link")} onClick={promptLink}>链接</TbBtn>}
              <TbBtn title={localImageDropOnly ? "请将本地 PNG/JPEG 图片拖入文档" : "插入图片"} disabled={localImageDropOnly} onClick={promptImage}>图片</TbBtn>
              <TbBtn title="插入表格（3×3）" onClick={() => chain().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run()}>表格</TbBtn>
            </ToolbarMenu>

            {paper !== undefined && (
              <ToolbarMenu label="页面视图" title="页面视图">
                <label className="ed-tb-field">
                  <span>纸张</span>
                  <select value={paper} onChange={(event) => onPaperChange?.(event.target.value as PaperKind)}>
                    {PAPER_KINDS.map((kind) => <option key={kind} value={kind}>{kind}</option>)}
                  </select>
                </label>
                <TbBtn title={orientation === "landscape" ? "切换为竖向视图" : "切换为横向视图"} active={orientation === "landscape"} onClick={() => onOrientationChange?.(orientation === "landscape" ? "portrait" : "landscape")}>
                  {orientation === "landscape" ? "横向" : "竖向"}
                </TbBtn>
              </ToolbarMenu>
            )}
          </>
        )}
      </div>
    </div>
  );
}
