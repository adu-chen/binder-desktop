import { useRef, useState } from "react";
import { Channel, invoke } from "@tauri-apps/api/core";

const readRecoveryCases = [
  ["page-contract-recovery", "completed", "分页参数拒绝后恢复（仅核心）"],
  ["page-contract-exhausted", "error", "分页参数纠错上限（仅核心）"],
  ["repeated-oversized-read", "completed", "重复读取失败后回答（仅核心）"],
  ["repeated-read-tools-blocked", "completed", "收尾期工具禁止（仅核心）"],
  ["read-failure-same-batch", "completed", "同批后续工具禁止（仅核心）"],
  ["read-failure-with-evidence", "completed", "失败后保留已有读取证据（仅核心）"],
  ["read-failure-gate-denied", "gate_rejected", "收尾阶段门禁拒绝（仅核心）"],
  ["read-failure-clarification", "completed", "失败澄清后窄读续行（仅核心）"],
] as const;

/** @GOV
 * codes: BR-AG-UI-001-IO-AG-AG-TURN-062
 * type: IO
 * chain: AG-TURN
 * rules: BR-AG-UI-001
 * boundary: in=DevelopmentScenarioSelectionSafeTurnEventsAndIsolatedToolMessages | out=ObservedVerificationStatusAndEvidence | does_not=manufactureToolSuccessOrMutateUserWorkspace
 */
