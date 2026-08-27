//! 表格数据预览链（2026-07-03 #1 · PRV 表格式预览 · DOC-CONVERT / X-SEQ-004）。
//!
//! 兑现 A-PRV-M-T-01 §9 预设通道：「Excel/PPT 经 LibreOffice 的保真度需实测，**不达另议管线**」——
//! xlsx→PDF 实测丢失表格基础特征（行列/多 sheet 不可选中复制），另议为**结构化表格数据管线**：
//! calamine 解析 xlsx/xls（二进制解析置于 DOC，X-SEQ-004 唯一通道，前端不内置解析库）→
//! 结构化 {sheets:[{name, rows}]} → PRV 前端渲染 HTML 表格（只读可选中复制，BR-PRV-DATA-001）。
//! csv/tsv 为文本格式由前端直接解析（不经本管线，同 .html 先例）。
//! 保真边界（诚实声明）：取单元格**值**（含公式计算结果），不取样式（列宽/颜色/合并单元格），
//! 与旧项目 SheetJS 方案特征集等同；全保真查看仍可走导出 PDF。

use crate::ipc::IpcError;
use calamine::{open_workbook_auto, Data, Reader};
use std::path::Path;

/// 单 sheet 数据：名称 + 行列值矩阵（字符串投影）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct SheetData {
    pub name: String,
    pub rows: Vec<Vec<String>>,
}

/// 预览规模上限（防超大表拖垮 IPC/渲染；超限截断并标记，前端明示）。
const MAX_ROWS: usize = 5000;
const MAX_COLS: usize = 256;

fn cell_to_string(c: &Data) -> String {
    match c {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            // 整数值不显示小数尾（Excel 常规格显示习惯）。
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#ERR:{e:?}"),
    }
}

/**
 * @GOV
 * codes:    BR-PRV-DATA-002-IO-PRV-PRV-VIEW-004
 * type:     IO
 * chain:    PRV-VIEW
 * rules:    BR-PRV-DATA-002
 * boundary: in=xlsxFilePath | out=sheetDataMatrix(readonlyValuesTruncatedAtLimitsNoStyleNoEdit)
 */
/// xlsx/xls → 结构化表格数据（BR-PRV-DATA-002 预览转换 / BR-PRV-DATA-001 只读）：calamine 读值
/// （含公式结果），多 sheet；超限截断（行 5000 / 列 256）随 truncated 标记明示不静默；
/// 解析失败回统一错误形状（不静默空表）。本地解析不上行。
pub fn xlsx_to_sheets(abs_path: &Path) -> Result<(Vec<SheetData>, bool), IpcError> {
    let mut wb = open_workbook_auto(abs_path).map_err(|e| {
        IpcError::new(
            "PRV_TABLE_PARSE_FAIL",
            &format!("表格解析失败: {e}"),
            false,
            false,
        )
    })?;
    let names: Vec<String> = wb.sheet_names().to_vec();
    let mut sheets = Vec::new();
    let mut truncated = false;
    for name in names {
        let range = match wb.worksheet_range(&name) {
            Ok(r) => r,
            Err(e) => {
                return Err(IpcError::new(
                    "PRV_TABLE_PARSE_FAIL",
                    &format!("工作表 {name} 读取失败: {e}"),
                    false,
                    false,
                ))
            }
        };
        let mut rows: Vec<Vec<String>> = Vec::new();
        for (ri, row) in range.rows().enumerate() {
            if ri >= MAX_ROWS {
                truncated = true;
                break;
            }
            if row.len() > MAX_COLS {
                truncated = true;
            }
            rows.push(row.iter().take(MAX_COLS).map(cell_to_string).collect());
        }
        sheets.push(SheetData { name, rows });
    }
    if sheets.is_empty() {
        return Err(IpcError::new(
            "PRV_TABLE_EMPTY",
            "工作簿无工作表",
            false,
            false,
        ));
    }
    Ok((sheets, truncated))
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-PRV-DATA-002
    // 单元格值投影：整数浮点不带小数尾、布尔大写、空为空串（表格式预览的值语义）。
    #[test]
    fn cell_value_projection() {
        assert_eq!(cell_to_string(&Data::Float(3.0)), "3");
        assert_eq!(cell_to_string(&Data::Float(3.5)), "3.5");
        assert_eq!(cell_to_string(&Data::Bool(true)), "TRUE");
        assert_eq!(cell_to_string(&Data::Empty), "");
        assert_eq!(cell_to_string(&Data::String("文本".into())), "文本");
    }

    // covers: BR-PRV-DATA-002
    // 解析失败回统一错误（不静默空表）；不存在的文件即失败路径。
    #[test]
    fn parse_failure_is_explicit() {
        let err = xlsx_to_sheets(Path::new("/不存在/x.xlsx")).unwrap_err();
        assert_eq!(err.code, "PRV_TABLE_PARSE_FAIL");
    }
}
