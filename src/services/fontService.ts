import { invoke } from "@tauri-apps/api/core";

/**
 * 本地字体库只读枚举（ED-FONT-LOCAL-LIBRARY-01）。
 * `fonts.list_fonts`（macOS CoreText）返回全部已注册字体族名；前端缓存，
 * 失败/空回退由调用方（EditorToolbar）处理，不阻塞编辑。
 */

interface CommandResponse<T> {
  correlation_id: string;
  result?: T;
  error?: { code: string; message: string; retriable: boolean; reauth_required: boolean };
}

let seq = 0;
async function callCommand<T>(command: string, params: Record<string, unknown>): Promise<T> {
  const request = { command, correlation_id: `fonts-${Date.now()}-${(seq += 1)}`, params };
  const response = await invoke<CommandResponse<T>>("dispatch", { request });
  if (response.error) throw response.error;
  if (response.result === undefined) {
    throw { code: "IPC_EMPTY_RESULT", message: `${command} 无返回`, retriable: false, reauth_required: false };
  }
  return response.result;
}

/** module 级缓存：枚举一次，下拉重复打开不重复调用。 */
let cachedFamilies: string[] | null = null;

/** 是否已尝试枚举（失败也置位，避免每次重试；重试由上层显式触发 resetLocalFontCache）。 */
let attempted = false;

/** 枚举本地已注册字体族名（只读）。失败抛错，由调用方回退。 */
export async function listLocalFonts(): Promise<string[]> {
  if (cachedFamilies !== null) return cachedFamilies;
  if (attempted) throw { code: "FONTS_LIST_FAILED", message: "字体枚举已失败", retriable: true, reauth_required: false };
  attempted = true;
  const res = await callCommand<{ families: string[] }>("fonts.list_fonts", {});
  cachedFamilies = Array.isArray(res.families) ? res.families : [];
  return cachedFamilies;
}

/** 清空缓存（如应用设置变化后重试）。 */
export function resetLocalFontCache(): void {
  cachedFamilies = null;
  attempted = false;
}
