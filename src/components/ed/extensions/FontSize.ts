import { Extension } from "@tiptap/core";

/**
 * FontSize 扩展（移植旧项目 binder-project）：为 textStyle 标记附加 fontSize 全局属性，
 * 解析 docx→HTML 的内联 `style="font-size:..."`（样式子集「部分支持」层），渲染回内联样式。
 * 与 TextStyle 配合（types: ['textStyle']）。
 */

export interface FontSizeOptions {
  types: string[];
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
            parseHTML: (element) => {
              const size = (element as HTMLElement).style.fontSize;
              if (!size) return null;
              // 移除单位，只保留数字（与旧项目一致）。
              return size.replace(/px|pt|em|rem|%/, "");
            },
            renderHTML: (attributes) => {
              if (!attributes.fontSize) return {};
              return { style: `font-size: ${attributes.fontSize}px` };
            },
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
