import { beforeEach, describe, expect, it } from "vitest";
import { workspaceShell } from "./workspaceShell";

beforeEach(() => {
  workspaceShell._projectActive(null);
  workspaceShell._projectRecent([]);
  workspaceShell.setNotice(null);
});

describe("workspaceShell 外壳态投影（X-FORBID-007）", () => {
  it("投影活动 workspace；置 null 回欢迎态", () => {
    workspaceShell._projectActive({ workspaceId: "w1", rootPath: "/p" });
    expect(workspaceShell.getSnapshot().activeWorkspace).toEqual({ workspaceId: "w1", rootPath: "/p" });
    workspaceShell._projectActive(null);
    expect(workspaceShell.getSnapshot().activeWorkspace).toBeNull();
  });

  it("最近列表 loading → ready / error", () => {
    workspaceShell._projectRecentLoading();
    expect(workspaceShell.getSnapshot().recentStatus).toBe("loading");
    workspaceShell._projectRecent([{ workspaceId: "w1", rootPath: "/p" }]);
    expect(workspaceShell.getSnapshot()).toMatchObject({ recentStatus: "ready", recent: [{ workspaceId: "w1", rootPath: "/p" }] });
    workspaceShell._projectRecentError("e");
    expect(workspaceShell.getSnapshot()).toMatchObject({ recentStatus: "error", recentError: "e" });
  });

  it("死项目提示 notice 可设可清", () => {
    workspaceShell.setNotice("项目不可用");
    expect(workspaceShell.getSnapshot().notice).toBe("项目不可用");
    workspaceShell.setNotice(null);
    expect(workspaceShell.getSnapshot().notice).toBeNull();
  });
});
