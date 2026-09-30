use crate::related::RelatedFile;
use kv_application::{
    EmbedOutcome, EmbedReport, HybridResultSet, IndexState, IndexStatus, SearchResultSet,
    SkipReason,
};

pub(crate) fn render_human(set: &SearchResultSet, related: Option<&[Vec<RelatedFile>]>) -> String {
    let mut lines = Vec::new();
    for (index, result) in set.results().iter().enumerate() {
        let position = result.position();
        let header = if position.line_start() == 0 {
            format!(
                "{}. {} ({}, score {:.3})",
                index + 1,
                result.path().display(),
                result.kind().as_str(),
                result.score()
            )
        } else {
            format!(
                "{}. {}:{}-{} ({}, score {:.3})",
                index + 1,
                result.path().display(),
                position.line_start(),
                position.line_end(),
                result.kind().as_str(),
                result.score()
            )
        };
        lines.push(header);
        lines.push(result.text().to_owned());
        render_related_lines(&mut lines, related, index);
    }
    if !set.results().is_empty() {
        lines.push(format!("{} match(es)", set.total()));
    }
    lines.join("\n")
}

pub(crate) fn render_related_lines(
    lines: &mut Vec<String>,
    related: Option<&[Vec<RelatedFile>]>,
    index: usize,
) {
    let Some(related) = related else {
        return;
    };
    for file in related.get(index).into_iter().flatten() {
        lines.push(format!(
            "related: {} ({})",
            file.path().display(),
            file.relation().as_str()
        ));
    }
}

pub(crate) fn render_json(
    set: &SearchResultSet,
    query: &str,
    scope: &str,
    limit: u16,
    related: Option<&[Vec<RelatedFile>]>,
) -> String {
    let results: Vec<serde_json::Value> = set
        .results()
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let position = result.position();
            let mut value = serde_json::json!({
                "collection": result.collection().display_name(),
                "path": result.path().to_string_lossy(),
                "kind": result.kind().as_str(),
                "text": result.text(),
                "score": result.score(),
                "position": {
                    "byte_offset": position.byte_offset(),
                    "byte_length": position.byte_length(),
                    "line_start": position.line_start(),
                    "line_end": position.line_end(),
                },
            });
            append_related_field(&mut value, related, index);
            value
        })
        .collect();

    serde_json::json!({
        "query": query,
        "scope": scope,
        "limit": limit,
        "total": set.total(),
        "results": results,
    })
    .to_string()
}

fn append_related_field(
    value: &mut serde_json::Value,
    related: Option<&[Vec<RelatedFile>]>,
    index: usize,
) {
    let Some(related) = related else {
        return;
    };
    let entries: Vec<serde_json::Value> = related
        .get(index)
        .into_iter()
        .flatten()
        .map(|file| {
            serde_json::json!({
                "path": file.path().to_string_lossy(),
                "relation": file.relation().as_str(),
            })
        })
        .collect();
    value["related"] = serde_json::json!(entries);
}

pub(crate) fn render_hybrid_human(
    set: &HybridResultSet,
    related: Option<&[Vec<RelatedFile>]>,
) -> String {
    let mut lines = Vec::new();
    for (index, result) in set.results().iter().enumerate() {
        let position = result.position();
        let header = if position.line_start() == 0 {
            format!(
                "{}. {} ({}, score {:.3})",
                index + 1,
                result.path().display(),
                result.kind().as_str(),
                result.ordering_score()
            )
        } else {
            format!(
                "{}. {}:{}-{} ({}, score {:.3})",
                index + 1,
                result.path().display(),
                position.line_start(),
                position.line_end(),
                result.kind().as_str(),
                result.ordering_score()
            )
        };
        lines.push(header);
        lines.push(result.text().to_owned());
        render_related_lines(&mut lines, related, index);
    }
    if !set.results().is_empty() {
        lines.push(format!("{} result(s)", set.results().len()));
    }
    if set.rerank_warning() {
        lines.push("re-ranking skipped: re-ranker model is not cached; pass --no-rerank to suppress this warning".to_owned());
    }
    lines.join("\n")
}

pub(crate) fn render_hybrid_json(
    set: &HybridResultSet,
    query: &str,
    scope: &str,
    limit: u16,
    related: Option<&[Vec<RelatedFile>]>,
) -> String {
    let results: Vec<serde_json::Value> = set
        .results()
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let position = result.position();
            let mut value = serde_json::json!({
                "collection": result.collection().display_name(),
                "path": result.path().to_string_lossy(),
                "kind": result.kind().as_str(),
                "text": result.text(),
                "reranker_score": result.rerank_score(),
                "fused_score": result.fused_score(),
                "bm25_score": result.lexical_score(),
                "cosine_similarity": result.semantic_score(),
                "ordering_score": result.ordering_score(),
                "position": {
                    "byte_offset": position.byte_offset(),
                    "byte_length": position.byte_length(),
                    "line_start": position.line_start(),
                    "line_end": position.line_end(),
                },
            });
            append_related_field(&mut value, related, index);
            value
        })
        .collect();

    serde_json::json!({
        "query": query,
        "scope": scope,
        "limit": limit,
        "reranked": set.reranked(),
        "rerank_warning": set.rerank_warning(),
        "total": results.len(),
        "results": results,
    })
    .to_string()
}

pub(crate) fn render_embed_report(report: &EmbedReport) -> String {
    let mut lines = report
        .outcomes()
        .iter()
        .map(render_embed_outcome)
        .collect::<Vec<_>>();
    if report.any_failed() {
        lines.push("embedding completed with failures".to_owned());
    }
    lines.join("\n")
}

pub(crate) fn render_embed_outcome(outcome: &EmbedOutcome) -> String {
    let name = outcome.collection().display_name();
    match outcome {
        EmbedOutcome::Embedded { passage_count, .. } => {
            format!("collection \"{name}\": embedded {passage_count} passage(s)")
        }
        EmbedOutcome::AlreadyCurrent { .. } => {
            format!("collection \"{name}\": already current")
        }
        EmbedOutcome::Skipped {
            reason: SkipReason::NoFiles,
            ..
        } => format!("collection \"{name}\": skipped (no files)"),
        EmbedOutcome::Skipped {
            reason: SkipReason::LexicalNotBuilt,
            ..
        } => format!("collection \"{name}\": skipped (lexical index not built)"),
        EmbedOutcome::Failed { message, .. } => {
            format!("collection \"{name}\": failed ({message})")
        }
    }
}

pub(crate) fn render_index_status(status: &IndexStatus) -> String {
    let semantic = status
        .semantic()
        .map(|line| {
            format!(
                ", embedded with {} ({} dimensions)",
                line.model().as_str(),
                line.dimension()
            )
        })
        .unwrap_or_default();
    match (status.state(), status.built_at()) {
        (IndexState::Built, Some(timestamp)) => format!(
            "collection \"{}\": lexical index built, {} file(s), {} passage(s), built at {}{semantic}",
            status.collection().display_name(),
            status.file_count(),
            status.passage_count(),
            timestamp.as_unix_seconds()
        ),
        _ => format!(
            "collection \"{}\": lexical index not built",
            status.collection().display_name()
        ),
    }
}
