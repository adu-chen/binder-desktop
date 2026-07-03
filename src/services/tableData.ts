import { invoke } from "@tauri-apps/api/core";
import { readDoc } from "./wsService";

/**
 * 表格预览数据服务（2026-07-03 #1 · PRV 表格式预览 · BR-PRV-DATA-001 只读）。
 * - csv/tsv：**文本格式**，前端直接解析（RFC4180 基本子集纯函数，同 .html 先例不经 DOC）。
 * - xlsx/xls：**二进制格式**，经 `prv.table_data`（DOC calamine 管线，X-SEQ-004 唯一通道）。
 * 输出统一 {sheets:[{name, rows}], truncated} 供 TablePreview 渲染（只读可选中复制，不可编辑）。
 */

export interface SheetData {
  name: string;
  rows: string[][];
}

export interface TableData {
  sheets: SheetData[];
  truncated: boolean;
}

/** csv 预览规模上限（与后端 xlsx 同口径：超限截断明示）。 */
const MAX_ROWS = 5000;

/**
 * RFC4180 基本子集解析纯逻辑（供预览与单测共用）：引号字段（内嵌分隔符/换行/`""` 转义）、
 * \r\n 归一。delimiter 传 "\t" 即 tsv。**只读解析不回写**——预览不承担 csv 序列化（BR-PRV-DATA-001）。
 */
export function parseCsv(text: string, delimiter = ","): { rows: string[][]; truncated: boolean } {
  const rows: string[][] = [];
  let field = "";
  let row: string[] = [];
  let inQuotes = false;
  let truncated = false;
  const pushField = () => {
    row.push(field);
    field = "";
  };
  const pushRow = () => {
    pushField();
    rows.push(row);
    row = [];
  };
  for (let i = 0; i < text.length; i++) {
    if (rows.length >= MAX_ROWS) {
      truncated = true;
      break;
    }
    const c = text[i];
    if (inQuotes) {
      if (c === '"') {
        if (text[i + 1] === '"') {
          field += '"'; // "" 转义
          i += 1;
        } else {
          inQuotes = false;
        }
      } else {
        field += c;
      }
    } else if (c === '"' && field === "") {
      inQuotes = true;
    } else if (c === delimiter) {
      pushField();
    } else if (c === "\n") {
      pushRow();
    } else if (c === "\r") {
      // \r\n 归一（跳过；孤立 \r 也按行终结处理）
      if (text[i + 1] !== "\n") pushRow();
    } else {
      field += c;
    }
  }
  if (!truncated && (field !== "" || row.length > 0)) pushRow();
  // 末尾空行（文件以换行结尾产生的 [""]）剔除。
  while (rows.length > 0 && rows[rows.length - 1].length === 1 && rows[rows.length - 1][0] === "") {
    rows.pop();
  }
  return { rows, truncated };
}

interface CommandResponse<T> {
  correlation_id: string;
  result?: T;
  error?: { code: string; message: string; retriable: boolean; reauth_required: boolean };
}

let seq = 0;
async function callCommand<T>(command: string, params: Record<string, unknown>): Promise<T> {
  const request = { command, correlation_id: `tbl-${Date.now()}-${(seq += 1)}`, params };
  const response = await invoke<CommandResponse<T>>("dispatch", { request });
  if (response.error) throw response.error;
  if (response.result === undefined) {
    throw { code: "IPC_EMPTY_RESULT", message: `${command} 无返回`, retriable: false, reauth_required: false };
  }
  return response.result;
}

/** 是否表格式预览格式（csv/tsv 前端解析；xlsx/xls 经 DOC 管线）。 */
export function isTableFile(relPath: string): boolean {
  const ext = relPath.toLowerCase().split(".").pop() ?? "";
  return ext === "csv" || ext === "tsv" || ext === "xlsx" || ext === "xls";
}

/** 统一取表格数据（按扩展名分流两条合规管线）。 */
export async function loadTableData(relPath: string): Promise<TableData> {
  const ext = relPath.toLowerCase().split(".").pop() ?? "";
  if (ext === "csv" || ext === "tsv") {
    const { content } = await readDoc(relPath);
    const { rows, truncated } = parseCsv(content, ext === "tsv" ? "\t" : ",");
    return { sheets: [{ name: relPath.split("/").pop() ?? relPath, rows }], truncated };
  }
  return callCommand<TableData>("prv.table_data", { relPath });
}
