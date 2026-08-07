import { useEffect, useState } from "react";
import { modelKey, modelStore, useModelStore, type AgentCapability } from "../../store/modelStore";
import { uploadByok, listPresetModels, byokListProviders, byokDeleteKey, type ByokProviderStatus } from "../../services/agService";

const PROVIDERS = ["deepseek", "openai", "anthropic"];

/** 四类 agent 能力展示项（REQ-AG-006 / BR-AG-DATA-007）。 */
const CAPABILITY_ITEMS: { cap: AgentCapability; label: string }[] = [
  { cap: "autocomplete", label: "辅助续写" },
  { cap: "inlineAssist", label: "局部修改" },
  { cap: "agentEdit", label: "代理编辑" },
  { cap: "build", label: "代理构建" },
];

const FOLLOW_GLOBAL = "__follow_global__";

/**
 * 模型配置界面（MODEL-CONFIG-01 演进，PI-AG-UI-01 入口）。
 * 四块：已配置模型（预设/BYOK 分组，选择/删除）、添加模型（BYOK 来源）、
 * Provider 密钥（BYOK 上送）、能力模型偏好（四类能力各自跟随全局或显式指定，BR-AG-DATA-007）。
 * 密钥零残留：仅一次性经后端 byok_upload 送网关代持，客户端不保存不回显（X-FORBID-001）。
 * 债务（A 类）：完整设置体系与模型能力探测延后续批。
 */
export function ModelConfig({ onClose }: { onClose: () => void }) {
  const { models, selectedKey, capabilityPref } = useModelStore();
  const [provider, setProvider] = useState(PROVIDERS[0]);
  const [modelName, setModelName] = useState("");
  const [label, setLabel] = useState("");
  const [byokProvider, setByokProvider] = useState(PROVIDERS[0]);
  const [byokKey, setByokKey] = useState("");
  const [byokStatus, setByokStatus] = useState("");
  const [presetStatus, setPresetStatus] = useState("");
  const [byokProviders, setByokProviders] = useState<ByokProviderStatus[]>([]);

  // 挂载时拉取云端预设清单并入注册表（TERM-AG-010 / BR-AG-DATA-006）与已配置 BYOK 状态（脱敏）。
  useEffect(() => {
    let cancelled = false;
    void listPresetModels()
      .then((list) => {
        if (cancelled) return;
        if (list.length) {
          modelStore.mergePresetModels(
            list.map((m) => ({ provider: m.provider, model: m.model, label: m.label, source: "preset" as const })),
          );
          setPresetStatus(`已同步 ${list.length} 个预设模型`);
        } else {
          setPresetStatus("预设清单为空（dev GW 未运行？）");
        }
      })
      .catch(() => {
        if (!cancelled) setPresetStatus("预设清单拉取失败（dev GW 不可达）");
      });
    void byokListProviders()
      .then((providers) => {
        if (!cancelled) setByokProviders(providers);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  const presetModels = models.filter((m) => m.source === "preset");
  const byokModels = models.filter((m) => m.source === "byok");

  const add = () => {
    const m = modelName.trim();
    if (!m) return;
    // 手动添加默认为 BYOK 来源（用户自带密钥模型；预设清单由云端下发，BR-AG-DATA-006）。
    modelStore.addModel({ provider, model: m, label: label.trim() || m, source: "byok" });
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
      setByokProviders(await byokListProviders().catch(() => []));
    } catch (e) {
      const err = e as { message?: string };
      setByokStatus(`✗ ${err?.message ?? "上送失败"}`);
    }
  };

  const removeByok = async (provider: string) => {
    try {
      await byokDeleteKey(provider);
      setByokProviders((prev) => prev.filter((p) => p.provider !== provider));
      setByokStatus(`✓ ${provider} 密钥已删除`);
    } catch (e) {
      const err = e as { message?: string };
      setByokStatus(`✗ ${err?.message ?? "删除失败"}`);
    }
  };

  const renderModelRow = (m: (typeof models)[number]) => {
    const key = modelKey(m);
    return (
      <div key={key} className="modelcfg-row">
        <button
          className={`modelcfg-pick${selectedKey === key ? " selected" : ""}`}
          onClick={() => modelStore.select(key)}
        >
          {selectedKey === key ? "✓ " : ""}
          {m.label}
          <span className="label modelcfg-src">{m.source === "byok" ? "BYOK" : "预设"}</span>
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
  };

  const renderCapabilityRow = (item: (typeof CAPABILITY_ITEMS)[number]) => {
    const pref = capabilityPref[item.cap];
    const currentValue = pref.mode === "explicit" ? pref.modelKey : FOLLOW_GLOBAL;
    return (
      <div key={item.cap} className="modelcfg-cap-row">
        <span className="label modelcfg-cap-label">{item.label}</span>
        <select
          className="modelcfg-cap-select"
          aria-label={`${item.label} 模型`}
          value={currentValue}
          onChange={(e) => {
            const value = e.target.value;
            modelStore.setCapabilityPref(
              item.cap,
              value === FOLLOW_GLOBAL
                ? { mode: "followGlobal" }
                : { mode: "explicit", modelKey: value },
            );
          }}
        >
          <option value={FOLLOW_GLOBAL}>跟随全局（{modelStore.current().label}）</option>
          {models.map((m) => {
            const key = modelKey(m);
            return (
              <option key={key} value={key}>
                {m.label}（{m.source === "byok" ? "BYOK" : "预设"}）
              </option>
            );
          })}
        </select>
      </div>
    );
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
          <span className="label">已配置模型 · 预设</span>
          {presetStatus && <p className="muted modelcfg-note">{presetStatus}</p>}
          {presetModels.length ? (
            presetModels.map(renderModelRow)
          ) : (
            <p className="muted modelcfg-note">无预设模型（清单由云端下发，待拉取）。</p>
          )}
          <span className="label modelcfg-group-gap">已配置模型 · BYOK</span>
          {byokModels.length ? (
            byokModels.map(renderModelRow)
          ) : (
            <p className="muted modelcfg-note">无 BYOK 模型，可在下方添加。</p>
          )}
        </div>

        <div className="modelcfg-sec">
          <span className="label">添加模型（BYOK）</span>
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

        {byokProviders.length > 0 && (
          <div className="modelcfg-sec">
            <span className="label">已配置 BYOK Provider</span>
            {byokProviders.map((p) => (
              <div key={p.provider} className="modelcfg-row">
                <span className="modelcfg-pick-readonly">
                  {p.provider}
                  <span className="label modelcfg-key">{p.key_preview}</span>
                </span>
                <button className="ghost modelcfg-del" onClick={() => void removeByok(p.provider)}>
                  删除
                </button>
              </div>
            ))}
          </div>
        )}

        <div className="modelcfg-sec">
          <span className="label">能力模型偏好（BR-AG-DATA-007）</span>
          {CAPABILITY_ITEMS.map(renderCapabilityRow)}
          <p className="muted modelcfg-note">各项默认跟随全局默认模型；可单独指定后独立生效（辅助续写/局部修改/代理编辑/代理构建）。</p>
        </div>
      </div>
    </div>
  );
}
