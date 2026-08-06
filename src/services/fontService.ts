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

/**
 * 枚举本地已注册字体族名（只读）。失败抛错，由调用方回退常用子集。
 * 失败**不缓存、不锁死**（修复 ED-FONT-LOCAL-LIBRARY-01 锁死缺陷）：dev 启动期 Rust backend
 * 未就绪的瞬时 IPC 失败只影响当次调用，下次调用自动重试；仅成功结果进入缓存。
 */
export async function listLocalFonts(): Promise<string[]> {
  if (cachedFamilies !== null) return cachedFamilies;
  const res = await callCommand<{ families: string[] }>("fonts.list_fonts", {});
  cachedFamilies = Array.isArray(res.families) ? res.families : [];
  return cachedFamilies;
}

/** 清空缓存（如应用设置变化后重试）。 */
export function resetLocalFontCache(): void {
  cachedFamilies = null;
}
