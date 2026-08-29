//! AgentTurn 显式目标集合与引用上下文装配。

use super::prompt_runtime::MENTION_CAP;
use super::reference_runtime::{format_reference_payload_context, BlockRef};
use crate::ag;
use crate::types::ReferencePayload;

pub(super) struct TurnReferenceRuntime {
    pub(super) mention_block: Option<String>,
}

/**
 * @GOV
 * codes:    BR-AG-DATA-001-DATA-AG-AG-TURN-030
 * type:     DATA
 * chain:    AG-TURN
 * rules:    BR-AG-DATA-001
 * term_ref: TERM-REF-003
 * boundary: in=ExplicitDocumentBlockAndReferencePayloads | out=ValidatedReferenceContext | delegate=format_reference_payload_context | does_not=grantDocumentEditAuthorization
 */
pub(super) fn build_turn_reference_runtime(
    mentions: &[String],
    block_refs: &[BlockRef],
    references: &[ReferencePayload],
    read_ctx: &ag::blocks::AgentReadContext,
) -> TurnReferenceRuntime {
    let mut parts = Vec::new();
    for rel_path in mentions {
        match read_ctx.snapshot(rel_path) {
            Ok(snapshot) => {
                let capped = ag::context::cap_text(
                    &snapshot.content,
                    MENTION_CAP,
                    "[引用文档较长，已截取前部；需要完整内容可用 read_doc]",
                );
                parts.push(format!("引用文档「{rel_path}」：\n{capped}"));
            }
            Err(error) => parts.push(format!(
                "引用文档「{rel_path}」读取失败（{}）：{}",
                error.code, error.message
            )),
        }
    }
    for block_ref in block_refs {
        if block_ref.text.trim().is_empty() {
            continue;
        }
        let payload = ReferencePayload::Selection {
            rel_path: block_ref.rel_path.clone(),
            block_id: block_ref.block_id.clone(),
            start: block_ref.start,
            end: block_ref.end,
            text: block_ref.text.clone(),
            label: None,
        };
        parts.push(format_reference_payload_context(
            &payload,
            Some(read_ctx),
            MENTION_CAP,
        ));
    }
    for reference in references {
        parts.push(format_reference_payload_context(
            reference,
            Some(read_ctx),
            MENTION_CAP,
        ));
    }
    let mention_block = (!parts.is_empty()).then(|| {
        format!(
            "[用户显式引用] 以下是用户本轮明确引用的内容（高优先，请据此回答相关问题；外部链接仅代表链接本身，不代表已抓取网页内容）：\n{}",
            parts.join("\n\n")
        )
    });

    TurnReferenceRuntime { mention_block }
}
