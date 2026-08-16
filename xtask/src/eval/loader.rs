#![forbid(unsafe_code)]

//! JSONL loader and integrity validator for evaluation fixture datasets.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::models::{CorpusDoc, QrelItem, QueryItem};

/// Loads a JSONL corpus file into a vector of [`CorpusDoc`] items.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or if a line fails JSON parsing.
pub fn load_corpus(path: &Path) -> Result<Vec<CorpusDoc>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut docs = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let text = line?;
        if text.trim().is_empty() {
            continue;
        }
        let doc: CorpusDoc = serde_json::from_str(&text).map_err(|err| {
            format!(
                "failed to parse corpus doc at line {} in {}: {}",
                index + 1,
                path.display(),
                err
            )
        })?;
        docs.push(doc);
    }

    Ok(docs)
}

/// Loads a JSONL queries file into a vector of [`QueryItem`] items.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or if a line fails JSON parsing.
pub fn load_queries(path: &Path) -> Result<Vec<QueryItem>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut queries = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let text = line?;
        if text.trim().is_empty() {
            continue;
        }
        let query: QueryItem = serde_json::from_str(&text).map_err(|err| {
            format!(
                "failed to parse query at line {} in {}: {}",
                index + 1,
                path.display(),
                err
            )
        })?;
        queries.push(query);
    }

    Ok(queries)
}

/// Loads a JSONL qrels ground truth file into a vector of [`QrelItem`] items.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or if a line fails JSON parsing.
pub fn load_qrels(path: &Path) -> Result<Vec<QrelItem>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut qrels = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let text = line?;
        if text.trim().is_empty() {
            continue;
        }
        let qrel: QrelItem = serde_json::from_str(&text).map_err(|err| {
            format!(
                "failed to parse qrel at line {} in {}: {}",
                index + 1,
                path.display(),
                err
            )
        })?;
        qrels.push(qrel);
    }

    Ok(qrels)
}

/// Validates cross-referential integrity and line boundaries across fixture datasets.
///
/// # Errors
///
/// Returns an error describing any orphaned identifiers, invalid scores, or out-of-bound line ranges.
pub fn validate_fixtures(
    corpus: &[CorpusDoc],
    queries: &[QueryItem],
    qrels: &[QrelItem],
) -> Result<(), String> {
    let corpus_doc_ids: HashSet<&str> = corpus.iter().map(|d| d.doc_id.as_str()).collect();
    let query_ids: HashSet<&str> = queries.iter().map(|q| q.query_id.as_str()).collect();

    // Map doc_id to line count in content
    let doc_lines_map: std::collections::HashMap<&str, usize> = corpus
        .iter()
        .map(|d| (d.doc_id.as_str(), d.content.lines().count()))
        .collect();

    let mut errors = Vec::new();

    for (idx, qrel) in qrels.iter().enumerate() {
        let entry_label = format!("qrel[{}] ({}:{})", idx + 1, qrel.query_id, qrel.doc_id);

        if !query_ids.contains(qrel.query_id.as_str()) {
            errors.push(format!(
                "{entry_label}: unknown query_id '{}'",
                qrel.query_id
            ));
        }

        if !corpus_doc_ids.contains(qrel.doc_id.as_str()) {
            errors.push(format!("{entry_label}: unknown doc_id '{}'", qrel.doc_id));
        } else if let Some(&max_lines) = doc_lines_map.get(qrel.doc_id.as_str())
            && (qrel.line_start == 0
                || qrel.line_end < qrel.line_start
                || qrel.line_end > max_lines)
        {
            errors.push(format!(
                "{entry_label}: invalid line range {}-{} (doc has {} lines)",
                qrel.line_start, qrel.line_end, max_lines
            ));
        }

        if qrel.score > 2 {
            errors.push(format!(
                "{entry_label}: invalid score {} (must be 0, 1, or 2)",
                qrel.score
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Fixture integrity validation failed with {} error(s):\n  - {}",
            errors.len(),
            errors.join("\n  - ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_fixtures_succeeds_on_valid_data() {
        let corpus = vec![CorpusDoc {
            doc_id: "doc1".to_string(),
            path: "doc1.md".to_string(),
            title: "Doc 1".to_string(),
            frontmatter: std::collections::HashMap::new(),
            content: "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n".to_string(),
        }];

        let queries = vec![QueryItem {
            query_id: "q1".to_string(),
            query_text: "test".to_string(),
            modality: "lexical".to_string(),
            description: "desc".to_string(),
        }];

        let qrels = vec![QrelItem {
            query_id: "q1".to_string(),
            doc_id: "doc1".to_string(),
            line_start: 1,
            line_end: 4,
            score: 2,
        }];

        assert!(validate_fixtures(&corpus, &queries, &qrels).is_ok());
    }

    #[test]
    fn validate_fixtures_detects_invalid_queries_and_bounds() {
        let corpus = vec![CorpusDoc {
            doc_id: "doc1".to_string(),
            path: "doc1.md".to_string(),
            title: "Doc 1".to_string(),
            frontmatter: std::collections::HashMap::new(),
            content: "Line 1\nLine 2\n".to_string(),
        }];

        let queries = vec![QueryItem {
            query_id: "q1".to_string(),
            query_text: "test".to_string(),
            modality: "lexical".to_string(),
            description: "desc".to_string(),
        }];

        // Bad query ID, bad doc ID, bad line bounds, bad score
        let qrels = vec![
            QrelItem {
                query_id: "q_unknown".to_string(),
                doc_id: "doc1".to_string(),
                line_start: 1,
                line_end: 2,
                score: 1,
            },
            QrelItem {
                query_id: "q1".to_string(),
                doc_id: "doc_unknown".to_string(),
                line_start: 1,
                line_end: 2,
                score: 1,
            },
            QrelItem {
                query_id: "q1".to_string(),
                doc_id: "doc1".to_string(),
                line_start: 1,
                line_end: 10, // exceeds 2 lines
                score: 1,
            },
            QrelItem {
                query_id: "q1".to_string(),
                doc_id: "doc1".to_string(),
                line_start: 1,
                line_end: 2,
                score: 5, // invalid score
            },
        ];

        let result = validate_fixtures(&corpus, &queries, &qrels);
        assert!(result.is_err());
        let err = result.err().unwrap_or_default();
        assert!(err.contains("unknown query_id"));
        assert!(err.contains("unknown doc_id"));
        assert!(err.contains("invalid line range"));
        assert!(err.contains("invalid score"));
    }
}
