#![forbid(unsafe_code)]

//! Formatting and threshold verification for search quality evaluation reports.

use std::fmt::Write as _;

use super::models::EvaluationReport;

/// Quality metric threshold criteria.
#[derive(Debug, Clone, PartialEq)]
pub struct QualityThresholds {
    /// Minimum acceptable Recall@K.
    pub min_recall: Option<f64>,
    /// Minimum acceptable MRR@K.
    pub min_mrr: Option<f64>,
    /// Minimum acceptable NDCG@K.
    pub min_ndcg: Option<f64>,
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            min_recall: Some(0.85),
            min_mrr: Some(0.70),
            min_ndcg: Some(0.75),
        }
    }
}

/// Formats the evaluation report into a human-readable ASCII summary table.
#[must_use]
pub fn format_report(report: &EvaluationReport, thresholds: Option<&QualityThresholds>) -> String {
    let mut out = String::new();

    let _ = writeln!(
        out,
        "\n================================================================================"
    );
    let _ = writeln!(
        out,
        "                    mdsearch Search Quality Evaluation Report"
    );
    let _ = writeln!(
        out,
        "================================================================================"
    );
    let _ = writeln!(
        out,
        "Cutoff (K): {:<4} | Evaluated Queries: {:<4}",
        report.cutoff_k, report.total_queries
    );
    let _ = writeln!(
        out,
        "--------------------------------------------------------------------------------"
    );
    let _ = writeln!(
        out,
        "{:<16} | {:<7} | {:<12} | {:<10} | {:<10}",
        "Modality", "Queries", "Recall@K", "MRR@K", "NDCG@K"
    );
    let _ = writeln!(
        out,
        "-----------------+---------+--------------+------------+------------"
    );

    for m in &report.modalities {
        let _ = writeln!(
            out,
            "{:<16} | {:<7} | {:<12.4} | {:<10.4} | {:<10.4}",
            m.modality, m.query_count, m.recall_at_k, m.mrr_at_k, m.ndcg_at_k
        );
    }

    let _ = writeln!(
        out,
        "-----------------+---------+--------------+------------+------------"
    );
    let _ = writeln!(
        out,
        "{:<16} | {:<7} | {:<12.4} | {:<10.4} | {:<10.4}",
        "OVERALL (ALL)",
        report.overall.query_count,
        report.overall.recall_at_k,
        report.overall.mrr_at_k,
        report.overall.ndcg_at_k
    );
    let _ = writeln!(
        out,
        "================================================================================"
    );

    if let Some(t) = thresholds {
        let _ = writeln!(out, "\nQuality Gate Targets (ADR-004):");
        if let Some(r) = t.min_recall {
            let status = if report.overall.recall_at_k >= r {
                "PASS"
            } else {
                "FAIL"
            };
            let _ = writeln!(
                out,
                "  - Recall@{}: Target >= {:.2} | Actual: {:.4} [{}]",
                report.cutoff_k, r, report.overall.recall_at_k, status
            );
        }
        if let Some(m) = t.min_mrr {
            let status = if report.overall.mrr_at_k >= m {
                "PASS"
            } else {
                "FAIL"
            };
            let _ = writeln!(
                out,
                "  - MRR@{}:    Target >= {:.2} | Actual: {:.4} [{}]",
                report.cutoff_k, m, report.overall.mrr_at_k, status
            );
        }
        if let Some(n) = t.min_ndcg {
            let status = if report.overall.ndcg_at_k >= n {
                "PASS"
            } else {
                "FAIL"
            };
            let _ = writeln!(
                out,
                "  - NDCG@{}:   Target >= {:.2} | Actual: {:.4} [{}]",
                report.cutoff_k, n, report.overall.ndcg_at_k, status
            );
        }
        let _ = writeln!(out);
    }

    out
}

/// Checks whether an evaluation report meets the required quality thresholds.
///
/// # Errors
///
/// Returns an error string detailing which metric thresholds were breached.
pub fn verify_thresholds(
    report: &EvaluationReport,
    thresholds: &QualityThresholds,
) -> Result<(), String> {
    let mut failures = Vec::new();

    if let Some(min_recall) = thresholds.min_recall
        && report.overall.recall_at_k < min_recall
    {
        failures.push(format!(
            "Recall@{} ({:.4}) below minimum threshold ({:.2})",
            report.cutoff_k, report.overall.recall_at_k, min_recall
        ));
    }

    if let Some(min_mrr) = thresholds.min_mrr
        && report.overall.mrr_at_k < min_mrr
    {
        failures.push(format!(
            "MRR@{} ({:.4}) below minimum threshold ({:.2})",
            report.cutoff_k, report.overall.mrr_at_k, min_mrr
        ));
    }

    if let Some(min_ndcg) = thresholds.min_ndcg
        && report.overall.ndcg_at_k < min_ndcg
    {
        failures.push(format!(
            "NDCG@{} ({:.4}) below minimum threshold ({:.2})",
            report.cutoff_k, report.overall.ndcg_at_k, min_ndcg
        ));
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Evaluation quality gate failed:\n  - {}",
            failures.join("\n  - ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::models::ModalityMetrics;
    use super::*;

    #[test]
    fn format_report_produces_readable_table() {
        let report = EvaluationReport {
            cutoff_k: 5,
            total_queries: 10,
            overall: ModalityMetrics {
                modality: "all".to_string(),
                query_count: 10,
                recall_at_k: 0.90,
                mrr_at_k: 0.80,
                ndcg_at_k: 0.85,
            },
            modalities: vec![ModalityMetrics {
                modality: "lexical".to_string(),
                query_count: 10,
                recall_at_k: 0.90,
                mrr_at_k: 0.80,
                ndcg_at_k: 0.85,
            }],
        };

        let formatted = format_report(&report, Some(&QualityThresholds::default()));
        assert!(formatted.contains("mdsearch Search Quality Evaluation Report"));
        assert!(formatted.contains("OVERALL (ALL)"));
        assert!(formatted.contains("PASS"));
    }

    #[test]
    fn verify_thresholds_succeeds_when_above_targets() {
        let report = EvaluationReport {
            cutoff_k: 5,
            total_queries: 1,
            overall: ModalityMetrics {
                modality: "all".to_string(),
                query_count: 1,
                recall_at_k: 0.95,
                mrr_at_k: 0.90,
                ndcg_at_k: 0.90,
            },
            modalities: Vec::new(),
        };

        let result = verify_thresholds(&report, &QualityThresholds::default());
        assert!(result.is_ok());
    }

    #[test]
    fn verify_thresholds_reports_failures_when_below_targets() {
        let report = EvaluationReport {
            cutoff_k: 5,
            total_queries: 1,
            overall: ModalityMetrics {
                modality: "all".to_string(),
                query_count: 1,
                recall_at_k: 0.70, // below 0.85
                mrr_at_k: 0.60,    // below 0.70
                ndcg_at_k: 0.65,   // below 0.75
            },
            modalities: Vec::new(),
        };

        let result = verify_thresholds(&report, &QualityThresholds::default());
        assert!(result.is_err());
        let err = result.err().unwrap_or_default();
        assert!(err.contains("Recall@5"));
        assert!(err.contains("MRR@5"));
        assert!(err.contains("NDCG@5"));
    }
}
