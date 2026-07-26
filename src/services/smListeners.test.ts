import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const eventMock = vi.hoisted(() => ({ listen: vi.fn() }));

vi.mock("@tauri-apps/api/event", () => ({ listen: eventMock.listen }));

import { listen } from "@tauri-apps/api/event";
import { startSmListeners, stopSmListeners } from "./smListeners";

const listenMock = vi.mocked(listen);

beforeEach(() => {
  listenMock.mockReset();
});

afterEach(() => {
  stopSmListeners();
});

describe("smListeners 启动控制", () => {
  it("并发 start 只注册一组状态机监听", async () => {
    listenMock.mockResolvedValue(vi.fn());

    await Promise.all([startSmListeners(), startSmListeners(), startSmListeners()]);

    expect(listenMock).toHaveBeenCalledTimes(5);
  });
});
