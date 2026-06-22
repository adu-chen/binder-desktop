import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { workspaceShell, type WorkspaceRefView } from "../store/workspaceShell";

/**
 * Service 层（X-SEQ-002）：封装跨进程 command 调用，前端 UI 经此触达后端，不跨层直连。
 * 命名承接 A-SYS-I-P-02（`模块前缀.动作` 逻辑命名 + CommandRequest 信封）+ A-WS-I-P-01（ws.* 签名）。
 * PI-FE-00：经统一 dispatch 入口投递信封，后端 CommandRegistry 校验后路由（BR-SYS-DATA-003）。
 */

/** CommandRequest 信封（A-SYS-I-P-02 §3，与后端 ipc::CommandRequest 对齐）。 */
interface CommandRequest {
  command: string;
  correlation_id: string;
  params: Record<string, unknown>;
}

/** 统一错误形状（BR-SYS-OBS-002，与后端 ipc::IpcError 对齐）。 */
interface IpcError {
  code: string;
  message: string;
  retriable: boolean;
  reauth_required: boolean;
}

/** 响应信封：result 与 error 互斥。 */
interface CommandResponse<T> {
  correlation_id: string;
  result?: T;
  error?: IpcError;
}

interface WorkspaceRefDto {
  workspace_id: string;
  root_path: string;
  is_active: boolean;
}

let correlationCounter = 0;
function nextCorrelationId(): string {
  correlationCounter += 1;
  return `fe-${Date.now()}-${correlationCounter}`;
}

/**
 * 经统一 dispatch 入口调用一个逻辑 command；result/error 互斥，error 抛出统一错误形状。
 * 这是前端触达后端的唯一通道（X-SEQ-002），各 ws.* / ag.* 调用复用本函数。
 */
async function callCommand<T>(command: string, params: Record<string, unknown>): Promise<T> {
  const request: CommandRequest = { command, correlation_id: nextCorrelationId(), params };
  const response = await invoke<CommandResponse<T>>("dispatch", { request });
  if (response.error) throw response.error;
  if (response.result === undefined) {
    throw { code: "IPC_EMPTY_RESULT", message: `${command} 无返回`, retriable: false, reauth_required: false } satisfies IpcError;
  }
  return response.result;
}

/**
 * 打开/新建 workspace：弹目录选择器（plugin-dialog）→ ws.open_workspace → 投影后端活动态。
 * 用户取消选择则静默返回（不视为错误）。workspace 活动态以后端为准，前端只投影（X-FORBID-007）。
 */
function toView(ref: WorkspaceRefDto): WorkspaceRefView {
  return { workspaceId: ref.workspace_id, rootPath: ref.root_path };
}

function ipcMessage(e: unknown): string {
  const err = e as Partial<IpcError>;
  return err?.message ? `[${err.code ?? "?"}] ${err.message}` : String(e);
}

export async function openWorkspace(): Promise<void> {
  const selected = await open({ directory: true, multiple: false, title: "选择 workspace 目录" });
  if (selected === null || Array.isArray(selected)) return; // 用户取消
  try {
    const ref = await callCommand<WorkspaceRefDto>("ws.open_workspace", { rootPath: selected });
    workspaceShell._projectActive(toView(ref));
  } catch (e) {
    console.error(`ws.open_workspace failed ${ipcMessage(e)}`);
  }
}

/** 拉取最近 workspace 列表（投影 loading → ready/error；BR-SYS-OBS-002 错误形状）。 */
export async function loadRecentWorkspaces(): Promise<void> {
  workspaceShell._projectRecentLoading();
  try {
    const list = await callCommand<WorkspaceRefDto[]>("ws.list_workspaces", {});
    workspaceShell._projectRecent(list.map(toView));
  } catch (e) {
    workspaceShell._projectRecentError(ipcMessage(e));
  }
}

/** 切换到已注册 workspace（最近列表点击入口）；活动态以后端为准。 */
export async function switchWorkspace(workspaceId: string): Promise<void> {
  try {
    const ref = await callCommand<WorkspaceRefDto>("ws.switch_workspace", { workspaceId });
    workspaceShell._projectActive(toView(ref));
  } catch (e) {
    console.error(`ws.switch_workspace failed ${ipcMessage(e)}`);
  }
}

/** 启动恢复：投影后端当前活动 workspace（无则保持欢迎页）。 */
export async function restoreActiveWorkspace(): Promise<void> {
  try {
    const ref = await callCommand<WorkspaceRefDto | null>("ws.active_workspace", {});
    if (ref) workspaceShell._projectActive(toView(ref));
  } catch (e) {
    console.error(`ws.active_workspace failed ${ipcMessage(e)}`);
  }
}