export function AgentVerificationPanel() {
  const [scenario, setScenario] = useState("chat");
  const [events, setEvents] = useState<unknown[]>([]);
  const [budgetChecks, setBudgetChecks] = useState<unknown[]>([]);
  const [requestCounts, setRequestCounts] = useState({ gate: 0, model: 0 });
  const [decisionTrace, setDecisionTrace] = useState<unknown[]>([]);
  const [previewTrace, setPreviewTrace] = useState<unknown | null>(null);
  const [effectTrace, setEffectTrace] = useState<unknown | null>(null);
  const [promptTrace, setPromptTrace] = useState<unknown[]>([]);
  const [sessionTrace, setSessionTrace] = useState<unknown | null>(null);
  const [followUp, setFollowUp] = useState<{ turnId: string; sessionId: string; status: string } | null>(null);
  const [toolMessages, setToolMessages] = useState<unknown[]>([]);
  const [status, setStatus] = useState("尚未运行");
  const [busy, setBusy] = useState(false);
  const active = useRef<string | null>(null);
  const [batchResults, setBatchResults] = useState<unknown[]>([]);
  const batchStopped = useRef(false);
  async function runBatch() {
    if (active.current) return;
    active.current = "batch";
    batchStopped.current = false;
    setBusy(true); setBatchResults([]); setStatus("批量核心验收执行中（非业务成功）");
    const cases = [
      ["chat", "completed"], ["glob", "completed"], ["invalid-json", "error"], ["truncated", "error"],
      ["tool-failure", "completed"], ["gate-denied", "gate_rejected"], ["delay", "cancelled"],
      ["budget-initial", "error"], ["budget-after-tool", "error"], ["budget-recovery", "error"],
      ["budget-recovery-delay", "error"], ["decision-chain", "completed"],
      ["decision-duplicate", "error"], ["decision-limit", "error"],
      ["preview-guard", "preview_guard_checked"], ["session-memory", "session_memory_checked"],
      ["effect-port", "effect_port_checked"],
      ...readRecoveryCases.map(([id, expected]) => [id, expected]),
      ["execution-budget-wrap", "completed"], ["action-handoff-stage", "completed"],
      ["action-handoff-plan", "completed"],
      ["tool-description-compatible", "completed"], ["protocol-cumulative", "error"],
      ["native-business-tools", "completed"],
    ];
    let verified = 0;
    try {
      for (const [selected, expectedStatus] of cases) {
        if (batchStopped.current) break;
        const id = crypto.randomUUID(); active.current = id;
        const channel = new Channel<unknown>();
        channel.onmessage = () => { /* 完整事件取自该次真实持久回执，不制造 UI 成功。 */ };
        let cancellation: Promise<unknown> | undefined;
        let cancellationError: unknown;
        let timer: ReturnType<typeof setTimeout> | undefined;
        if (selected === "delay" || selected === "budget-recovery-delay") {
          timer = setTimeout(() => {
            if (!batchStopped.current) cancellation = invoke("ag_verify_cancel", { runId: id })
              .catch(error => { cancellationError = error; });
          }, 500);
        }
        let receipt: Record<string, unknown>;
        try {
          receipt = await invoke<Record<string, unknown>>("ag_verify_run", { scenario: selected, runId: id, onEvent: channel });
          if (cancellation) await cancellation;
          if (cancellationError) throw cancellationError;
        } finally { if (timer) clearTimeout(timer); }
        const followUp = receipt.followUp as { status?: string } | undefined;
        const matched = receipt.status === expectedStatus
          && (selected !== "budget-recovery" || followUp?.status === "completed")
          && (selected !== "budget-recovery-delay" || followUp?.status === "cancelled");
        setBatchResults(old => [...old, { scenario: selected, expectedStatus, matched, receipt }]);
        if (!matched) throw new Error(`场景 ${selected} 的实际终态不符合验收要求`);
        verified += 1;
      }
      setStatus(batchStopped.current ? `批量验收已停止，已核验 ${verified}/${cases.length}；未运行项不计通过。` : `批量核心验收 ${verified}/${cases.length} 终态匹配；请查看执行回执，不代表专业业务成功。`);
    } catch (error) {
      setBatchResults(old => [...old, { infrastructureError: error instanceof Error ? error.message : String(error), verified }]);
      setStatus(`批量验收失败，已核验 ${verified}/${cases.length}；失败及未运行项不计通过。`);
    } finally { active.current = null; setBusy(false); }
  }
  async function run() {
    if (active.current) return;
    const id = crypto.randomUUID();
    active.current = id;
    setBusy(true); setEvents([]); setBudgetChecks([]); setRequestCounts({ gate: 0, model: 0 }); setDecisionTrace([]); setPreviewTrace(null); setEffectTrace(null); setPromptTrace([]); setSessionTrace(null); setFollowUp(null); setToolMessages([]); setStatus("执行中");
    const channel = new Channel<unknown>();
    channel.onmessage = (event) => { if (active.current === id) setEvents((old) => [...old, event]); };
    try {
      const receipt = await invoke<{ status: string; followUp?: { turnId: string; sessionId: string; status: string } | null; events: unknown[]; budgetChecks?: unknown[]; requestCounts?: { gate: number; model: number }; decisionTrace?: unknown[]; previewTrace?: unknown; effectTrace?: unknown; promptTrace?: unknown[]; sessionTrace?: unknown; modelToolMessages: unknown[] }>("ag_verify_run", { scenario, runId: id, onEvent: channel });
      if (active.current === id) { setEvents(receipt.events); setBudgetChecks(receipt.budgetChecks ?? []); setRequestCounts(receipt.requestCounts ?? { gate: 0, model: 0 }); setDecisionTrace(receipt.decisionTrace ?? []); setPreviewTrace(receipt.previewTrace ?? null); setEffectTrace(receipt.effectTrace ?? null); setPromptTrace(receipt.promptTrace ?? []); setSessionTrace(receipt.sessionTrace ?? null); setFollowUp(receipt.followUp ?? null); setToolMessages(receipt.modelToolMessages); setStatus(receipt.status === "preview_guard_checked" ? "隔离预览检查点守卫已核验（非 Agent 回合、非编辑器验收）" : receipt.status === "session_memory_checked" ? "隔离会话与显式引用装配已核验（非真实模型、非记忆库面板验收）" : receipt.status === "effect_port_checked" ? "独立执行端口已核验（真实文本提案；未接受、未保存）" : `Agent 状态：${receipt.status}（不等于业务成功）`); }
    } catch (error) {
      setStatus(`验证入口失败：${JSON.stringify(error)}`);
    } finally { active.current = null; setBusy(false); }
  }
  async function cancel() {
    batchStopped.current = true;
    const id = active.current;
    if (!id) return;
    try { await invoke("ag_verify_cancel", { runId: id }); if (active.current === id) setStatus("已请求取消，等待运行终态"); }
    catch (error) { if (active.current === id) setStatus(`取消未确认：${JSON.stringify(error)}`); }
  }
  return <details>
    <summary>开发验证（隔离环境）</summary>
    <p>仅固定模型传输；生产解析、授权与工具仍执行。使用临时文件，不使用当前工作区。不是实际订阅或专业编辑验收。</p>
    <select aria-label="验证场景" value={scenario} disabled={busy} onChange={(e) => setScenario(e.target.value)}>
      <option value="chat">纯对话</option><option value="glob">真实 txt 文件枚举</option>
      <option value="invalid-json">非法 JSON</option><option value="truncated">流截断</option>
      <option value="tool-failure">读取失败</option><option value="gate-denied">门禁拒绝</option>
      <option value="delay">延迟响应（可取消）</option>
      <option value="budget-initial">首次装配超限</option><option value="budget-after-tool">工具后装配超限</option>
      <option value="budget-recovery">预算错误后同会话新回合</option>
      <option value="budget-recovery-delay">预算错误后新回合延迟（可取消）</option>
      <option value="decision-chain">自动裁决 A→B（仅核心）</option>
      <option value="decision-duplicate">裁决恢复重复调用拒绝（仅核心）</option>
      <option value="decision-limit">连续裁决预算不重置（仅核心）</option>
      <option value="preview-guard">预览检查点守卫（无编辑器）</option>
      <option value="session-memory">多轮、跨会话与显式记忆（仅核心）</option>
      <option value="effect-port">执行端口：文本提案/资源扫描取消与超时（不接受、不保存）</option>
      {readRecoveryCases.map(([id, , label]) => <option key={id} value={id}>{label}</option>)}
      <option value="execution-budget-wrap">执行额度用尽后收尾（仅核心）</option>
      <option value="action-handoff-stage">缺计划纠正为阶段报告（仅核心）</option>
      <option value="action-handoff-plan">缺计划纠正后确认续行（临时TXT）</option>
      <option value="tool-description-compatible">多工具描述兼容（仅核心）</option>
      <option value="native-business-tools">独立业务工具与双工具真实读取（仅核心）</option>
      <option value="protocol-cumulative">拒绝→成功→拒绝累计纠错（仅核心）</option>
    </select>
    <button disabled={busy} onClick={() => void run()}>运行验证</button>
    <button disabled={busy} onClick={() => void runBatch()}>批量运行核心验收</button>
    <button disabled={!busy} onClick={() => void cancel()}>取消验证</button>
    <p role="status">{status}</p>
    <pre aria-label="批量核心验收证据" style={{ maxHeight: 240, overflow: "auto", whiteSpace: "pre-wrap" }}>{JSON.stringify(batchResults, null, 2)}</pre>
    {followUp && <p aria-label="后续回合状态">同会话 {followUp.sessionId} 的新回合 {followUp.turnId}：{followUp.status}。首轮错误仍保留；两个 AgentTurn 各自判定，不代表业务成功。</p>}
    <pre aria-label="运行证据" style={{ maxHeight: 240, overflow: "auto", whiteSpace: "pre-wrap" }}>{JSON.stringify(events, null, 2)}</pre>
    <p>隔离网关实际请求：Gate {requestCounts.gate} 次，模型 {requestCounts.model} 次。以下是内部预算计数，不是公开 TurnEvent 或业务成功证据：</p>
    <pre aria-label="预算分项证据" style={{ maxHeight: 240, overflow: "auto", whiteSpace: "pre-wrap" }}>{JSON.stringify(budgetChecks, null, 2)}</pre>
    <p>隔离入口自动裁决轨迹（非用户点击、非 DecisionBar 布局验收）：</p>
    <pre aria-label="自动裁决证据" style={{ maxHeight: 240, overflow: "auto", whiteSpace: "pre-wrap" }}>{JSON.stringify(decisionTrace, null, 2)}</pre>
    <p>隔离预览检查点证据（只验证后端身份、版本与一次消费；不验证前端超时、实际编辑器、接受或保存）：</p>
    <pre aria-label="预览检查点证据" style={{ maxHeight: 240, overflow: "auto", whiteSpace: "pre-wrap" }}>{JSON.stringify(previewTrace, null, 2)}</pre>
    <p>隔离模型请求的分层标记观察（仅布尔值；不导出完整 Prompt，不验证模型理解或记忆库面板）：</p>
    <pre aria-label="执行端口效果证据" style={{ maxHeight: 240, overflow: "auto", whiteSpace: "pre-wrap" }}>{JSON.stringify(effectTrace, null, 2)}</pre>
    <pre aria-label="会话装配证据" style={{ maxHeight: 240, overflow: "auto", whiteSpace: "pre-wrap" }}>{JSON.stringify(promptTrace, null, 2)}</pre>
    <p>隔离 AgentSession / AgentTurn 持久状态与等待证据（非实际标签切换布局验收）：</p>
    <pre aria-label="会话状态证据" style={{ maxHeight: 240, overflow: "auto", whiteSpace: "pre-wrap" }}>{JSON.stringify(sessionTrace, null, 2)}</pre>
    <p>隔离模型请求中实际收到的工具回注（不是界面安全摘要）：</p>
    <pre aria-label="工具回注证据" style={{ maxHeight: 240, overflow: "auto", whiteSpace: "pre-wrap" }}>{JSON.stringify(toolMessages, null, 2)}</pre>
  </details>;
}
