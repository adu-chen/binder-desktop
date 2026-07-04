import { useState } from "react";
import { modelKey, modelStore, useModelStore } from "../../store/modelStore";
import { uploadByok } from "../../services/agService";

const PROVIDERS = ["deepseek", "openai", "anthropic"];

/**
 * 模型配置界面（PI-AG-UI-01「添加模型」入口，最小可用版）。
 * 三块：已配置模型（选择/删除）、添加模型、Provider 密钥（BYOK，自原 Composer 🔑 入口迁入）。
 * 密钥零残留：仅一次性经后端 byok_upload 送网关代持，客户端不保存不回显（X-FORBID-001）。
 * 债务（A 类）：完整设置体系与模型能力探测延后续批。
 */
export function ModelConfig({ onClose }: { onClose: () => void }) {
  const { models, selectedKey } = useModelStore();
  const [provider, setProvider] = useState(PROVIDERS[0]);
  const [modelName, setModelName] = useState("");
  const [label, setLabel] = useState("");
  const [byokProvider, setByokProvider] = useState(PROVIDERS[0]);
  const [byokKey, setByokKey] = useState("");
  const [byokStatus, setByokStatus] = useState("");

  const add = () => {
    const m = modelName.trim();
    if (!m) return;
    modelStore.addModel({ provider, model: m, label: label.trim() || m });
    setModelName("");
    setLabel("");
  };

  const saveByok = async () => {
    if (!byokKey.trim()) return;
    setByokStatus("上送中…");
    try {
      await uploadByok(byokProvider, byokKey);
      setByokKey(""); // 客户端不保留密钥
      setByokStatus(`✓ ${byokProvider} 密钥已送网关`);
    } catch (e) {
      const err = e as { message?: string };
      setByokStatus(`✗ ${err?.message ?? "上送失败"}`);
    }
  };

  return (
    <div className="modelcfg-overlay" onClick={onClose}>
      <div className="modelcfg" role="dialog" aria-label="模型配置" onClick={(e) => e.stopPropagation()}>
        <div className="modelcfg-head">
          <span className="modelcfg-title">模型配置</span>
          <button className="ghost modelcfg-close" onClick={onClose} aria-label="关闭">
            ×
          </button>
        </div>

        <div className="modelcfg-sec">
          <span className="label">已配置模型</span>
          {models.map((m) => {
            const key = modelKey(m);
            return (
              <div key={key} className="modelcfg-row">
                <button
                  className={`modelcfg-pick${selectedKey === key ? " selected" : ""}`}
                  onClick={() => modelStore.select(key)}
                >
                  {selectedKey === key ? "✓ " : ""}
                  {m.label}
                  <span className="label modelcfg-key">{key}</span>
                </button>
                <button
                  className="ghost modelcfg-del"
                  onClick={() => modelStore.removeModel(key)}
                  disabled={models.length <= 1}
                  title={models.length <= 1 ? "至少保留一个模型" : "删除"}
                >
                  删除
                </button>
              </div>
            );
          })}
        </div>

        <div className="modelcfg-sec">
          <span className="label">添加模型</span>
          <div className="modelcfg-form">
            <select className="byok-provider" value={provider} onChange={(e) => setProvider(e.target.value)}>
              {PROVIDERS.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
            <input
              className="byok-input"
              placeholder="模型 ID（如 deepseek-chat）"
              value={modelName}
              onChange={(e) => setModelName(e.target.value)}
            />
            <input
              className="byok-input modelcfg-label-input"
              placeholder="显示名（可选）"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
            />
            <button onClick={add} disabled={!modelName.trim()}>
              添加
            </button>
          </div>
        </div>

        <div className="modelcfg-sec">
          <span className="label">Provider 密钥（BYOK）</span>
          <div className="modelcfg-form">
            <select className="byok-provider" value={byokProvider} onChange={(e) => setByokProvider(e.target.value)}>
              {PROVIDERS.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
            <input
              className="byok-input"
              type="password"
              placeholder="粘贴 provider 密钥"
              value={byokKey}
              onChange={(e) => setByokKey(e.target.value)}
            />
            <button onClick={() => void saveByok()} disabled={!byokKey.trim()}>
              保存
            </button>
            {byokStatus && <span className="byok-status">{byokStatus}</span>}
          </div>
          <p className="muted modelcfg-note">密钥仅一次性送网关代持，不在本地保存、不回显（X-FORBID-001）。</p>
        </div>
      </div>
    </div>
  );
}
