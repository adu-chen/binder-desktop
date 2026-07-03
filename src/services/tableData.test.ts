import { describe, it, expect } from "vitest";
import { parseCsv, isTableFile } from "./tableData";

// covers: BR-PRV-DATA-001
// csv 只读解析纯逻辑（RFC4180 基本子集）：引号字段/内嵌分隔符/"" 转义/\r\n 归一/末尾空行剔除/
// 超限截断明示。预览只解析不回写（只读硬边界）。
describe("tableData · parseCsv", () => {
  it("基本行列 + \\r\\n 归一 + 末尾空行剔除", () => {
    const { rows, truncated } = parseCsv("a,b,c\r\n1,2,3\r\n");
    expect(rows).toEqual([["a", "b", "c"], ["1", "2", "3"]]);
    expect(truncated).toBe(false);
  });

  it("引号字段：内嵌逗号/换行/双引号转义", () => {
    const { rows } = parseCsv('"x,y",2\n"多\n行","he said ""hi"""');
    expect(rows).toEqual([
      ["x,y", "2"],
      ["多\n行", 'he said "hi"'],
    ]);
  });

  it("tsv 分隔符", () => {
    expect(parseCsv("a\tb\n1\t2", "\t").rows).toEqual([["a", "b"], ["1", "2"]]);
  });

  it("空文件 → 空行集", () => {
    expect(parseCsv("").rows).toEqual([]);
  });

  it("isTableFile：csv/tsv/xlsx/xls 判定", () => {
    expect(isTableFile("a/b.csv")).toBe(true);
    expect(isTableFile("x.XLSX")).toBe(true);
    expect(isTableFile("x.tsv")).toBe(true);
    expect(isTableFile("x.md")).toBe(false);
  });
});
