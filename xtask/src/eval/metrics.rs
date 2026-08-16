#![forbid(unsafe_code)]

//! Information Retrieval (IR) metrics calculation engine.

use std::cmp::Ordering;
use std::collections::HashMap;

use super::models::{EvaluationReport, ModalityMetrics, QrelItem, QueryItem, RankedItem};

/// Calculates whether two line ranges [s1, e1] and [s2, e2] overlap.
#[must_use]
pub fn line_ranges_overlap(s1: usize, e1: usize, s2: usize, e2: usize) -> bool {
    s1 <= e2 && s2 <= e1
}

/// Matches a retrieved item against query ground truth judgments (qrels) and returns the relevance grade (0, 1, or 2).
#[must_use]
pub fn match_relevance(item: &RankedItem, judgments: &[&QrelItem]) -> u8 {
    let mut max_score = 0;
    for judgment in judgments {
        if judgment.doc_id == item.doc_id {
            // If the judgment spans the full doc (e.g. line 1 to max) or ranges overlap
            let overlaps = (judgment.line_start == 1 && judgment.line_end >= 1)
                || (item.line_start == 1 && item.line_end >= 1)
                || line_ranges_overlap(
                    item.line_start,
                    item.line_end,
                    judgment.line_start,
                    judgment.line_end,
                );

            if overlaps && judgment.score > max_score {
                max_score = judgment.score;
            }
        }
    }
    max_score
}

/// Computes Discounted Cumulative Gain (DCG@K) for a sequence of relevance grades.
#[allow(
    clippy::cast_precision_loss,
    reason = "rank index is bounded by K (small integer)"
)]
#[must_use]
pub fn compute_dcg(relevance_grades: &[u8], k: usize) -> f64 {
    let mut dcg = 0.0;
    let limit = relevance_grades.len().min(k);

    for (index, &grade) in relevance_grades.iter().take(limit).enumerate() {
        let gain = f64::from((1_u32 << u32::from(grade)) - 1);
        let rank = (index + 1) as f64;
        let discount = (rank + 1.0).log2();
        if discount > 0.0 {
            dcg += gain / discount;
        }
    }

    dcg
}

/// Computes Ideal Discounted Cumulative Gain (IDCG@K) given all ground truth judgments for a query.
#[must_use]
pub fn compute_idcg(judgments: &[&QrelItem], k: usize) -> f64 {
    let mut grades: Vec<u8> = judgments
        .iter()
        .map(|judgment| judgment.score)
        .filter(|&score| score > 0)
        .collect();

    grades.sort_unstable_by(|a, b| b.cmp(a));
    compute_dcg(&grades, k)
}

/// Computes Normalized Discounted Cumulative Gain (NDCG@K) for a sequence of retrieved relevance grades.
#[must_use]
pub fn compute_ndcg(relevance_grades: &[u8], judgments: &[&QrelItem], k: usize) -> f64 {
    let idcg = compute_idcg(judgments, k);
    if idcg <= 0.0 {
        return 1.0;
    }
    let dcg = compute_dcg(relevance_grades, k);
    (dcg / idcg).min(1.0)
}

/// Computes Reciprocal Rank (RR@K) for a sequence of relevance grades (1/rank of first item with grade >= 1).
#[allow(
    clippy::cast_precision_loss,
    reason = "rank index is bounded by K (small integer)"
)]
#[must_use]
pub fn compute_reciprocal_rank(relevance_grades: &[u8], k: usize) -> f64 {
    let limit = relevance_grades.len().min(k);
    for (index, &grade) in relevance_grades.iter().take(limit).enumerate() {
        if grade >= 1 {
            return 1.0 / ((index + 1) as f64);
        }
    }
    0.0
}

/// Computes Hit Rate (Recall@K: 1.0 if at least one item has grade >= 1 within top-K, else 0.0).
#[must_use]
pub fn compute_hit_rate(relevance_grades: &[u8], k: usize) -> f64 {
    let limit = relevance_grades.len().min(k);
    for &grade in relevance_grades.iter().take(limit) {
        if grade >= 1 {
            return 1.0;
        }
    }
    0.0
}

