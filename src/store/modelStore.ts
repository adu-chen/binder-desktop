import { useSyncExternalStore } from "react";

/**
 * 模型选择偏好 store（PI-AG-UI-01）。
 * 客户端用户偏好（localStorage 持久化），非后端权威业务态（不属 X-FORBID-007 投影范围）；
 * 仅保存 provider/model 标识符，**不含任何密钥**（X-FORBID-001，密钥经 BYOK 送网关代持）。
 * 债务（A 类）：偏好存储并入统一设置体系延后续批。
 */

export interface ModelOption {
  provider: string;
  model: string;
  label: string;
}

/** 默认清单与 dev 网关三供应商对齐（lib.rs default_model）。 */
const DEFAULT_MODELS: ModelOption[] = [
  { provider: "deepseek", model: "deepseek-chat", label: "DeepSeek Chat" },
  { provider: "openai", model: "gpt-4o-mini", label: "GPT-4o mini" },
  { provider: "anthropic", model: "claude-sonnet-4-6", label: "Claude Sonnet" },
];

const LS_MODELS = "binder.ag.models";
const LS_SELECTED = "binder.ag.model.selected";

export const modelKey = (m: ModelOption) => `${m.provider}/${m.model}`;

const canPersist = typeof localStorage !== "undefined";

function loadModels(): ModelOption[] {
  if (!canPersist) return DEFAULT_MODELS;
  try {
    const raw = localStorage.getItem(LS_MODELS);
    if (!raw) return DEFAULT_MODELS;
    const arr = JSON.parse(raw) as ModelOption[];
    if (!Array.isArray(arr) || arr.length === 0) return DEFAULT_MODELS;
    return arr.filter((m) => m && typeof m.provider === "string" && typeof m.model === "string");
  } catch {
    return DEFAULT_MODELS;
  }
}

interface ModelState {
  models: ModelOption[];
  selectedKey: string;
}

function init(): ModelState {
  const models = loadModels();
  let selectedKey = canPersist ? localStorage.getItem(LS_SELECTED) ?? "" : "";
  if (!models.some((m) => modelKey(m) === selectedKey)) selectedKey = modelKey(models[0]);
  return { models, selectedKey };
}

let state: ModelState = init();
const listeners = new Set<() => void>();

function persist() {
  if (!canPersist) return;
  try {
    localStorage.setItem(LS_MODELS, JSON.stringify(state.models));
    localStorage.setItem(LS_SELECTED, state.selectedKey);
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
  /** 当前选中模型（列表兜底第一项，恒非空）。 */
  current(): ModelOption {
    return state.models.find((m) => modelKey(m) === state.selectedKey) ?? state.models[0];
  },
  select(key: string) {
    if (state.models.some((m) => modelKey(m) === key)) setState({ ...state, selectedKey: key });
  },
  /** 添加并选中；同 provider/model 去重（更新 label）。 */
  addModel(m: ModelOption) {
    const key = modelKey(m);
    const models = state.models.some((x) => modelKey(x) === key)
      ? state.models.map((x) => (modelKey(x) === key ? m : x))
      : [...state.models, m];
    setState({ models, selectedKey: key });
  },
  /** 删除（至少保留一个）；删除选中项时回落第一项。 */
  removeModel(key: string) {
    if (state.models.length <= 1) return;
    const models = state.models.filter((m) => modelKey(m) !== key);
    const selectedKey = state.selectedKey === key ? modelKey(models[0]) : state.selectedKey;
    setState({ models, selectedKey });
  },
};

export function useModelStore(): ModelState {
  return useSyncExternalStore(modelStore.subscribe, modelStore.getSnapshot);
}
