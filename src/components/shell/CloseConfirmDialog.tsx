import { useState } from "react";
import { useCloseStore } from "../../store/closeStore";
import type { CloseDecision } from "../../store/closeStore";
import { cancelClose, submitDecision, forceQuit } from "../../services/closeService";

/**
 * 关闭前处理弹窗（BR-SYS-STATE-004，APP-CLOSE-DIFF-LIFECYCLE-01）。
 * 单弹窗方案：批量决策（接受全部/拒绝全部 + 保存/放弃），底部唯一执行按钮"关闭应用"；
 * 选择"接受全部"时未保存文件强制走保存（不提供"接受但放弃"组合，2026-07-20 人类决策 2）。
 * 详情区默认折叠，按文档聚合展示，不逐条罗列 diff（裁决粒度已是全部接受/全部拒绝）。
 */

type DiffChoice = "accept" | "reject" | null;
type FileChoice = "save" | "discard" | null;

export function CloseConfirmDialog() {
  const { phase, pendingDiffCount, awaitingSaveCount, dirtyDocuments, busy, error } = useCloseStore();
  const [diffChoice, setDiffChoice] = useState<DiffChoice>(null);
  const [fileChoice, setFileChoice] = useState<FileChoice>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);

  if (phase === "idle" || phase === "closing") return null;

  const hasDiffs = pendingDiffCount > 0;
  const hasAwaitingSave = awaitingSaveCount > 0;
  const hasDirty = dirtyDocuments.length > 0;
  const checking = phase === "checking";

  const selectDiffChoice = (choice: DiffChoice) => {
    if (busy) return;
    setDiffChoice(choice);
    if (choice === "accept" && hasDirty) setFileChoice("save"); // 强制耦合：接受全部必须保存
  };
  const selectFileChoice = (choice: FileChoice) => {
    if (busy || diffChoice === "accept") return; // 接受全部时禁用"放弃"
    setFileChoice(choice);
  };

  function computeDecision(): CloseDecision | null {
    if (hasDiffs && diffChoice === null) return null;
    if (hasDirty && fileChoice === null) return null;
    if (diffChoice === "accept") return "acceptAllAndSave";
    if (!hasDirty) return "rejectAllAndDiscard"; // 无文件可保存/放弃，二者等价
    return fileChoice === "save" ? "rejectAllAndSave" : "rejectAllAndDiscard";
  }
  const decision = computeDecision();

  const summary = (() => {
    if (!decision) return "请先选择处理方式";
    const parts: string[] = [];
    if (hasDiffs) parts.push(diffChoice === "accept" ? `接受 ${pendingDiffCount} 条修改` : `拒绝 ${pendingDiffCount} 条修改`);
    if (hasDirty) {
      parts.push(
        decision === "acceptAllAndSave" || decision === "rejectAllAndSave"
          ? `保存 ${dirtyDocuments.length} 个文件`
          : `放弃 ${dirtyDocuments.length} 个文件的修改`,
      );
    }
    if (hasAwaitingSave) parts.push(`处理 ${awaitingSaveCount} 项未确认保存的修改效果`);
    return `将：${parts.join("，并")}`;
  })();

  const handleClose = async () => {
    if (!decision || busy) return;
    await submitDecision(decision);
  };

  return (
    <div className="close-dialog-overlay" role="dialog" aria-modal="true">
      <div className="close-dialog">
        <div className="close-dialog-head">
          <div className="close-dialog-title">关闭前处理</div>
          <div className="close-dialog-subtitle">
            {checking
              ? "正在检查未处理项…"
              : `本次关闭涉及 ${pendingDiffCount} 条待裁决 AI 修改、${dirtyDocuments.length} 个未保存文件${hasAwaitingSave ? `，以及 ${awaitingSaveCount} 项未确认保存的效果` : ""}`}
          </div>
        </div>

        {!checking && (
          <div className="close-dialog-body">
            {hasDiffs && (
              <div className="close-dialog-section">
                <div className="close-dialog-section-label">AI 修改</div>
                <div className="close-dialog-choices">
                  <button
                    type="button"
                    className={`close-dialog-choice ${diffChoice === "accept" ? "selected" : ""}`}
                    disabled={busy}
                    onClick={() => selectDiffChoice("accept")}
                  >
                    <span className="close-dialog-choice-title">接受全部 {pendingDiffCount}</span>
                    <span className="close-dialog-choice-hint">写入所涉文件</span>
                  </button>
                  <button
                    type="button"
                    className={`close-dialog-choice ${diffChoice === "reject" ? "selected" : ""}`}
                    disabled={busy}
                    onClick={() => selectDiffChoice("reject")}
                  >
                    <span className="close-dialog-choice-title">拒绝全部 {pendingDiffCount}</span>
                    <span className="close-dialog-choice-hint">不写入文件</span>
                  </button>
                </div>
              </div>
            )}

            {hasDirty && (
              <div className="close-dialog-section">
                <div className="close-dialog-section-label">未保存文件</div>
                <label className={`close-dialog-radio ${diffChoice === "accept" ? "disabled" : ""}`}>
                  <input
                    type="radio"
                    name="close-file-choice"
                    checked={fileChoice === "save"}
                    disabled={busy || diffChoice === "accept"}
                    onChange={() => selectFileChoice("save")}
                  />
                  保存修改（{dirtyDocuments.length} 个文件）
                </label>
                <label className={`close-dialog-radio ${diffChoice === "accept" ? "disabled" : ""}`}>
                  <input
                    type="radio"
                    name="close-file-choice"
                    checked={fileChoice === "discard"}
                    disabled={busy || diffChoice === "accept"}
                    onChange={() => selectFileChoice("discard")}
                  />
                  放弃修改（不可恢复）
                </label>
              </div>
            )}

            {hasAwaitingSave && !hasDirty && (
              <div className="close-dialog-section">
                <div className="close-dialog-section-label">未确认保存的修改效果</div>
                <div className="close-dialog-note">
                  这些修改已被接受但未确认写入磁盘；关闭时将放弃该效果，避免其跨会话继续生效。
                </div>
              </div>
            )}

            {dirtyDocuments.length > 0 && (
              <div className="close-dialog-details">
                <button
                  type="button"
                  className="close-dialog-details-toggle"
                  onClick={() => setDetailsOpen((v) => !v)}
                >
                  详情 {detailsOpen ? "﹀" : "›"}
                </button>
                {detailsOpen && (
                  <ul className="close-dialog-details-list">
                    {dirtyDocuments.map((d) => (
                      <li key={d.relPath}>
                        {d.relPath}
                        {d.hasAcceptedAwaitingSave ? " · 含已接受未保存的 AI 修改" : ""} · 未保存
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}

            {error && <div className="close-dialog-error">{error}</div>}
          </div>
        )}

        <div className="close-dialog-foot">
          <div className="close-dialog-summary">{checking ? "" : summary}</div>
          <div className="close-dialog-actions">
            {error && (
              <button type="button" className="close-dialog-force" disabled={busy} onClick={() => void forceQuit()}>
                强制退出（可能丢失数据）
              </button>
            )}
            <button type="button" className="ghost" disabled={busy} onClick={cancelClose}>
              取消
            </button>
            <button
              type="button"
              className="close-dialog-confirm"
              disabled={checking || busy || !decision}
              onClick={() => void handleClose()}
            >
              {busy ? "处理中…" : "关闭应用"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
