import { useAgStore, agStore, type AgMode } from "../../store/agStore";
import { MessageList } from "./MessageList";
import { Composer } from "./Composer";
import { DiffList } from "../de/DiffList";

/**
 * 代理编辑面板 AgentPanel（A-SYS-C-T-02 §3 · PI-FE-06；原名 ChatPanel，PI-AG-SEM-01 更名）。
 * 编辑/构建 模式切换 + 会话标签 + 消息流 + composer。
 * 编辑：AI 交互 + 编辑器感知，对当前内容精确修改并出 diff（人最后裁定）；
 * 构建：项目级蓝图/大纲确认后整体构建（接 CBT，2026-07-04 人类澄清定义）。
 * AI 调用前经订阅门禁硬门（X-SEQ-001，后端 ensure_gate_passed）；客户端不持密钥（X-FORBID-001）。
 */
const MODES: { id: AgMode; label: string }[] = [
  { id: "edit", label: "编辑" },
  { id: "build", label: "构建" },
];

export function AgentPanel() {
  const { mode } = useAgStore();
  return (
    <div className="chatpanel">
      <div className="chatpanel-head">
        <div className="chatpanel-modes">
          {MODES.map((m) => (
            <button
              key={m.id}
              className={`chatmode${mode === m.id ? " active" : ""}`}
              onClick={() => agStore.setMode(m.id)}
              aria-pressed={mode === m.id}
            >
              {m.label}
            </button>
          ))}
        </div>
        <span className="label">会话 · 1</span>
      </div>
      <div className="chatpanel-scroll">
        <MessageList />
        <DiffList />
      </div>
      <Composer />
    </div>
  );
}
