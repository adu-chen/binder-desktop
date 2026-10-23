//! AgentTurn 显式目标集合与引用上下文装配。

use super::prompt_runtime::{MENTION_CAP, REFERENCE_TOTAL_CAP};
use super::reference_runtime::{build_reference_context_projection, BlockRef};
use crate::ag;
use crate::types::ReferencePayload;

pub(super) struct TurnReferenceRuntime {
    pub(super) mention_block: Option<String>,
    pub(super) prompt_envelopes: Vec<ag::context::PromptDataEnvelopeV1>,
    pub(super) deduplicated_count: usize,
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
    let mut prompt_envelopes = Vec::new();
    let mut reference_chars = 0usize;
    let mut seen = std::collections::HashSet::new();
    let mut deduplicated_count = 0;
    let mut append_payload = |payload: &ReferencePayload| {
        let identified = match payload {
            ReferencePayload::Doc { .. }
            | ReferencePayload::Selection { .. }
            | ReferencePayload::Session { .. }
            | ReferencePayload::Url { .. } => true,
            ReferencePayload::Memory { entry_id, .. }
            | ReferencePayload::Knowledge { entry_id, .. } => {
                entry_id.as_ref().is_some_and(|id| !id.is_empty())
            }
            ReferencePayload::Template { template_id, .. } => {
                template_id.as_ref().is_some_and(|id| !id.is_empty())
            }
            ReferencePayload::File { path, .. } => {
                path.as_ref().is_some_and(|path| !path.is_empty())
            }
            ReferencePayload::LongText { .. } => false,
        };
        if identified
            && !seen.insert(serde_json::to_string(payload).expect("reference serialization"))
        {
            deduplicated_count += 1;
            None
        } else {
            Some(build_reference_context_projection(
                payload,
                Some(read_ctx),
                MENTION_CAP,
            ))
        }
    };
    let mut append_projection =
        |mut projection: super::reference_runtime::ReferenceContextProjection| {
            let remaining = REFERENCE_TOTAL_CAP.saturating_sub(reference_chars);
            let projection_chars = projection.text.chars().count();
            if projection_chars > remaining {
                projection.text = ag::context::cap_text(
                    &projection.text,
                    remaining,
                    "[该显式引用因本轮引用总预算已裁剪；可减少引用或用 read_doc 按需读取]",
                );
                let aggregate_reason =
                    format!("reference_total_budget_exhausted:{projection_chars}>{remaining}");
                projection.truncation_reason = Some(match projection.truncation_reason {
                    Some(reason) => format!("{reason};{aggregate_reason}"),
                    None => aggregate_reason,
                });
            }
            reference_chars = reference_chars.saturating_add(projection.text.chars().count());
            parts.push(projection.text.clone());
            prompt_envelopes.push(ag::context::prompt_data_envelope_with_budget(
                projection.source_type,
                &projection.source_id,
                &projection.provenance,
                &projection.freshness,
                projection.trust_level,
                &projection.text,
                projection.original_char_count,
                projection.truncation_reason.as_deref(),
            ));
        };
    for rel_path in mentions {
        let payload = ReferencePayload::Doc {
            rel_path: rel_path.clone(),
            label: None,
        };
        if let Some(projection) = append_payload(&payload) {
            append_projection(projection);
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
        if let Some(projection) = append_payload(&payload) {
            append_projection(projection);
        }
    }
    for reference in references {
        if let Some(projection) = append_payload(reference) {
            append_projection(projection);
        }
    }
    let mention_block = (!parts.is_empty()).then(|| {
        format!(
            "[用户显式引用] 以下是用户本轮明确引用的内容（高优先，请据此回答相关问题；外部链接仅代表链接本身，不代表已抓取网页内容）：\n{}",
            parts.join("\n\n")
        )
    });

    TurnReferenceRuntime {
        mention_block,
        prompt_envelopes,
        deduplicated_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: BR-AG-DATA-001, BR-AG-DATA-005
    #[test]
    fn reference_dedup_is_local_exact_and_identity_sensitive() {
        let ctx = ag::blocks::AgentReadContext::new(std::path::Path::new("."), vec![]).unwrap();
        let doc = ReferencePayload::Doc {
            rel_path: "p05-missing.txt".into(),
            label: None,
        };
        let anon = ReferencePayload::LongText {
            text: "same".into(),
            char_count: 4,
            label: None,
        };
        let selection = ReferencePayload::Selection {
            rel_path: "p05-missing.txt".into(),
            block_id: "block-1".into(),
            start: Some(0),
            end: Some(4),
            text: "same".into(),
            label: None,
        };
        let refs = vec![
            doc.clone(),
            ReferencePayload::Doc {
                rel_path: "different.txt".into(),
                label: None,
            },
            anon.clone(),
            anon,
            selection.clone(),
        ];
        let first = build_turn_reference_runtime(&["p05-missing.txt".into()], &[], &refs, &ctx);
        assert_eq!(first.deduplicated_count, 1);
        assert_eq!(first.prompt_envelopes.len(), 5);
        let again = build_turn_reference_runtime(&[], &[], &[doc], &ctx);
        assert_eq!(again.deduplicated_count, 0);
        let mut changed = selection.clone();
        if let ReferencePayload::Selection { text, .. } = &mut changed {
            *text = "new!".into();
        }
        let selections =
            build_turn_reference_runtime(&[], &[], &[selection.clone(), selection, changed], &ctx);
        assert_eq!(selections.deduplicated_count, 1);
        assert_eq!(selections.prompt_envelopes.len(), 2);
    }

    // covers: BR-AG-DATA-001, BR-AG-DATA-002, BR-AG-DATA-005
    #[test]
    fn explicit_references_are_individually_traced_and_globally_budgeted() {
        let read_ctx = ag::blocks::AgentReadContext::new(std::path::Path::new("."), vec![])
            .expect("empty read context");
        let references = vec![
            ReferencePayload::LongText {
                text: "甲".repeat(6_000),
                char_count: 6_000,
                label: None,
            },
            ReferencePayload::LongText {
                text: "乙".repeat(6_000),
                char_count: 6_000,
                label: None,
            },
        ];
        let runtime = build_turn_reference_runtime(&[], &[], &references, &read_ctx);

        assert_eq!(runtime.prompt_envelopes.len(), 2);
        assert!(runtime.prompt_envelopes[0].truncation_reason.is_some());
        assert!(runtime.prompt_envelopes[1]
            .truncation_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("reference_total_budget_exhausted")));
        assert!(runtime
            .mention_block
            .as_deref()
            .is_some_and(|text| text.contains("引用总预算已裁剪")));
    }
}
