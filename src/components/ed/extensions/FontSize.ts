import { Extension } from "@tiptap/core";

/**
 * FontSize 扩展（移植旧项目 binder-project）：为 textStyle 标记附加 fontSize 全局属性，
 * 解析 docx→HTML 的内联 `style="font-size:..."`（样式子集「部分支持」层），渲染回内联样式。
 * 与 TextStyle 配合（types: ['textStyle']）。
 */

export interface FontSizeOptions {
  types: string[];
}

export function parseFontSize(element: HTMLElement): string | null {
  const size = element.style.fontSize;
  return size ? normalizeFontSize(size) : null;
}

/** 正规化字号值：去除空格、去除 ".0" 后缀、统一 pt 单位格式。
 *  审计修复（2026-07-24）：Pandoc 产出的 "12.0pt" 与手动设置的 "12pt" 应视为同一字号，
 *  否则 sameMark 的 JSON 比较会认为它们是不同的 attrs，阻止 mark 段合并。 */
function normalizeFontSize(raw: string): string {
  let s = raw.trim().toLowerCase();
  // 正规化 "12.0pt" → "12pt", "12.00pt" → "12pt"
  s = s.replace(/\.0+(pt)$/, "$1");
  // 统一空格： "12 pt" → "12pt"
  s = s.replace(/\s*(pt)\s*$/, "$1");
  return s;
}

export function renderFontSize(fontSize: unknown): Record<string, string> {
  return fontSize ? { style: `font-size: ${fontSize}` } : {};
}

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    fontSize: {
      setFontSize: (fontSize: string) => ReturnType;
      unsetFontSize: () => ReturnType;
    };
  }
}

export const FontSize = Extension.create<FontSizeOptions>({
  name: "fontSize",

  addOptions() {
    return { types: ["textStyle"] };
  },

  addGlobalAttributes() {
    return [
      {
        types: this.options.types,
        attributes: {
          fontSize: {
            default: null,
            parseHTML: parseFontSize,
            renderHTML: (attributes) => renderFontSize(attributes.fontSize),
          },
        },
      },
    ];
  },

  addCommands() {
    return {
      setFontSize:
        (fontSize: string) =>
        ({ chain }) =>
          chain().setMark("textStyle", { fontSize }).run(),
      unsetFontSize:
        () =>
        ({ chain }) =>
          chain().setMark("textStyle", { fontSize: null }).removeEmptyTextStyle().run(),
    };
  },
});
