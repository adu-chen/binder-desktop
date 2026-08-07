import { useSyncExternalStore } from "react";

/**
 * 模型配置偏好 store（PI-AG-UI-01 演进 · MODEL-CONFIG-01）。
 * 模型注册表（来源分层：preset/byok，BR-AG-DATA-006）+ 能力模型偏好（BR-AG-DATA-007）。
 * 客户端用户偏好（localStorage 持久化），非后端权威业务态（不属 X-FORBID-007 投影范围）；
 * 仅保存 provider/model 标识符与偏好，**不含任何密钥**（X-FORBID-001，密钥经 BYOK 送网关代持）。
 * 债务（A 类）：偏好存储并入统一设置体系延后续批。
 */

export interface ModelOption {
  provider: string;
  model: string;
  label: string;
  /** 来源分层（BR-AG-DATA-006）：preset=预设模型（云端下发/订阅可调用，TERM-AG-010）；byok=BYOK 模型（用户自带密钥、网关代持）。注册表不保存密钥（X-FORBID-001）。 */
  source: "preset" | "byok";
}

/** 四类 agent 能力（REQ-AG-006）：辅助续写 / 局部修改 / 代理编辑 / 代理构建。 */
export type AgentCapability = "autocomplete" | "inlineAssist" | "agentEdit" | "build";

/** 单项能力模型偏好（BR-AG-DATA-007）：跟随全局默认，或显式指定 modelKey。 */
export type ModelPrefValue = { mode: "followGlobal" } | { mode: "explicit"; modelKey: string };

export interface CapabilityPref {
  autocomplete: ModelPrefValue;
  inlineAssist: ModelPrefValue;
  agentEdit: ModelPrefValue;
  build: ModelPrefValue;
}

const DEFAULT_CAPABILITY_PREF: CapabilityPref = {
  autocomplete: { mode: "followGlobal" },
  inlineAssist: { mode: "followGlobal" },
  agentEdit: { mode: "followGlobal" },
  build: { mode: "followGlobal" },
};

/** 默认预设清单（BR-AG-DATA-006：source=preset）与 dev 网关三供应商对齐（lib.rs default_model）。
 *  正式版清单由云端 GET /models 下发（TERM-AG-010，桌面端仅消费）。 */
const DEFAULT_MODELS: ModelOption[] = [
  { provider: "deepseek", model: "deepseek-chat", label: "DeepSeek Chat", source: "preset" },
  { provider: "openai", model: "gpt-4o-mini", label: "GPT-4o mini", source: "preset" },
  { provider: "anthropic", model: "claude-sonnet-4-6", label: "Claude Sonnet", source: "preset" },
];

const LS_MODELS = "binder.ag.models";
const LS_SELECTED = "binder.ag.model.selected";
const LS_CAPABILITY_PREF = "binder.ag.model.capability";

export const modelKey = (m: ModelOption) => `${m.provider}/${m.model}`;

const canPersist = typeof localStorage !== "undefined";

function loadModels(): ModelOption[] {
  if (!canPersist) return DEFAULT_MODELS;
  try {
    const raw = localStorage.getItem(LS_MODELS);
    if (!raw) return DEFAULT_MODELS;
    const arr = JSON.parse(raw) as ModelOption[];
    if (!Array.isArray(arr) || arr.length === 0) return DEFAULT_MODELS;
    // 旧数据无 source 字段 → 归一为 preset（BR-AG-DATA-006 向后兼容读取）。
    return arr
      .filter((m) => m && typeof m.provider === "string" && typeof m.model === "string")
      .map((m) => ({ ...m, source: m.source === "byok" ? "byok" : "preset" }));
  } catch {
    return DEFAULT_MODELS;
  }
}

function loadCapabilityPref(): CapabilityPref {
  if (!canPersist) return DEFAULT_CAPABILITY_PREF;
  try {
    const raw = localStorage.getItem(LS_CAPABILITY_PREF);
    if (!raw) return DEFAULT_CAPABILITY_PREF;
    const parsed = JSON.parse(raw) as Partial<CapabilityPref>;
    return {
      autocomplete: parsed.autocomplete ?? { mode: "followGlobal" },
      inlineAssist: parsed.inlineAssist ?? { mode: "followGlobal" },
      agentEdit: parsed.agentEdit ?? { mode: "followGlobal" },
      build: parsed.build ?? { mode: "followGlobal" },
    };
  } catch {
    return DEFAULT_CAPABILITY_PREF;
  }
}

interface ModelState {
  models: ModelOption[];
  selectedKey: string;
  capabilityPref: CapabilityPref;
}

function init(): ModelState {
  const models = loadModels();
  let selectedKey = canPersist ? localStorage.getItem(LS_SELECTED) ?? "" : "";
  if (!models.some((m) => modelKey(m) === selectedKey)) selectedKey = modelKey(models[0]);
  return { models, selectedKey, capabilityPref: loadCapabilityPref() };
}

let state: ModelState = init();
const listeners = new Set<() => void>();

