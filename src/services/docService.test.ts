import { describe, it, expect } from "vitest";
import { rewriteMediaSrc } from "./docService";

// covers: BR-ED-DATA-003
// 图文呈现：编辑链 HTML 相对 <img src> 改写为 asset URL；绝对/内联/已处理 src 原样保留；不改正文文本。
// 用 DI fake converter 断言改写落点，不依赖 tauri 运行时（convertFileSrc）。
const fake = (p: string) => `asset://localhost/${p}`;

describe("docService · rewriteMediaSrc (BR-ED-DATA-003)", () => {
  it("相对 media 路径 → asset URL（拼接绝对路径）", () => {
    const html = `<p>x</p><img src="media/image1.png" alt="a">`;
    const out = rewriteMediaSrc(html, "/tmp/work", fake);
    expect(out).toContain(`src="asset://localhost//tmp/work/media/image1.png"`);
    expect(out).toContain(`alt="a"`); // 其余属性保留
    expect(out).toContain(`<p>x</p>`); // 正文文本不动
  });

  it("多张图片全部改写", () => {
    const html = `<img src="media/a.png"><img src='media/b.jpg'>`;
    const out = rewriteMediaSrc(html, "/w", fake);
    expect(out).toContain(`asset://localhost//w/media/a.png`);
    expect(out).toContain(`asset://localhost//w/media/b.jpg`);
  });

  it("data: / https: / asset: / blob: src 原样保留", () => {
    const html =
      `<img src="data:image/png;base64,AAAA">` +
      `<img src="https://example.com/x.png">` +
      `<img src="asset://localhost/already">` +
      `<img src="blob:abc">`;
    expect(rewriteMediaSrc(html, "/w", fake)).toBe(html);
  });

  it("./ 前缀相对路径归一化后拼接", () => {
    const out = rewriteMediaSrc(`<img src="./media/x.png">`, "/w", fake);
    expect(out).toContain(`asset://localhost//w/media/x.png`);
  });

  it("base 尾部斜杠去重", () => {
    const out = rewriteMediaSrc(`<img src="media/x.png">`, "/w/", fake);
    expect(out).toContain(`asset://localhost//w/media/x.png`);
    expect(out).not.toContain(`//w//media`);
  });

  it("无图片的 HTML 原样返回", () => {
    const html = `<h1>标题</h1><p>正文 <strong>粗体</strong></p>`;
    expect(rewriteMediaSrc(html, "/w", fake)).toBe(html);
  });

  it("单双引号 src 均支持", () => {
    expect(rewriteMediaSrc(`<img src='media/x.png'>`, "/w", fake)).toContain(
      `asset://localhost//w/media/x.png`,
    );
  });
});
