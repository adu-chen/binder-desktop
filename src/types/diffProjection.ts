/**
 * Rust `types::DiffProjection` 的前端传输投影。
 * 字段语义以 A-SYS-I-P-01 的 DiffProjection 契约为准，业务模块不得各自复制定义。
 */
export type AgentEditPatchKind =
  | "replace_block"
  | "block_ops"
  | "replace_document"
  | "style_ops"
  | "table_ops"
  | "image_ops";

export interface DiffProjectionDto {
  diff_id: string;
  rel_path: string;
  patch_kind: AgentEditPatchKind;
  block_id?: string | null;
  block_index?: number | null;
  before_text: string;
  after_text: string;
  location_label: string;
  draft_mode: "current_editor" | "open_session" | "background";
  snapshot_source:
    | "active_editor"
    | "active_editor_session"
    | "open_editor_session"
    | "disk_snapshot";
  base_session_version?: number | null;
  base_content_hash?: string | null;
  style_ops?: unknown;
  block_ops?: unknown;
  table_ops?: unknown;
  image_ops?: unknown;
  batch?: { id: string; index: number; total: number };
  preview_checkpoint_required?: boolean;
  cross_doc?: boolean | null;
}
