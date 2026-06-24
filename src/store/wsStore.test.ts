import { beforeEach, describe, expect, it } from "vitest";
import { wsStore } from "./wsStore";

beforeEach(() => wsStore.reset());

describe("wsStore 文件树状态投影", () => {
  it("初始 idle；reset 回 idle 空集", () => {
    expect(wsStore.getSnapshot()).toEqual({ docs: [], status: "idle", error: null });
  });

  it("loading → ready（投影 docs）", () => {
    wsStore._projectLoading();
    expect(wsStore.getSnapshot().status).toBe("loading");
    wsStore._projectDocs([{ relPath: "a.md", docType: "md", isDir: false }]);
    const s = wsStore.getSnapshot();
    expect(s.status).toBe("ready");
    expect(s.docs).toHaveLength(1);
    expect(s.error).toBeNull();
  });

  it("error 投影错误信息", () => {
    wsStore._projectError("boom");
    expect(wsStore.getSnapshot()).toMatchObject({ status: "error", error: "boom" });
  });

  it("subscribe 收到变更通知", () => {
    let calls = 0;
    const unsub = wsStore.subscribe(() => (calls += 1));
    wsStore._projectLoading();
    wsStore._projectError("x");
    unsub();
    wsStore._projectLoading();
    expect(calls).toBe(2); // 退订后不再收到
  });
});
