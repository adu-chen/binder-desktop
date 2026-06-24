import { useSyncExternalStore } from "react";

/**
 * PRV 预览态（PRV-VIEW · 视图层）。previewPath = 当前预览的文件（docx/Excel/PPT）；
 * 非空时主区显示只读预览面板替代编辑器（只读硬边界，A-PRV-M-T-01）。预览不进 ED、不可编辑。
 */
interface PrvState {
  previewPath: string | null;
}

let state: PrvState = { previewPath: null };
const listeners = new Set<() => void>();
function emit() {
  for (const l of listeners) l();
}

export const prvStore = {
  subscribe(cb: () => void) {
    listeners.add(cb);
    return () => listeners.delete(cb);
  },
  getSnapshot(): PrvState {
    return state;
  },
  open(path: string) {
    state = { previewPath: path };
    emit();
  },
  close() {
    if (state.previewPath !== null) {
      state = { previewPath: null };
      emit();
    }
  },
};

export function usePrvStore(): PrvState {
  return useSyncExternalStore(prvStore.subscribe, prvStore.getSnapshot);
}
