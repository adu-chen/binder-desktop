import { describe, it, expect } from "vitest";
import { reconcileBlockIds } from "./BlockIdExtension";

// 确定性 id 生成器（注入，避免时间戳/随机）。
function seqGen() {
  let n = 0;
  return () => `n${++n}`;
}

// covers: BR-ED-DATA-001
// 稳定 blockId 守则：既有唯一 id 稳定不变；缺失补新；复制/分裂的重复 id 令后者改取新 id；新 id 不复用。
describe("BlockIdExtension · reconcileBlockIds (BR-ED-DATA-001)", () => {
  it("既有且唯一的 id 全部保持稳定（返回 null=不改）", () => {
    expect(reconcileBlockIds(["a", "b", "c"], seqGen())).toEqual([null, null, null]);
  });

  it("缺失（null）的块补发新 id", () => {
    expect(reconcileBlockIds(["a", null, "c"], seqGen())).toEqual([null, "n1", null]);
  });

  it("重复 id（复制/分裂）：首个保持稳定，后出现者改取新 id", () => {
    // 第二个 "a" 为重复 → 改新；第一个 "a" 稳定
    expect(reconcileBlockIds(["a", "a"], seqGen())).toEqual([null, "n1"]);
  });

  it("新 id 不复用既有 id、全序列唯一", () => {
    const out = reconcileBlockIds([null, null, "x"], seqGen());
    const finalIds = ["a-placeholder"]; // 占位，下方按规则核验
    void finalIds;
    // 两个新 id 互不相同，且不等于既有 "x"
    expect(out[0]).not.toBe(out[1]);
    expect(out).not.toContain("x");
    expect(out[2]).toBeNull(); // 既有 x 稳定
  });

  it("生成 id 与既有冲突时跳过重取（不复用）", () => {
    // gen 先吐 "x"（与既有冲突）应被跳过，改取下一个
    let i = 0;
    const collidingGen = () => (i++ === 0 ? "x" : "y");
    const out = reconcileBlockIds(["x", null], collidingGen);
    expect(out[0]).toBeNull(); // 既有 x 稳定
    expect(out[1]).toBe("y"); // 跳过冲突的 "x"，取 "y"
  });
});
