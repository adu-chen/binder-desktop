import { Extension } from "@tiptap/core";

const HEX_COLOR = /^#([0-9a-f]{6})$/i;
const RGB_COLOR = /^rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})(?:\s*,\s*([01](?:\.\d+)?))?\s*\)$/i;

/** 将 DOM/CSS 输入收紧为 TableCellShading 支持的 `#RRGGBB`；透明色和其他 CSS 形式不进入模型。 */
export function normalizeTableCellShading(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const raw = value.trim();
  const hex = raw.match(HEX_COLOR);
  if (hex) return `#${hex[1].toUpperCase()}`;
  const rgb = raw.match(RGB_COLOR);
  if (!rgb) return null;
  const channels = rgb.slice(1, 4).map(Number);
  const alpha = rgb[4] === undefined ? 1 : Number(rgb[4]);
  if (channels.some((channel) => !Number.isInteger(channel) || channel < 0 || channel > 255) || alpha !== 1) return null;
  return `#${channels.map((channel) => channel.toString(16).padStart(2, "0")).join("").toUpperCase()}`;
}

export function parseTableCellShading(element: HTMLElement): string | null {
  return normalizeTableCellShading(
    element.getAttribute("data-binder-cell-shading") ?? element.style.backgroundColor,
  );
}

export function renderTableCellShading(value: unknown): Record<string, string> {
  const color = normalizeTableCellShading(value);
  return color
    ? {
        "data-binder-cell-shading": color,
        style: `background-color: ${color}`,
      }
    : {};
}

/**
 * @GOV
 * codes:    BR-ED-DATA-007-DATA-ED-ED-EDIT-018
 * type:     DATA
 * chain:    ED-EDIT
 * rules:    BR-ED-DATA-007
 * term_ref: TERM-ED-007, TERM-DOC-005
 * boundary: in=tableCellOrTableHeaderDOM | out=TableCellShadingAttribute(backgroundColorAsOpaqueHexOrNull)
 */
export const TableCellShading = Extension.create({
  name: "tableCellShading",

  addGlobalAttributes() {
    return [{
      types: ["tableCell", "tableHeader"],
      attributes: {
        backgroundColor: {
          default: null,
          parseHTML: parseTableCellShading,
          renderHTML: (attributes) => renderTableCellShading(attributes.backgroundColor),
        },
      },
    }];
  },
});
