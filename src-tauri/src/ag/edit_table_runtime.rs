//! TableOperation 基线、目标、动态能力、坐标和原子载荷校验。

use super::edit_support::capability_error_to_ipc;
use crate::ag;
use crate::ipc::IpcError;

/**
 * @GOV
 * codes:    BR-ED-DATA-007-GUARD-AG-AG-TOOL-018
 * type:     GUARD
 * chain:    AG-TOOL, DE-DIFF, ED-EDIT
 * rules:    BR-ED-DATA-007
 * term_ref: TERM-ED-007
 * boundary: in=UntrustedTableOperationAndActiveTableSnapshot | out=ValidatedTableOperationPayloadOrPreDiffRejection | consumer=BR-ED-DATA-007
 */
pub(super) fn table_ops_payload(
    params: &serde_json::Value,
    snapshot: &ag::blocks::AgentEditableSnapshot,
    tables: &[serde_json::Value],
) -> Result<serde_json::Value, IpcError> {
    let base = params
        .get("base")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TOOL_ARG",
                "table_ops 缺 base（list_tables 的 sessionVersion/contentHash）",
                false,
                false,
            )
        })?;
    let base_version = base
        .get("sessionVersion")
        .or_else(|| base.get("session_version"))
        .and_then(|v| v.as_u64());
    let base_hash = base
        .get("contentHash")
        .or_else(|| base.get("content_hash"))
        .and_then(|v| v.as_str());
    if base_version != snapshot.session_version || base_hash != snapshot.content_hash.as_deref() {
        return Err(IpcError::new(
            "AG_TABLE_STALE_SNAPSHOT",
            "表格快照已过期；请重新调用 list_tables 后再提出修改建议。",
            true,
            false,
        ));
    }
    let target = params
        .get("target")
        .and_then(|v| v.as_object())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TOOL_ARG",
                "table_ops 缺 target（tableId/fingerprint）",
                false,
                false,
            )
        })?;
    let table_id = target
        .get("tableId")
        .or_else(|| target.get("table_id"))
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "table_ops target 缺 tableId", false, false))?;
    let fingerprint = target
        .get("fingerprint")
        .and_then(|v| v.as_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TOOL_ARG",
                "table_ops target 缺 fingerprint",
                false,
                false,
            )
        })?;
    let known = tables.iter().find(|table| {
        table.get("tableId").and_then(|v| v.as_str()) == Some(table_id)
            && table.get("fingerprint").and_then(|v| v.as_str()) == Some(fingerprint)
    });
    let Some(table) = known else {
        return Err(IpcError::new(
            "AG_TABLE_ANCHOR_LOST",
            "未找到匹配 tableId/fingerprint 的当前表格；请重新调用 list_tables。",
            true,
            false,
        ));
    };
    let operations = params
        .get("operations")
        .and_then(|v| v.as_array())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| IpcError::new("AG_TOOL_ARG", "table_ops 缺非空 operations", false, false))?;
    ag::bdocx_capabilities::validate_table_operation_shapes(operations)
        .map_err(capability_error_to_ipc)?;
    let rows = table.get("rows").and_then(|v| v.as_u64()).ok_or_else(|| {
        IpcError::new("AG_TABLE_SNAPSHOT_INVALID", "表格快照缺 rows", true, false)
    })?;
    let columns = table
        .get("columns")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TABLE_SNAPSHOT_INVALID",
                "表格快照缺 columns",
                true,
                false,
            )
        })?;
    let capabilities = table
        .get("capabilities")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            IpcError::new(
                "AG_TABLE_SNAPSHOT_INVALID",
                "表格快照缺 capabilities",
                true,
                false,
            )
        })?;
    let has_capability = |operation: &str| {
        capabilities
            .iter()
            .any(|value| value.as_str() == Some(operation))
    };
    let coordinate = |operation: &serde_json::Value, field: &str| -> Result<(u64, u64), IpcError> {
        let object = operation
            .get(field)
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                IpcError::new(
                    "AG_TABLE_OPERATION_INVALID",
                    &format!("{field} 必须是 {{row,column}} 坐标"),
                    false,
                    false,
                )
            })?;
        let row = object.get("row").and_then(|v| v.as_u64()).ok_or_else(|| {
            IpcError::new(
                "AG_TABLE_OPERATION_INVALID",
                &format!("{field}.row 必须是非负整数"),
                false,
                false,
            )
        })?;
        let column = object
            .get("column")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                IpcError::new(
                    "AG_TABLE_OPERATION_INVALID",
                    &format!("{field}.column 必须是非负整数"),
                    false,
                    false,
                )
            })?;
        if row >= rows || column >= columns {
            return Err(IpcError::new(
                "AG_TABLE_OPERATION_INVALID",
                &format!("{field} 超出当前表格范围"),
                false,
                false,
            ));
        }
        Ok((row, column))
    };
    for operation in operations {
        let name = operation.get("op").and_then(|v| v.as_str()).unwrap_or("");
        if operation.get("html").is_some() || operation.get("content").is_some() {
            return Err(IpcError::new(
                "AG_TABLE_OPERATION_INVALID",
                "table_ops 只允许受限 TableOperation，禁止 HTML/content。",
                false,
                false,
            ));
        }
        let require_capability = |capability: &str| -> Result<(), IpcError> {
            if has_capability(capability) {
                Ok(())
            } else {
                Err(IpcError::new(
                    "AG_TABLE_OPERATION_UNAVAILABLE",
                    &format!("当前表格不可执行 {capability}"),
                    false,
                    false,
                ))
            }
        };
        match name {
            "insert_row" => {
                require_capability(name)?;
                if operation
                    .get("at")
                    .and_then(|v| v.as_u64())
                    .filter(|at| *at <= rows)
                    .is_none()
                {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "insert_row.at 必须是 0..=rows 的整数",
                        false,
                        false,
                    ));
                }
            }
            "delete_row" => {
                require_capability(name)?;
                if operation
                    .get("row")
                    .and_then(|v| v.as_u64())
                    .filter(|row| *row < rows)
                    .is_none()
                {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "delete_row.row 必须是表内行号",
                        false,
                        false,
                    ));
                }
            }
            "insert_column" => {
                require_capability(name)?;
                if operation
                    .get("at")
                    .and_then(|v| v.as_u64())
                    .filter(|at| *at <= columns)
                    .is_none()
                {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "insert_column.at 必须是 0..=columns 的整数",
                        false,
                        false,
                    ));
                }
            }
            "delete_column" => {
                require_capability(name)?;
                if operation
                    .get("column")
                    .and_then(|v| v.as_u64())
                    .filter(|column| *column < columns)
                    .is_none()
                {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "delete_column.column 必须是表内列号",
                        false,
                        false,
                    ));
                }
            }
            "set_header_row" => {
                require_capability(name)?;
                if operation.get("enabled").and_then(|v| v.as_bool()).is_none() {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "set_header_row.enabled 必须是布尔值",
                        false,
                        false,
                    ));
                }
            }
            "merge_cells" => {
                require_capability(name)?;
                let from = coordinate(operation, "from")?;
                let to = coordinate(operation, "to")?;
                if from == to {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "merge_cells 必须覆盖至少两个单元格",
                        false,
                        false,
                    ));
                }
            }
            "split_cell" => {
                require_capability(name)?;
                coordinate(operation, "at")?;
            }
            "set_cell_text" => {
                require_capability(name)?;
                let (row, column) = coordinate(operation, "at")?;
                if operation.get("text").and_then(|v| v.as_str()).is_none() {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "set_cell_text.text 必须是字符串",
                        false,
                        false,
                    ));
                }
                let editable = table
                    .get("cells")
                    .and_then(|v| v.as_array())
                    .map(|cells| {
                        cells.iter().any(|cell| {
                            let start_row =
                                cell.get("row").and_then(|v| v.as_u64()).unwrap_or(u64::MAX);
                            let start_column = cell
                                .get("column")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(u64::MAX);
                            let rowspan = cell.get("rowspan").and_then(|v| v.as_u64()).unwrap_or(1);
                            let colspan = cell.get("colspan").and_then(|v| v.as_u64()).unwrap_or(1);
                            row >= start_row
                                && row < start_row.saturating_add(rowspan)
                                && column >= start_column
                                && column < start_column.saturating_add(colspan)
                                && cell.get("contentKind").and_then(|v| v.as_str())
                                    == Some("plain_text")
                        })
                    })
                    .unwrap_or(false);
                if !editable {
                    return Err(IpcError::new(
                        "AG_TABLE_RICH_CELL_UNSUPPORTED",
                        "代理只能改写单一纯文本段落单元格；富内容单元格请保留人工编辑。",
                        false,
                        false,
                    ));
                }
            }
            "set_cell_background" => {
                require_capability(name)?;
                let (row, column) = coordinate(operation, "at")?;
                let anchored = table
                    .get("cells")
                    .and_then(|v| v.as_array())
                    .map(|cells| {
                        cells.iter().any(|cell| {
                            cell.get("row").and_then(|v| v.as_u64()) == Some(row)
                                && cell.get("column").and_then(|v| v.as_u64()) == Some(column)
                        })
                    })
                    .unwrap_or(false);
                if !anchored {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "set_cell_background.at 必须是单元格左上锚；合并单元格内部坐标不允许使用",
                        false,
                        false,
                    ));
                }
                if let Some(color) = operation.get("color").filter(|value| !value.is_null()) {
                    let valid = color
                        .as_str()
                        .map(|value| {
                            value.len() == 7
                                && value.starts_with('#')
                                && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
                        })
                        .unwrap_or(false);
                    if !valid {
                        return Err(IpcError::new(
                            "AG_TABLE_OPERATION_INVALID",
                            "set_cell_background.color 只能是 #RRGGBB 或 null",
                            false,
                            false,
                        ));
                    }
                } else if operation.get("color").is_none() {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "set_cell_background 必须显式提供 color（#RRGGBB 或 null）",
                        false,
                        false,
                    ));
                }
            }
            "move_table" => {
                require_capability(name)?;
                let direction = operation.get("direction").and_then(|v| v.as_str());
                if !matches!(direction, Some("up" | "down")) {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_INVALID",
                        "move_table.direction 只能是 up 或 down",
                        false,
                        false,
                    ));
                }
                let available = table
                    .get("moveDirections")
                    .and_then(|v| v.as_array())
                    .map(|directions| directions.iter().any(|value| value.as_str() == direction))
                    .unwrap_or(false);
                if !available {
                    return Err(IpcError::new(
                        "AG_TABLE_OPERATION_UNAVAILABLE",
                        "表格在当前文档流位置不能向该方向移动",
                        false,
                        false,
                    ));
                }
            }
            _ => {
                return Err(IpcError::new(
                    "AG_TABLE_OPERATION_INVALID",
                    "table_ops 包含不支持的表格操作",
                    false,
                    false,
                ))
            }
        }
    }
    Ok(
        serde_json::json!({ "tableId": table_id, "fingerprint": fingerprint, "operations": operations }),
    )
}