/// Evaluates a collection of queries and rankings against ground truth judgments.
#[allow(
    clippy::cast_precision_loss,
    reason = "query counts are small numbers in test suites"
)]
#[must_use]
pub fn evaluate_rankings<S: std::hash::BuildHasher>(
    queries: &[QueryItem],
    qrels: &[QrelItem],
    rankings: &HashMap<String, Vec<RankedItem>, S>,
    k: usize,
) -> EvaluationReport {
    // Group qrels by query_id
    let mut qrels_by_query: HashMap<&str, Vec<&QrelItem>> = HashMap::new();
    for qrel in qrels {
        qrels_by_query.entry(&qrel.query_id).or_default().push(qrel);
    }

    // Group queries by modality
    let mut modality_queries: HashMap<&str, Vec<&QueryItem>> = HashMap::new();
    for query in queries {
        modality_queries
            .entry(&query.modality)
            .or_default()
            .push(query);
    }

    let mut query_scores: Vec<(String, f64, f64, f64)> = Vec::new();

    for query in queries {
        let empty_judgments = Vec::new();
        let judgments = qrels_by_query
            .get(query.query_id.as_str())
            .unwrap_or(&empty_judgments);
        let empty_ranking = Vec::new();
        let retrieved = rankings
            .get(query.query_id.as_str())
            .unwrap_or(&empty_ranking);

        let grades: Vec<u8> = retrieved
            .iter()
            .map(|item| match_relevance(item, judgments))
            .collect();

        let hit = compute_hit_rate(&grades, k);
        let rr = compute_reciprocal_rank(&grades, k);
        let ndcg = compute_ndcg(&grades, judgments, k);

        query_scores.push((query.modality.clone(), hit, rr, ndcg));
    }

    let total = query_scores.len();
    let overall_metrics = if total == 0 {
        ModalityMetrics {
            modality: "all".to_string(),
            query_count: 0,
            recall_at_k: 0.0,
            mrr_at_k: 0.0,
            ndcg_at_k: 0.0,
        }
    } else {
        let sum_hit: f64 = query_scores.iter().map(|(_, hit, _, _)| *hit).sum();
        let sum_rr: f64 = query_scores.iter().map(|(_, _, rr, _)| *rr).sum();
        let sum_ndcg: f64 = query_scores.iter().map(|(_, _, _, ndcg)| *ndcg).sum();
        let count = total as f64;

        ModalityMetrics {
            modality: "all".to_string(),
            query_count: total,
            recall_at_k: sum_hit / count,
            mrr_at_k: sum_rr / count,
            ndcg_at_k: sum_ndcg / count,
        }
    };

    let mut modality_list: Vec<ModalityMetrics> = Vec::new();
    let mut sorted_modalities: Vec<&&str> = modality_queries.keys().collect();
    sorted_modalities.sort();

    for modality in sorted_modalities {
        let entries: Vec<&(String, f64, f64, f64)> = query_scores
            .iter()
            .filter(|(m, _, _, _)| m == *modality)
            .collect();
        let count = entries.len();
        if count > 0 {
            let sum_hit: f64 = entries.iter().map(|(_, hit, _, _)| *hit).sum();
            let sum_rr: f64 = entries.iter().map(|(_, _, rr, _)| *rr).sum();
            let sum_ndcg: f64 = entries.iter().map(|(_, _, _, ndcg)| *ndcg).sum();
            let n = count as f64;

            modality_list.push(ModalityMetrics {
                modality: (*modality).to_string(),
                query_count: count,
                recall_at_k: sum_hit / n,
                mrr_at_k: sum_rr / n,
                ndcg_at_k: sum_ndcg / n,
            });
        }
    }

    modality_list.sort_by(|a, b| match (a.modality.as_str(), b.modality.as_str()) {
        ("lexical", _) => Ordering::Less,
        (_, "lexical") => Ordering::Greater,
        ("semantic", _) => Ordering::Less,
        (_, "semantic") => Ordering::Greater,
        ("contextual", _) => Ordering::Less,
        (_, "contextual") => Ordering::Greater,
        _ => a.modality.cmp(&b.modality),
    });

    EvaluationReport {
        cutoff_k: k,
        total_queries: total,
        overall: overall_metrics,
        modalities: modality_list,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_ranges_overlap_detects_intersections() {
        assert!(line_ranges_overlap(10, 20, 15, 25));
        assert!(line_ranges_overlap(10, 20, 20, 30));
        assert!(line_ranges_overlap(15, 25, 10, 20));
        assert!(!line_ranges_overlap(10, 20, 21, 30));
        assert!(!line_ranges_overlap(21, 30, 10, 20));
    }

    #[test]
    fn compute_hit_rate_returns_one_when_relevant_in_top_k() {
        assert!((compute_hit_rate(&[0, 2, 0], 3) - 1.0).abs() < f64::EPSILON);
        assert!((compute_hit_rate(&[0, 1, 0], 2) - 1.0).abs() < f64::EPSILON);
        assert!((compute_hit_rate(&[0, 0, 1], 2) - 0.0).abs() < f64::EPSILON);
        assert!((compute_hit_rate(&[0, 0, 0], 5) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_reciprocal_rank_calculates_correct_reciprocals() {
        assert!((compute_reciprocal_rank(&[2, 0, 0], 3) - 1.0).abs() < f64::EPSILON);
        assert!((compute_reciprocal_rank(&[0, 1, 0], 3) - 0.5).abs() < f64::EPSILON);
        assert!((compute_reciprocal_rank(&[0, 0, 2], 3) - 0.333_333_333_333_333_3).abs() < 1e-6);
        assert!((compute_reciprocal_rank(&[0, 0, 0], 3) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_ndcg_handles_perfect_and_partial_rankings() {
        let qrels = [
            QrelItem {
                query_id: "q1".to_string(),
                doc_id: "d1".to_string(),
                line_start: 1,
                line_end: 10,
                score: 2,
            },
            QrelItem {
                query_id: "q1".to_string(),
                doc_id: "d2".to_string(),
                line_start: 1,
                line_end: 10,
                score: 1,
            },
        ];
        let qrel_refs: Vec<&QrelItem> = qrels.iter().collect();

        // Perfect ranking [2, 1]
        let perfect_ndcg = compute_ndcg(&[2, 1], &qrel_refs, 2);
        assert!((perfect_ndcg - 1.0).abs() < 1e-6);

        // Suboptimal ranking [1, 2]
        let suboptimal_ndcg = compute_ndcg(&[1, 2], &qrel_refs, 2);
        assert!(suboptimal_ndcg < 1.0);
        assert!(suboptimal_ndcg > 0.0);

        // Empty ground truth
        let empty_refs: Vec<&QrelItem> = Vec::new();
        assert!((compute_ndcg(&[0, 0], &empty_refs, 2) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn evaluate_rankings_aggregates_modalities_correctly() {
        let queries = vec![
            QueryItem {
                query_id: "q1".to_string(),
                query_text: "lexical test".to_string(),
                modality: "lexical".to_string(),
                description: "desc".to_string(),
            },
            QueryItem {
                query_id: "q2".to_string(),
                query_text: "semantic test".to_string(),
                modality: "semantic".to_string(),
                description: "desc".to_string(),
            },
        ];

        let qrels = vec![
            QrelItem {
                query_id: "q1".to_string(),
                doc_id: "d1".to_string(),
                line_start: 1,
                line_end: 10,
                score: 2,
            },
            QrelItem {
                query_id: "q2".to_string(),
                doc_id: "d2".to_string(),
                line_start: 1,
                line_end: 10,
                score: 2,
            },
        ];

        let mut rankings = HashMap::new();
        rankings.insert(
            "q1".to_string(),
            vec![RankedItem {
                doc_id: "d1".to_string(),
                line_start: 1,
                line_end: 10,
                score: 0.95,
            }],
        );
        rankings.insert(
            "q2".to_string(),
            vec![RankedItem {
                doc_id: "d2".to_string(),
                line_start: 1,
                line_end: 10,
                score: 0.88,
            }],
        );

        let report = evaluate_rankings(&queries, &qrels, &rankings, 5);

        assert_eq!(report.total_queries, 2);
        assert_eq!(report.cutoff_k, 5);
        assert!((report.overall.recall_at_k - 1.0).abs() < f64::EPSILON);
        assert!((report.overall.mrr_at_k - 1.0).abs() < f64::EPSILON);
        assert!((report.overall.ndcg_at_k - 1.0).abs() < f64::EPSILON);
        assert_eq!(report.modalities.len(), 2);
    }
}