function persist() {
  if (!canPersist) return;
  try {
    localStorage.setItem(LS_MODELS, JSON.stringify(state.models));
    localStorage.setItem(LS_SELECTED, state.selectedKey);
    localStorage.setItem(LS_CAPABILITY_PREF, JSON.stringify(state.capabilityPref));
  } catch {
    /* 存储不可用时静默（偏好非关键路径） */
  }
}
function setState(next: ModelState) {
  state = next;
  persist();
  for (const l of listeners) l();
}

export const modelStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): ModelState {
    return state;
  },
  /** 全局默认模型（列表兜底第一项，恒非空）。 */
  current(): ModelOption {
    return state.models.find((m) => modelKey(m) === state.selectedKey) ?? state.models[0];
  },
  /** 设置全局默认模型。 */
  select(key: string) {
    if (state.models.some((m) => modelKey(m) === key)) setState({ ...state, selectedKey: key });
  },
  /** 添加并选中；同 provider/model 去重（更新 label）。 */
  addModel(m: ModelOption) {
    const key = modelKey(m);
    const models = state.models.some((x) => modelKey(x) === key)
      ? state.models.map((x) => (modelKey(x) === key ? m : x))
      : [...state.models, m];
    setState({ ...state, models, selectedKey: key });
  },
  /** 删除（至少保留一个）；删除选中项时回落第一项；显式偏好引用被删条目 → 回落跟随全局（BR-AG-DATA-007）。 */
  removeModel(key: string) {
    if (state.models.length <= 1) return;
    const models = state.models.filter((m) => modelKey(m) !== key);
    const selectedKey = state.selectedKey === key ? modelKey(models[0]) : state.selectedKey;
    const capabilityPref: CapabilityPref = Object.fromEntries(
      Object.entries(state.capabilityPref).map(([cap, pref]) => [
        cap,
        pref.mode === "explicit" && pref.modelKey === key ? { mode: "followGlobal" } : pref,
      ]),
    ) as CapabilityPref;
    setState({ models, selectedKey, capabilityPref });
  },
  /** 设置单项能力模型偏好（BR-AG-DATA-007）。 */
  setCapabilityPref(cap: AgentCapability, pref: ModelPrefValue) {
    setState({ ...state, capabilityPref: { ...state.capabilityPref, [cap]: pref } });
  },
  /**
   * @GOV
   * codes:    BR-AG-DATA-007-QUERY-AG-AG-MODEL-CATALOG-013
   * type:     QUERY
   * chain:    AG-MODEL-CATALOG, AG-ASSIST, AG-TURN
   * rules:    BR-AG-DATA-007
   * term_ref: TERM-AG-010
   * boundary: in=AgentCapabilityAndCapabilityPrefAndModelRegistry | out=ModelOption(resolvedExplicitPreferenceOrGlobalDefaultNeverDanglingModelKey)
   */
  resolveFor(cap: AgentCapability): ModelOption {
    const pref = state.capabilityPref[cap];
    if (pref.mode === "explicit") {
      const explicit = state.models.find((m) => modelKey(m) === pref.modelKey);
      if (explicit) return explicit;
    }
    return state.models.find((m) => modelKey(m) === state.selectedKey) ?? state.models[0];
  },
  /**
   * @GOV
   * codes:    BR-AG-DATA-006-EFFECT-AG-AG-MODEL-CATALOG-006
   * type:     EFFECT
   * chain:    AG-MODEL-CATALOG
   * rules:    BR-AG-DATA-006
   * term_ref: TERM-AG-010
   * boundary: in=PresetModelOptionListFromCloud | out=models(presetEntriesReplacedByokEntriesPreservedSelectedKeyKeptValid)
   *           | effect=memory
   *           | does_not=persist_api_key
   *           | does_not=touch_byok_entries
   */
  /** 合并云端预设清单（BR-AG-DATA-006）：替换 preset 条目、保留 byok 条目；空清单不改变注册表；原选中项被移除时回落第一项。 */
  mergePresetModels(presetModels: ModelOption[]) {
    if (!presetModels.length) return;
    const seen = new Set<string>();
    const merged: ModelOption[] = state.models.filter((m) => m.source === "byok");
    for (const m of presetModels) {
      const key = modelKey(m);
      if (seen.has(key)) continue;
      seen.add(key);
      merged.push({ provider: m.provider, model: m.model, label: m.label, source: "preset" });
    }
    if (!merged.length) return;
    const selectedKey = state.models.some((m) => modelKey(m) === state.selectedKey)
      ? state.selectedKey
      : modelKey(merged[0]);
    setState({ ...state, models: merged, selectedKey });
  },
  /** 测试辅助：恢复到默认注册表与偏好（仅测试使用）。 */
  resetForTest() {
    state = init();
    for (const l of listeners) l();
  },
};

export function useModelStore(): ModelState {
  return useSyncExternalStore(modelStore.subscribe, modelStore.getSnapshot);
}
