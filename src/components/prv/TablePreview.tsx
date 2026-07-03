import { useEffect, useState } from "react";
import { loadTableData, type TableData } from "../../services/tableData";

/**
 * 表格式只读预览（2026-07-03 #1 · PRV-VIEW · BR-PRV-DATA-001 只读硬边界）。
 * csv/tsv/xlsx/xls 渲染为 HTML 表格：保留行列结构与多 sheet（标签切换）、支持选中复制；
 * **无编辑命令面**（无输入控件/无可编辑单元格/无写回调用）。超限截断明示。
 * 保真边界：值投影（不含列宽/颜色/合并单元格样式），全保真查看走导出 PDF。
 */
export function TablePreview({ path }: { path: string }) {
  const [data, setData] = useState<TableData | null>(null);
  const [active, setActive] = useState(0);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const [errMsg, setErrMsg] = useState("");

  useEffect(() => {
    let alive = true;
    setStatus("loading");
    setActive(0);
    loadTableData(path)
      .then((d) => {
        if (!alive) return;
        setData(d);
        setStatus("ready");
      })
      .catch((e: { message?: string }) => {
        if (!alive) return;
        setErrMsg(e?.message ?? "表格解析失败");
        setStatus("error");
      });
    return () => {
      alive = false;
    };
  }, [path]);

  if (status === "loading") return <p className="muted tablepreview-hint">解析表格中…</p>;
  if (status === "error" || !data) return <p className="muted tablepreview-hint">✗ {errMsg}</p>;

  const sheet = data.sheets[Math.min(active, data.sheets.length - 1)];
  const colCount = sheet.rows.reduce((m, r) => Math.max(m, r.length), 0);

  return (
    <div className="tablepreview">
      {data.sheets.length > 1 && (
        <div className="tablepreview-tabs" role="tablist">
          {data.sheets.map((s, i) => (
            <button
              key={s.name}
              role="tab"
              aria-selected={i === active}
              className={`tablepreview-tab${i === active ? " active" : ""}`}
              onClick={() => setActive(i)}
            >
              {s.name}
            </button>
          ))}
        </div>
      )}
      <div className="tablepreview-scroll">
        <table className="tablepreview-table">
          <tbody>
            {sheet.rows.map((row, ri) => (
              <tr key={ri}>
                <th className="tablepreview-rownum">{ri + 1}</th>
                {Array.from({ length: colCount }, (_, ci) => (
                  <td key={ci}>{row[ci] ?? ""}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        {sheet.rows.length === 0 && <p className="muted tablepreview-hint">（空表）</p>}
      </div>
      {data.truncated && (
        <div className="tablepreview-truncated">表格过大，仅显示前 5000 行 / 256 列（全量请导出查看）</div>
      )}
    </div>
  );
}
